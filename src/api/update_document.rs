use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItem;
use sea_orm::{
    error::DbErr, prelude::Uuid, ActiveModelTrait, RuntimeErr, Set, TransactionError,
    TransactionTrait,
};
use tokio::sync::oneshot;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{
    auth::User,
    dto,
    hooks::{
        HookContext, HookContextData, HookSuccessResult, ItemActionStage, ItemActionType,
        RequestContext,
    },
    select_document_for_update,
};

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

                let before_document: dto::CollectionDocument = (&document).into();
                let mut document: collection_document::ActiveModel = document.into();
                let mut after_document: dto::CollectionDocument = (payload).into();
                if let Some(sender) = hook_processor {
                    let (tx, rx) = oneshot::channel::<Result<HookSuccessResult, ApiErrors>>();
                    let cdctx = HookContext::new(
                        HookContextData::DocumentUpdating {
                            before_document,
                            after_document,
                        },
                        RequestContext::new(collection),
                        tx,
                    );

                    sender
                        .send(cdctx)
                        .await
                        .map_err(|_| ApiErrors::InternalServerError)?;

                    let hook_result = rx
                        .await
                        .map_err(|_| ApiErrors::InternalServerError)??
                        .document;
                    match hook_result {
                        crate::api::hooks::DocumentResult::Store(document) => {
                            after_document = document;
                        }
                        crate::api::hooks::DocumentResult::NoUpdate => {
                            return Err(ApiErrors::BadRequest("Not accepted for storage".into()))
                        }
                        crate::api::hooks::DocumentResult::Err(err) => return Err(err),
                    }
                }

                document.f = Set(after_document.fields().clone());
                let _ = document.save(txn).await.map_err(|err| match err {
                    DbErr::Exec(RuntimeErr::SqlxError(error)) => match error {
                        sqlx::error::Error::Database(e) => {
                            let code: String = e.code().unwrap_or_default().to_string();
                            // We check the error code thrown by the database (PostgreSQL in this case),
                            // `23505` means `value violates unique constraint`: we have a duplicate key in the table.
                            if code == "23505" {
                                ApiErrors::BadRequest("Duplicate document".to_string())
                            } else {
                                error!("Database runtime error: {}", e);
                                ApiErrors::BadRequest(format!(
                                    "Cannot create document (code {})",
                                    code
                                ))
                            }
                        }
                        _ => {
                            error!("Database runtime error: {}", error);
                            ApiErrors::InternalServerError
                        }
                    },
                    _ => {
                        println!("{:?}", err);
                        error!("Database error: {}", err);
                        ApiErrors::InternalServerError
                    }
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
