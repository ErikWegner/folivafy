use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document::Entity as Documents;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItemsList;
use sea_orm::{
    prelude::Uuid, ColumnTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use sea_query::Expr;
use serde_json::json;
use tracing::warn;

use crate::{api::auth::User, axumext::extractors::ValidatedQueryParams};

use super::{db::get_collection_by_name, types::Pagination, ApiContext, ApiErrors};

#[derive(FromQueryResult)]
struct DocumentIdAndTitleQueryResult {
    id: Uuid,
    title: String,
}

#[debug_handler]
pub(crate) async fn api_list_document(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
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
    let items: Vec<DocumentIdAndTitleQueryResult> = basefind
        .select_only()
        .columns([entity::collection_document::Column::Id])
        .column_as(Expr::cust("f->>'title'"), "title")
        .order_by_asc(entity::collection_document::Column::Id)
        .limit(Some(pagination.limit().into()))
        .offset(Some(pagination.offset().into()))
        .into_model::<DocumentIdAndTitleQueryResult>()
        .all(&ctx.db)
        .await
        .map_err(ApiErrors::from)?;
    Ok(Json(CollectionItemsList {
        limit: pagination.limit(),
        offset: pagination.offset(),
        total,
        items: items
            .iter()
            .map(|dbitem| openapi::models::CollectionItem {
                id: dbitem.id,
                f: json!({"title": dbitem.title}),
            })
            .collect(),
    }))
}
