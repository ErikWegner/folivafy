use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document::Entity as Documents;
use jwt_authorizer::JwtClaims;
use openapi::models::CreateEventBody;
use sea_orm::EntityTrait;
use tokio::sync::oneshot;
use tracing::debug;
use validator::Validate;

use crate::api::{
    db::get_collection_by_name,
    hooks::{HookContext, HookContextData, ItemActionStage, ItemActionType, RequestContext},
};

use super::{auth::User, dto::Event, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_create_event(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
    Json(payload): Json<CreateEventBody>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Validate the payload
    payload.validate().map_err(ApiErrors::from)?;
    let unchecked_collection_name = payload.collection;
    let unchecked_document_id = payload.document;

    let collection = get_collection_by_name(&ctx.db, &unchecked_collection_name).await;
    if collection.is_none() {
        debug!("Collection {} not found", unchecked_collection_name);
        return Err(ApiErrors::PermissionDenied);
    }
    let collection_name = unchecked_collection_name;

    if !user.is_collection_reader(&collection_name) {
        debug!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let collection = collection.unwrap();
    let document = Documents::find_by_id(unchecked_document_id)
        .one(&ctx.db)
        .await?
        .and_then(|doc| {
            if collection.oao && doc.owner != user.subuuid() {
                None
            } else {
                Some(doc)
            }
        });

    if document.is_none() {
        debug!("Document {} not found", unchecked_document_id);
        return Err(ApiErrors::PermissionDenied);
    }
    let document = document.unwrap();

    let sender = ctx.hooks.execute_hook(
        collection_name.as_ref(),
        ItemActionType::AppendEvent,
        ItemActionStage::Before,
    );

    if let Some(sender) = sender {
        let (tx, rx) = oneshot::channel::<Result<Event, ApiErrors>>();
        let cdctx = HookContext::new(
            HookContextData::DocumentAdding { document: payload },
            RequestContext::new(collection),
            tx,
        );

        sender
            .send(cdctx)
            .await
            .map_err(|_e| ApiErrors::InternalServerError)?;

        rx.await.map_err(|_e| ApiErrors::InternalServerError)??
    }
    return Err(ApiErrors::BadRequest("Event not accepted".to_string()));
}
