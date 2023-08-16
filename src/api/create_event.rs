use axum::{extract::State, http::StatusCode, Json};
use axum_macros::debug_handler;

use jwt_authorizer::JwtClaims;
use openapi::models::CreateEventBody;
use sea_orm::{TransactionError, TransactionTrait};
use tokio::sync::oneshot;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{
    db::{get_collection_by_name, save_document_and_events},
    dto,
    hooks::{
        DocumentResult, HookContext, HookContextData, ItemActionStage, ItemActionType,
        RequestContext,
    },
    select_document_for_update,
};

use super::{auth::User, dto::Event, hooks::HookSuccessResult, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_create_event(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
    Json(payload): Json<CreateEventBody>,
) -> Result<(StatusCode, String), ApiErrors> {
    let post_payload = payload.clone();
    let post_user = user.clone();

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
    // Check if collection is locked
    if collection.locked {
        warn!(
            "User {} tried to add events to document in locked collection {}",
            user.name_and_sub(),
            collection_name
        );
        return Err(ApiErrors::BadRequest("Read only collection".into()));
    }
    let hook_transmitter = ctx.hooks.get_registered_hook(
        collection_name.as_ref(),
        ItemActionType::AppendEvent {
            category: payload.category,
        },
        ItemActionStage::Before,
    );
    let after_hook = ctx.hooks.get_registered_hook(
        collection_name.as_ref(),
        ItemActionType::AppendEvent {
            category: payload.category,
        },
        ItemActionStage::After,
    );
    let post_collection = collection.clone();

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let document = select_document_for_update(unchecked_document_id, txn).await?;
                if document.is_none() {
                    debug!("Document {} not found", unchecked_document_id);
                    return Err(ApiErrors::PermissionDenied);
                }
                let document = document.unwrap();
                let before_document: dto::CollectionDocument = (&document).into();
                let after_document: dto::CollectionDocument = (&document).into();

                if hook_transmitter.is_none() {
                    debug!("No hook was executed");
                    return Err(ApiErrors::BadRequest("Event not accepted".to_string()));
                }
                let hook_transmitter = hook_transmitter.unwrap();

                let (tx, rx) = oneshot::channel::<Result<HookSuccessResult, ApiErrors>>();
                let cdctx = HookContext::new(
                    HookContextData::EventAdding {
                        after_document,
                        before_document,
                        collection: (&collection).into(),
                        event: Event::new(document.id, payload.category, payload.e.clone()),
                    },
                    RequestContext::new(
                        &collection.name,
                        user.subuuid().clone(),
                        user.preferred_username().clone(),
                    ),
                    tx,
                );

                hook_transmitter
                    .send(cdctx)
                    .await
                    .map_err(|_e| ApiErrors::InternalServerError)?;

                let result = rx.await.map_err(|_e| ApiErrors::InternalServerError)??;
                let events = result.events;
                if events.is_empty() {
                    debug!("No events were permitted");
                    return Err(ApiErrors::PermissionDenied);
                }

                let document = match result.document {
                    DocumentResult::Store(d) => Some(d),
                    DocumentResult::NoUpdate => None,
                    DocumentResult::Err(e) => {
                        error!("Error while updating document: {:?}", e);
                        return Err(ApiErrors::InternalServerError);
                    }
                };

                save_document_and_events(txn, &user, document, None, events)
                    .await
                    .map_err(|e| {
                        error!("Error while creating event: {:?}", e);
                        ApiErrors::InternalServerError
                    })?;

                Ok((StatusCode::CREATED, "Done".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
        .and_then(|res| {
            // Start thread
            tokio::spawn(async move {
                if let Some(hook) = after_hook {
                    let (tx, rx) = oneshot::channel::<Result<HookSuccessResult, ApiErrors>>();
                    let cdctx = HookContext::new(
                        HookContextData::EventAdded {
                            collection: (&post_collection).into(),
                            event: Event::new(
                                unchecked_document_id,
                                post_payload.category,
                                post_payload.e.clone(),
                            ),
                        },
                        RequestContext::new(
                            &collection_name,
                            post_user.subuuid().clone(),
                            post_user.preferred_username(),
                        ),
                        tx,
                    );

                    let _ = hook.send(cdctx).await;
                    let _ = rx.await.ok().map(|i| i.ok()).and_then(|r| {
                        if let Some(result) = r {
                            if result.events.len() > 0 {
                                error!("Not implemented");
                            }
                        }
                        Some(())
                    });
                }
            });
            Ok(res)
        })
}
