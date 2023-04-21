use axum::{extract::State, Json};
use axum_macros::debug_handler;
use entity::collection::Entity as Collection;
use openapi::models::CollectionsList;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder, QuerySelect};

use crate::axumext::extractors::ValidatedQueryParams;

use super::{types::Pagination, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_list_collections(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
) -> Result<Json<CollectionsList>, ApiErrors> {
    let total = Collection::find()
        .count(&ctx.db)
        .await
        .map_err(ApiErrors::from)
        .and_then(|t| Ok(u32::try_from(t).unwrap_or_default()))?;
    let items = Collection::find()
        .order_by_asc(entity::collection::Column::Name)
        .limit(Some(pagination.limit().into()))
        .offset(Some(pagination.offset().into()))
        .all(&ctx.db)
        .await
        .map_err(ApiErrors::from)?;
    Ok(Json(CollectionsList {
        limit: pagination.limit(),
        offset: pagination.offset(),
        total,
        items: items
            .iter()
            .map(|dbitem| openapi::models::Collection {
                name: dbitem.name.clone(),
                title: dbitem.title.clone(),
                oao: dbitem.oao,
            })
            .collect(),
    }))
}
