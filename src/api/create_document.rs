use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItem;
use sea_orm::{DbErr, RuntimeErr, TransactionError, TransactionTrait};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{auth::User, db::save_document_events_mails, hooks::RequestContext, ApiErrors};

use super::{db::get_collection_by_name, dto, hooks::HookCreateContext, ApiContext};

#[debug_handler]
pub(crate) async fn api_create_document(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
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
        return Err(ApiErrors::BadRequest("Read only collection".into()));
    }

    let collection_id = collection.id;
    let hook_processor = ctx.hooksn.create();
    let mut after_document: dto::CollectionDocument = (payload.clone()).into();
    let mut events: Vec<dto::Event> = vec![];
    let mut mails: Vec<dto::MailMessage> = vec![];
    if let Some(ref hook) = hook_processor {
        let request_context = Arc::new(RequestContext::new(
            &collection.name,
            user.subuuid(),
            user.preferred_username(),
        ));

        let ctx = HookCreateContext::new((payload).into(), ctx.data_service, request_context);
        let hook_result = hook.on_creating(&ctx).await?;
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
    };

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let document_id = *after_document.id();
                events.insert(
                    0,
                    dto::Event::new(
                        document_id,
                        crate::api::CATEGORY_DOCUMENT_UPDATES,
                        json!({
                            "user": {
                                "id": user.subuuid(),
                                "name": user.preferred_username(),
                            },
                            "new": true,
                        }),
                    ),
                );
                save_document_events_mails(
                    txn,
                    &user.subuuid(),
                    Some(after_document),
                    Some(crate::api::db::InsertDocumentData {
                        collection_id,
                        owner: user.subuuid(),
                    }),
                    events,
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
                            return ApiErrors::BadRequest("Duplicate document".to_string());
                        }
                    }

                    ApiErrors::InternalServerError
                })?;
                debug!("Document {:?} saved to {collection_name}", document_id,);
                Ok((StatusCode::CREATED, "Document saved".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
}
