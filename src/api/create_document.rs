use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use sea_orm::{DbErr, RuntimeErr, TransactionError, TransactionTrait};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, warn};
use uuid::Uuid;
use validator::Validate;

use crate::api::{
    auth,
    db::{get_collection_by_name, save_document_events_mails},
    dto::{self, GrantForDocument},
    hooks::{HookCreateContext, RequestContext},
    ApiContext, ApiErrors,
};
use crate::models::CollectionItem;

use super::grants::default_document_grants;

#[debug_handler]
pub(crate) async fn api_create_document(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<auth::User>,
    Path(collection_name): Path<String>,
    Json(payload): Json<CollectionItem>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Check if user is allowed to create a document within the collection
    if !user.is_collection_editor(&collection_name) {
        warn!("User {} is not a collection editor", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    // Validate the payload
    payload.validate().map_err(ApiErrors::from)?;

    let collection = get_collection_by_name(&ctx.db, &collection_name).await;

    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }
    let collection = collection.unwrap();

    // Check if collection is locked
    if collection.locked {
        warn!(
            "User {} tried to add document to locked collection {}",
            user.name_and_sub(),
            collection_name
        );
        return Err(ApiErrors::BadRequestJsonSimpleMsg(
            "Read only collection".into(),
        ));
    }

    let collection_id = collection.id;
    let hook_processor = ctx.hooks.get_create_hook(&collection.name);
    let mut after_document: dto::CollectionDocument = (payload.clone()).into();
    let document_id = *after_document.id();
    let mut events: Vec<dto::Event> = vec![];
    let mut mails: Vec<dto::MailMessage> = vec![];
    let mut grants: Vec<GrantForDocument> = vec![];
    let mut trigger_cron = false;
    let trigger_cron_ctx = ctx.clone();
    if let Some(ref hook) = hook_processor {
        let request_context = Arc::new(RequestContext::new(
            &collection.name,
            collection_id,
            dto::UserWithRoles::read_from(&user),
        ));

        let ctx = HookCreateContext::new((payload).into(), ctx.data_service, request_context);
        let hook_result = hook.on_creating(&ctx).await?;
        trigger_cron = hook_result.trigger_cron;
        match hook_result.document {
            crate::api::hooks::DocumentResult::Store(document) => {
                debug!("Received document: {:?}", document);
                after_document = document;
            }
            crate::api::hooks::DocumentResult::NoUpdate => {
                debug!("Not accepted for storage");
                return Err(ApiErrors::BadRequestJsonSimpleMsg(
                    "Not accepted for storage".into(),
                ));
            }
            crate::api::hooks::DocumentResult::Err(err) => return Err(err),
        }
        events.extend(hook_result.events);
        grants.extend(match hook_result.grants {
            crate::api::hooks::GrantSettings::Default => {
                default_document_grants(collection.oao, collection_id, user.subuuid())
                    .into_iter()
                    .map(|g| GrantForDocument::new(g, document_id))
                    .collect()
            }
            crate::api::hooks::GrantSettings::Replace(g) => g,
            crate::api::hooks::GrantSettings::NoChange => {
                error!("Hook did not provide grants");
                return Err(ApiErrors::InternalServerError);
            }
        });
        mails.extend(hook_result.mails);
    } else {
        grants.extend(
            default_document_grants(collection.oao, collection_id, user.subuuid())
                .into_iter()
                .map(|g| GrantForDocument::new(g, document_id))
                .collect::<Vec<_>>(),
        );
    };

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let dtouser = dto::User::read_from(&user);
                save_document_events_mails(
                    txn,
                    &dtouser,
                    Some(after_document),
                    Some(crate::api::db::InsertDocumentData { collection_id }),
                    events,
                    crate::api::db::DbGrantUpdate::Replace(grants),
                    mails,
                )
                .await
                .map_err(|e| {
                    error!("Create document error: {:?}", e);
                    // Check if anyhow contains a DbErr
                    let d = e.downcast_ref::<DbErr>().unwrap();
                    debug!("DB error: {:?}", d);
                    if let Some(DbErr::Query(RuntimeErr::SqlxError(sqlx::Error::Database(e)))) =
                        e.downcast_ref::<DbErr>()
                    {
                        let code = e.code().unwrap_or_default().to_string();
                        debug!("DB error code: {}", code);
                        if code == "23505" {
                            return ApiErrors::BadRequestJsonSimpleMsg(
                                "Duplicate document".to_string(),
                            );
                        }
                    }

                    ApiErrors::InternalServerError
                })?;
                debug!("Document {:?} saved to {collection_name}", document_id,);
                trigger_cron_ctx
                    .trigger_cron_with_condition(trigger_cron)
                    .await;
                Ok((StatusCode::CREATED, "Document saved".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
}

pub(crate) fn create_document_event(document_id: Uuid, user: &dto::User) -> dto::Event {
    debug!(
        "create_document_event: document_id: {:?}, user: {:?}",
        document_id, user
    );
    dto::Event::new(
        document_id,
        crate::api::CATEGORY_DOCUMENT_UPDATES,
        json!({
            "user": {
                "id": user.id(),
                "name": user.name(),
            },
            "new": true,
        }),
    )
}
