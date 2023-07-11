use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document::Entity as Documents;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItem;
use sea_orm::{prelude::Uuid, EntityTrait};
use tracing::warn;

use crate::api::auth::User;

use super::{db::get_collection_by_name, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_read_document(
    State(ctx): State<ApiContext>,
    Path((collection_name, document_id)): Path<(String, String)>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItem>, ApiErrors> {
    let uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiErrors::BadRequest("Invalid uuid".to_string()))?;

    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }

    if !user.is_collection_reader(&collection_name) {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let collection = collection.unwrap();
    let document = Documents::find_by_id(uuid)
        .one(&ctx.db)
        .await?
        .and_then(|doc| (doc.collection_id == collection.id).then_some(doc))
        .and_then(|doc| {
            if collection.oao && doc.owner != user.subuuid() {
                None
            } else {
                Some(doc)
            }
        });

    if document.is_none() {
        return Err(ApiErrors::NotFound(format!(
            "Document {document_id} not found"
        )));
    }
    let document = document.unwrap();

    Ok(Json(CollectionItem {
        id: document.id,
        f: document.f,
    }))
}
