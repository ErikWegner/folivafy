use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItem;
use sea_orm::{prelude::Uuid, TransactionError, TransactionTrait};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{
    auth,
    db::{get_accessible_document, get_collection_by_name, save_document_events_mails},
    dto,
    hooks::{HookUpdateContext, RequestContext},
    select_document_for_update, ApiContext, ApiErrors,
};

#[debug_handler]
pub(crate) async fn api_update_document(
    State(ctx): State<ApiContext>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<auth::User>,
    Json(payload): Json<CollectionItem>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Validate the payload
    payload.validate().map_err(ApiErrors::from)?;

    let document_id = payload.id.to_string();
    let uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiErrors::BadRequest("Invalid uuid".to_string()))?;

    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }

    if !user.is_collection_editor(&collection_name) {
        warn!("User {} is not a collection editor", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let collection = collection.unwrap();
    // Check if collection is locked
    if collection.locked {
        warn!(
            "User {} tried to update document in locked collection {}",
            user.name_and_sub(),
            collection_name
        );
        return Err(ApiErrors::BadRequest("Read only collection".into()));
    }

    let document = get_accessible_document(&ctx, &user, uuid, &collection).await?;

    if document.is_none() {
        return Err(ApiErrors::NotFound(format!(
            "Document {document_id} not found"
        )));
    }

    let hook_processor = ctx.hooks.get_update_hook(&collection.name);

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let document = select_document_for_update(uuid, txn)
                    .await?
                    .and_then(|doc| {
                        if collection.oao && doc.owner != user.subuuid() {
                            None
                        } else {
                            Some(doc)
                        }
                    });
                if document.is_none() {
                    debug!("Document {} not found", uuid);
                    return Err(ApiErrors::PermissionDenied);
                }
                let document = document.unwrap();

                let before_document: dto::CollectionDocument = (&document).into();
                let mut after_document: dto::CollectionDocument = (payload).into();
                let mut events: Vec<dto::Event> = vec![];
                let mut mails: Vec<dto::MailMessage> = vec![];
                let request_context = Arc::new(RequestContext::new(
                    &collection.name,
                    dto::UserWithRoles::read_from(&user),
                ));
                if let Some(ref hook_processor) = hook_processor {
                    let ctx = HookUpdateContext::new(
                        before_document,
                        after_document,
                        ctx.data_service,
                        request_context,
                    );
                    let hook_result = hook_processor.on_updating(&ctx).await?;
                    drop(ctx);

                    match hook_result.document {
                        crate::api::hooks::DocumentResult::Store(document) => {
                            after_document = document;
                        }
                        crate::api::hooks::DocumentResult::NoUpdate => {
                            return Err(ApiErrors::BadRequest("Not accepted for storage".into()))
                        }
                        crate::api::hooks::DocumentResult::Err(err) => return Err(err),
                    }
                    events.extend(hook_result.events);
                    mails.extend(hook_result.mails);
                }

                events.insert(
                    0,
                    dto::Event::new(
                        uuid,
                        crate::api::CATEGORY_DOCUMENT_UPDATES,
                        json!({
                            "user": {
                                "id": user.subuuid(),
                                "name": user.preferred_username(),
                            },
                        }),
                    ),
                );

                let dtouser = dto::User::read_from(&user);
                save_document_events_mails(
                    txn,
                    &dtouser,
                    Some(after_document),
                    None,
                    events,
                    mails,
                )
                .await
                .map_err(|e| {
                    error!("Update document error: {:?}", e);
                    ApiErrors::InternalServerError
                })?;
                debug!(
                    "Document {:?} updated in collection {}",
                    document_id, collection_name
                );
                Ok((StatusCode::CREATED, "Document updated".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
}
