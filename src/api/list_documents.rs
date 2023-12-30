use std::str::FromStr;

use axum::{
    extract::{Path, State},
    Json,
};

use entity::DELETED_AT_FIELD;
use jwt_authorizer::JwtClaims;
use lazy_static::lazy_static;
use regex::Regex;
use sea_orm::prelude::Uuid;
use sea_orm::DatabaseConnection;

use serde::Deserialize;
use tracing::warn;
use validator::Validate;

use crate::api::grants::{hook_or_default_user_grants, GrantCollection};
use crate::models::{CollectionItem, CollectionItemsList};
use crate::{
    api::{
        auth::User,
        db::{list_documents, FieldFilter},
        types::Pagination,
        ApiContext, ApiErrors,
    },
    axumext::extractors::ValidatedQueryParams,
};

use super::db::{get_unlocked_collection_by_name, DbListDocumentParams, ListDocumentGrants};

lazy_static! {
    static ref RE_EXTRA_FIELDS: Regex = Regex::new(r"^[a-zA-Z0-9]+(,[a-zA-Z0-9]+)*$").unwrap();
    static ref RE_SORT_FIELDS: Regex = Regex::new(
        r"^[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+\-fb](,[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+\-fb])*$"
    )
    .unwrap();
}

pub(crate) enum DeletedDocuments {
    LimitToDeletedDocuments,
    Exclude,
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
    let collection = get_unlocked_collection_by_name(&ctx.db, &collection_name)
        .await
        .ok_or_else(|| ApiErrors::NotFound(collection_name.clone()))?;

    let user_is_permitted = user.is_collection_admin(&collection_name)
        || user.can_access_all_documents(&collection_name)
        || user.is_collection_reader(&collection_name);
    if !user_is_permitted {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let dto_collection: GrantCollection = (&collection).into();
    let user_grants =
        hook_or_default_user_grants(&ctx.hooks, &dto_collection, &user, ctx.data_service.clone())
            .await?;

    let grants = ListDocumentGrants::Restricted(user_grants);

    generic_list_documents(
        &ctx.db,
        collection.id,
        DeletedDocuments::Exclude,
        list_params,
        grants,
        pagination,
    )
    .await
}

pub(crate) fn parse_pfilter(s: Option<String>) -> Vec<FieldFilter> {
    // Split s by ampersand
    s.map(|s| s.split('&').filter_map(FieldFilter::from_str).collect())
        .unwrap_or_default()
}

pub(crate) async fn generic_list_documents(
    db: &DatabaseConnection,
    collection_id: Uuid,
    deleted_documents: DeletedDocuments,
    list_params: ListDocumentParams,
    grants: ListDocumentGrants,
    pagination: Pagination,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let extra_fields = list_params.extra_fields.unwrap_or("title".to_string());
    let mut extra_fields: Vec<String> = extra_fields.split(',').map(|s| s.to_string()).collect();
    let title = "title".to_string();
    if !extra_fields.contains(&title) {
        extra_fields.push(title);
    }

    let filters = match deleted_documents {
        DeletedDocuments::LimitToDeletedDocuments => FieldFilter::FieldIsNotNull {
            field_name: DELETED_AT_FIELD.to_string(),
        },
        DeletedDocuments::Exclude => FieldFilter::FieldIsNull {
            field_name: DELETED_AT_FIELD.to_string(),
        },
    };

    let mut request_filters = parse_pfilter(list_params.pfilter);
    let mut filters = vec![filters];
    filters.append(&mut request_filters);

    let db_params = DbListDocumentParams::builder()
        .collection(collection_id)
        .exact_title(list_params.exact_title)
        .grants(grants)
        .extra_fields(extra_fields)
        .sort_fields(list_params.sort_fields)
        .filters(filters)
        .pagination(pagination.clone())
        .build();

    let (total, items) = list_documents(db, &db_params)
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

    #[test]
    pub fn test_starts_with() {
        // Arrange
        let s = "az=@'kl'";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 1);
        match r.get(0).unwrap() {
            FieldFilter::FieldStartsWith { field_name, value } => {
                assert_eq!(field_name, "az");
                assert_eq!(value, "kl");
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_contains() {
        // Arrange
        let s = "pt=~'imi'";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 1);
        match r.get(0).unwrap() {
            FieldFilter::FieldContains { field_name, value } => {
                assert_eq!(field_name, "pt");
                assert_eq!(value, "imi");
            }
            _ => panic!("Unexpected value"),
        }
    }
}
