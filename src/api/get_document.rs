use axum::{
    extract::{Path, State},
    Json,
};
use axum_macros::debug_handler;
use entity::event::Entity as Events;
use jwt_authorizer::JwtClaims;
use sea_orm::{prelude::Uuid, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use sqlx::types::chrono::DateTime;
use tracing::warn;

use crate::api::{
    auth::User,
    db::{get_accessible_document, get_collection_by_name},
    ApiContext, ApiErrors,
};
use crate::models::{CollectionItemDetails, CollectionItemEvent};

use super::grants::{hook_or_default_user_grants, GrantCollection};

/// Get item
///
/// Get item data, i. e. read the document from the collection.
#[debug_handler]
#[utoipa::path(
    get,
    path = "/collections/{collection_name}/{document_id}",
    operation_id = "getItemById",
    params(
        (
            "collection_name" = String,
            Path,
            description = "Name of the collection",
            min_length = 1,
            max_length = 32,
            pattern = r"^[a-z][-a-z0-9]*$",
        ),
        ("document_id" = String, Path, description = "UUID of the document", format = Uuid )
    ),
    responses(
        (status = OK, description = "Document data", body = CollectionItemDetails ),
        (status = UNAUTHORIZED, description = "User is not a collection reader" ),
        (status = NOT_FOUND, description = "Document not found" ),
        (status = BAD_REQUEST, description = "Invalid request" ),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error"),
    ),
    tag = super::TAG_COLLECTION,
)]
pub(crate) async fn api_read_document(
    State(ctx): State<ApiContext>,
    Path((collection_name, document_id)): Path<(String, String)>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemDetails>, ApiErrors> {
    let document_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiErrors::BadRequestJsonSimpleMsg("Invalid uuid".to_string()))?;

    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }

    let user_is_permitted = user.is_collection_admin(&collection_name)
        || user.can_access_all_documents(&collection_name)
        || user.is_collection_reader(&collection_name);
    if !user_is_permitted {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let collection = collection.unwrap();

    let dto_collection: GrantCollection = (&collection).into();
    let user_grants =
        hook_or_default_user_grants(&ctx.hooks, &dto_collection, &user, ctx.data_service.clone())
            .await?;

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
