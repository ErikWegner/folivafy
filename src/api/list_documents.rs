use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document::Entity as Documents;
use jwt_authorizer::JwtClaims;
use openapi::models::{CollectionItem, CollectionItemsList};
use regex::Regex;
use sea_orm::{
    prelude::Uuid, ColumnTrait, DbBackend, EntityTrait, FromQueryResult, JsonValue, PaginatorTrait,
    QueryFilter, Statement,
};
use serde::Deserialize;
use tracing::warn;

use crate::{api::auth::User, axumext::extractors::ValidatedQueryParams};

use super::{db::get_collection_by_name, types::Pagination, ApiContext, ApiErrors};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct ListDocumentParams {
    #[serde(rename = "extraFields")]
    extra_fields: Option<String>,
}

#[debug_handler]
pub(crate) async fn api_list_document(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    Query(list_params): Query<ListDocumentParams>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let extra_fields = list_params.extra_fields.unwrap_or("title".to_string());
    let re = Regex::new(r"^[a-zA-Z0-9]+(,[a-zA-Z0-9]+)*$").unwrap();
    if !re.is_match(&extra_fields) {
        return Err(ApiErrors::BadRequest(
            "Invalid extraFields value".to_string(),
        ));
    }
    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }
    let collection = collection.unwrap();

    if !user.is_collection_reader(&collection_name) {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let mut basefind = Documents::find()
        .filter(entity::collection_document::Column::CollectionId.eq(collection.id));

    if collection.oao {
        basefind = basefind.filter(entity::collection_document::Column::Owner.eq(user.subuuid()));
    }

    let total = basefind
        .clone()
        .count(&ctx.db)
        .await
        .map_err(ApiErrors::from)
        .map(|t| u32::try_from(t).unwrap_or_default())?;

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

    let items: Vec<JsonValue> = JsonValue::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!(
            "{}{}{}",
            r#"SELECT "id", "t"."new_f" as "f"
                FROM "collection_document"
                cross join lateral (
                 select jsonb_object_agg("key", "value") as "new_f"
                 from jsonb_each("f") as x("key", "value")
                 WHERE
                    "key" in ("#,
            extra_fields,
            r#")
                  ) as "t"
                WHERE "collection_id" = $1
                ORDER BY "id"
                LIMIT 50
                OFFSET 0"#
        )
        .as_str(),
        [collection.id.into()],
    ))
    .all(&ctx.db)
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
