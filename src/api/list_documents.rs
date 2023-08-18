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
    db::{get_collection_by_name, list_documents, CollectionDocumentVisibility},
    types::Pagination,
    ApiContext, ApiErrors,
};

lazy_static! {
    static ref RE_EXTRA_FIELDS: Regex = Regex::new(r"^[a-zA-Z0-9]+(,[a-zA-Z0-9]+)*$").unwrap();
    static ref RE_SORT_FIELDS: Regex =
        Regex::new(r"^[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+-](,[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+-])*$")
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

    let extra_fields = extra_fields
        .into_iter()
        .map(|f| format!("'{f}'"))
        .collect::<Vec<_>>()
        .join(",");

    let oao_access = if collection.oao {
        if user.can_access_all_documents(&collection_name) {
            CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments
        } else {
            CollectionDocumentVisibility::PrivateAndUserIs(user.subuuid())
        }
    } else {
        CollectionDocumentVisibility::PublicAndUserIsReader
    };
    let limit = pagination.limit();
    let offset = pagination.offset();
    let (total, items) = list_documents(
        &ctx.db,
        collection.id,
        list_params.exact_title,
        oao_access,
        extra_fields,
        list_params.sort_fields,
        pagination,
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
        limit,
        offset,
        total,
        items,
    }))
}
