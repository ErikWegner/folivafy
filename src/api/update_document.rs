use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItem;
use sea_orm::{prelude::Uuid, EntityTrait, TransactionError, TransactionTrait};
use serde_json::json;
use tokio::sync::oneshot;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{
    auth::User,
    db::save_document_events_mails,
    dto,
    hooks::{
        HookContext, HookContextData, HookSuccessResult, ItemActionStage, ItemActionType,
        RequestContext,
    },
    select_document_for_update,
};
use entity::collection_document::Entity as Documents;

use super::{db::get_collection_by_name, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_update_document(
    State(ctx): State<ApiContext>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
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

    let new_hook_processor = ctx.hooksn.updates().cloned();

    let hook_processor = ctx.hooks.get_registered_hook(
        collection_name.as_ref(),
        ItemActionType::Update,
        ItemActionStage::Before,
    );

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

                if let Some(new_hook_processor) = new_hook_processor {
                    let hr = new_hook_processor.on_updating((&document).into()).await?;
                }
                let before_document: dto::CollectionDocument = (&document).into();
                let mut after_document: dto::CollectionDocument = (payload).into();
                let mut events: Vec<dto::Event> = vec![];
                let mut mails: Vec<dto::MailMessage> = vec![];
                if let Some(sender) = hook_processor {
                    let (tx, rx) = oneshot::channel::<Result<HookSuccessResult, ApiErrors>>();
                    let cdctx = HookContext::new(
                        HookContextData::DocumentUpdating {
                            before_document,
                            after_document,
                        },
                        RequestContext::new(
                            &collection.name,
                            user.subuuid(),
                            user.preferred_username(),
                        ),
                        tx,
                        ctx.data_service,
                    );

                    sender
                        .send(cdctx)
                        .await
                        .map_err(|_| ApiErrors::InternalServerError)?;

                    let hook_result = rx.await.map_err(|_| ApiErrors::InternalServerError)??;
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

                save_document_events_mails(
                    txn,
                    &user.subuuid(),
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
