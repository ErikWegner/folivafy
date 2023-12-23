use axum::{extract::State, http::StatusCode, Json};
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use sea_orm::{TransactionError, TransactionTrait};
use std::sync::Arc;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{
    auth,
    db::{get_collection_by_name, save_documents_events_mails, DbGrantUpdate},
    dto::{self, Event},
    hooks::{DocumentResult, HookCreatedEventContext, HookCreatingEventContext, RequestContext},
    select_document_for_update, ApiContext, ApiErrors,
};
use crate::models::CreateEventBody;

#[debug_handler]
pub(crate) async fn api_create_event(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<auth::User>,
    Json(payload): Json<CreateEventBody>,
) -> Result<(StatusCode, String), ApiErrors> {
    let post_payload = payload.clone();

    // Validate the payload
    payload.validate().map_err(ApiErrors::from)?;
    let unchecked_collection_name = payload.collection;
    let unchecked_document_id = payload.document;

    let trigger_cron_ctx = ctx.clone();
    let trigger_cron_post_ctx = ctx.clone();
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
    let hook = ctx.hooks.get_event_hook(&collection.name, payload.category);

    if hook.is_none() {
        debug!("No hook was executed");
        return Err(ApiErrors::BadRequest("Event not accepted".to_string()));
    }
    let hook = hook.unwrap();
    let post_hook = hook.clone();

    let data_service1 = ctx.data_service.clone();
    let data_service2 = ctx.data_service.clone();

    let request_context1 = Arc::new(RequestContext::new(
        &collection.name,
        collection.id,
        dto::UserWithRoles::read_from(&user),
    ));
    let request_context2 = request_context1.clone();

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

                let cdctx = HookCreatingEventContext::new(
                    Event::new(document.id, payload.category, payload.e.clone()),
                    after_document,
                    before_document,
                    data_service1,
                    request_context1,
                );

                let result = hook.on_creating(&cdctx).await?;
                let events = result.events;
                let mails = result.mails;
                if events.is_empty() {
                    debug!("No events were permitted");
                    return Err(ApiErrors::PermissionDenied);
                }
                let grants = match result.grants {
                    crate::api::hooks::GrantSettingsOnEvents::NoChange => DbGrantUpdate::Keep,
                    crate::api::hooks::GrantSettingsOnEvents::Replace(new_grants) => {
                        DbGrantUpdate::Replace(new_grants)
                    }
                };

                let dtouser = dto::User::read_from(&user);
                save_documents_events_mails(txn, &dtouser, result.documents, events, grants, mails)
                    .await
                    .map_err(|e| {
                        error!("Error while creating event: {:?}", e);
                        ApiErrors::InternalServerError
                    })?;

                trigger_cron_ctx
                    .trigger_cron_with_condition(result.trigger_cron)
                    .await;
                Ok((StatusCode::CREATED, "Done".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
        .map(|res| {
            // Start thread for background task
            tokio::spawn(async move {
                let cdctx = HookCreatedEventContext::new(
                    Event::new(
                        unchecked_document_id,
                        post_payload.category,
                        post_payload.e.clone(),
                    ),
                    data_service2,
                    request_context2,
                );

                let post_result = post_hook.on_created(&cdctx).await;
                if let Ok(r) = post_result {
                    match r.document {
                        DocumentResult::Store(_) => todo!("Document update not implemented!"),
                        DocumentResult::NoUpdate => {}
                        DocumentResult::Err(_) => todo!("Document update not implemented!"),
                    }
                    if !r.events.is_empty() {
                        error!("Not implemented");
                    }
                    trigger_cron_post_ctx
                        .trigger_cron_with_condition(r.trigger_cron)
                        .await;
                }
            });
            res
        })
}
