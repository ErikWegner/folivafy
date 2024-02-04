use axum::{extract::State, Json};
use axum_macros::debug_handler;
use entity::collection::Entity as Collection;
use jwt_authorizer::JwtClaims;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder, QuerySelect};
use tracing::warn;

use crate::{
    api::{auth::User, types::Pagination, ApiContext, ApiErrors},
    axumext::extractors::ValidatedQueryParams,
    models::{self, CollectionsList},
};

#[debug_handler]
pub(crate) async fn api_list_collections(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionsList>, ApiErrors> {
    if !user.is_collections_administrator() {
        warn!("User {} is not a collections admin", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }
    let total = Collection::find()
        .count(&ctx.db)
        .await
        .map_err(ApiErrors::from)
        .map(|t| u32::try_from(t).unwrap_or_default())?;
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
            .map(|dbitem| models::Collection {
                locked: dbitem.locked,
                name: dbitem.name.clone(),
                oao: dbitem.oao,
                title: dbitem.title.clone(),
            })
            .collect(),
    }))
}
