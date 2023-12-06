use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use entity::event::Entity as Events;
use jwt_authorizer::JwtClaims;
use openapi::models::{CollectionItemDetails, CollectionItemEvent};
use sea_orm::{prelude::Uuid, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use sqlx::types::chrono::DateTime;
use tracing::warn;

use crate::api::{
    auth::User,
    db::{get_accessible_document, get_collection_by_name, CollectionDocumentVisibility},
    grants::{default_user_grants, DefaultUserGrantsParameters},
    ApiContext, ApiErrors,
};

#[debug_handler]
pub(crate) async fn api_read_document(
    State(ctx): State<ApiContext>,
    Path((collection_name, document_id)): Path<(String, String)>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemDetails>, ApiErrors> {
    let document_uuid = Uuid::parse_str(&document_id)
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
    let oao_access = if collection.oao {
        if user.can_access_all_documents(&collection_name) {
            CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments
        } else {
            CollectionDocumentVisibility::PrivateAndUserIs(user.subuuid())
        }
    } else {
        CollectionDocumentVisibility::PublicAndUserIsReader
    };
    // TODO: allow override
    let user_grants = default_user_grants(
        DefaultUserGrantsParameters::builder()
            .collection_uuid(collection.id)
            .visibility(oao_access)
            .build(),
    );

    let document = get_accessible_document(
        &ctx,
        &user_grants,
        user.subuuid(),
        &collection,
        document_uuid,
    )
    .await?;

    if document.is_none() {
        return Err(ApiErrors::NotFound(format!(
            "Document {document_id} not found"
        )));
    }
    let document = document.unwrap();

    let events = Events::find()
        .filter(entity::event::Column::DocumentId.eq(Uuid::parse_str(document_id.as_ref()).ok()))
        .order_by_desc(entity::event::Column::Id)
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|event| CollectionItemEvent {
            id: u32::try_from(event.id).unwrap(),
            category: event.category_id,
            e: event.payload,
            ts: DateTime::from_naive_utc_and_offset(event.timestamp.unwrap(), chrono::Utc),
        })
        .collect();

    Ok(Json(CollectionItemDetails {
        id: document.id,
        f: document.f,
        e: events,
    }))
}
