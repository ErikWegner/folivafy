use std::str::FromStr;

use axum::{
    extract::{Path, State},
    Json,
};

use jwt_authorizer::JwtClaims;
use lazy_static::lazy_static;
use openapi::models::{CollectionItem, CollectionItemsList};
use regex::Regex;
use sea_orm::prelude::Uuid;

use serde::Deserialize;
use tracing::warn;
use validator::Validate;

use crate::{api::auth::User, axumext::extractors::ValidatedQueryParams};

use super::{
    db::{get_collection_by_name, list_documents, CollectionDocumentVisibility, FieldFilter},
    types::Pagination,
    ApiContext, ApiErrors,
};

lazy_static! {
    static ref RE_EXTRA_FIELDS: Regex = Regex::new(r"^[a-zA-Z0-9]+(,[a-zA-Z0-9]+)*$").unwrap();
    static ref RE_SORT_FIELDS: Regex = Regex::new(
        r"^[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+\-fb](,[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+\-fb])*$"
    )
    .unwrap();
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(default)]
pub(crate) struct ListDocumentParams {
    #[serde(rename = "exactTitle")]
    pub(crate) exact_title: Option<String>,

    #[validate(regex = "RE_EXTRA_FIELDS")]
    #[serde(rename = "extraFields")]
    pub(crate) extra_fields: Option<String>,

    #[validate(regex = "RE_SORT_FIELDS")]
    #[serde(rename = "sort")]
    pub(crate) sort_fields: Option<String>,

    #[serde(rename = "pfilter")]
    pub(crate) pfilter: Option<String>,
}

pub(crate) async fn api_list_document(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    ValidatedQueryParams(list_params): ValidatedQueryParams<ListDocumentParams>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let extra_fields = list_params.extra_fields.unwrap_or("title".to_string());
    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }
    let collection = collection.unwrap();

    if !user.is_collection_reader(&collection_name) {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let mut extra_fields: Vec<String> = extra_fields.split(',').map(|s| s.to_string()).collect();
    let title = "title".to_string();
    if !extra_fields.contains(&title) {
        extra_fields.push(title);
    }

    let oao_access = if collection.oao {
        if user.can_access_all_documents(&collection_name) {
            CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments
        } else {
            CollectionDocumentVisibility::PrivateAndUserIs(user.subuuid())
        }
    } else {
        CollectionDocumentVisibility::PublicAndUserIsReader
    };

    // Call hook to set additional filters
    /*
       Wenn bestimmte Collection (Abteilungsfilter)
       - Frage ab, zu welchen Abteilungen der Benutzer gehört
       - Feld "org_unit" enthält eine der Abteilung

       - oder -
       - Feld signatur1.id = id des Benutzers
       - Feld signatur2.id = id des Benutzers

    */

    let (total, items) = list_documents(
        &ctx.db,
        crate::api::db::ListDocumentParams {
            collection: collection.id,
            exact_title: list_params.exact_title,
            oao_access,
            extra_fields,
            sort_fields: list_params.sort_fields,
            filters: parse_pfilter(list_params.pfilter),
            pagination: pagination.clone(),
        },
    )
    .await
    .map_err(ApiErrors::from)?;

    let items = items
        .into_iter()
        .map(|i| CollectionItem {
            id: Uuid::from_str(i["id"].as_str().unwrap()).unwrap(),
            f: i["f"].clone(),
        })
        .collect();

    Ok(Json(CollectionItemsList {
        limit: pagination.limit(),
        offset: pagination.offset(),
        total,
        items,
    }))
}

fn parse_pfilter(s: Option<String>) -> Vec<FieldFilter> {
    // Split s by ampersand
    s.map(|s| s.split('&').filter_map(FieldFilter::from_str).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_empty_pfilter() {
        assert_eq!(parse_pfilter(Some("".to_string())).len(), 0, "Empty string");
        assert_eq!(parse_pfilter(None).len(), 0, "None value");
    }

    #[test]
    pub fn test_simple() {
        // Arrange
        let s = "f1='v12'";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 1);
        match r.get(0).unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "f1");
                assert_eq!(value, "v12");
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_multiple() {
        // Arrange
        let s = "a='k'&f1=4";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 2);
        match r.get(0).unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "a");
                assert_eq!(value, "k");
            }
            _ => panic!("Unexpected value"),
        }
        match r.get(1).unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "f1");
                assert_eq!(value, "4");
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_list() {
        // Arrange
        let s = "a='k'&f3=['p1','p4','p9']";
        let expected_values = ["p1", "p4", "p9"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 2);
        match r.get(0).unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "a");
                assert_eq!(value, "k");
            }
            _ => panic!("Unexpected value"),
        }
        match r.get(1).unwrap() {
            FieldFilter::FieldValueInMatch { field_name, values } => {
                assert_eq!(field_name, "f3");
                assert_eq!(values, &expected_values);
            }
            _ => panic!("Unexpected value"),
        }
    }
}
