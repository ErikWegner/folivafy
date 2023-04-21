use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use axum_macros::debug_handler;
use openapi::models::CollectionsList;

use crate::axumext::extractors::ValidatedQueryParams;

use super::{types::Pagination, ApiContext};

#[debug_handler]
pub(crate) async fn api_list_collections(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
) -> Result<Json<CollectionsList>, Response> {
    Ok(Json(CollectionsList {
        limit: 0,
        offset: 0,
        total: 0,
        items: vec![],
    }))
}
