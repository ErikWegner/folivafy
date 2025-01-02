use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use sea_orm::{prelude::Uuid, TransactionError, TransactionTrait};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, warn};
use validator::Validate;

use crate::api::{
    auth,
    db::{
        get_accessible_document, get_collection_by_name, save_document_events_mails, DbGrantUpdate,
    },
    dto::{self, GrantForDocument},
    grants::default_document_grants,
    hooks::{HookUpdateContext, RequestContext},
    select_document_for_update, ApiContext, ApiErrors,
};
use crate::models::CollectionItem;

use super::grants::{hook_or_default_user_grants, GrantCollection};

/// Replace item
///
/// Replace the item data
#[debug_handler]
#[utoipa::path(
    put,
    path = "/collections/{collection_name}",
    operation_id = "updateItemById",
    params(
        ("collection_name" = String, Path, description = "Name of the collection", pattern = r"^[a-z][-a-z0-9]*$" ),
    ),
    responses(
        (status = CREATED, description = "Document updated" ),
        (status = UNAUTHORIZED, description = "User is not a collection editor" ),
        (status = NOT_FOUND, description = "Collection not found" ),
        (status = BAD_REQUEST, description = "Invalid request" ),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error"),
    ),
    request_body(content = CollectionItem, description = "Create a new document", content_type = "application/json"),
    tag = super::TAG_COLLECTION,
)]
pub(crate) async fn api_update_document(
    State(ctx): State<ApiContext>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<auth::User>,
    Json(payload): Json<CollectionItem>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Validate the payload
    payload.validate().map_err(ApiErrors::from)?;

    let document_id = payload.id.to_string();
    let document_uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiErrors::BadRequestJsonSimpleMsg("Invalid uuid".to_string()))?;

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
        return Err(ApiErrors::BadRequestJsonSimpleMsg(
            "Read only collection".into(),
        ));
    }

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

    let hook_processor = ctx.hooks.get_update_hook(&collection.name);
    let trigger_cron_ctx = ctx.clone();

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let document = select_document_for_update(document_uuid, txn)
                    .await?
                    .and_then(|doc| {
                        if collection.oao && doc.owner != user.subuuid() {
                            None
                        } else {
                            Some(doc)
                        }
                    });
                if document.is_none() {
                    debug!("Document {} not found", document_uuid);
                    return Err(ApiErrors::PermissionDenied);
                }
                let document = document.unwrap();

                let before_document: dto::CollectionDocument = (&document).into();
                let mut after_document: dto::CollectionDocument = (payload).into();
                let mut events: Vec<dto::Event> = vec![];
                let mut mails: Vec<dto::MailMessage> = vec![];
                let mut dbgrants: DbGrantUpdate = DbGrantUpdate::Keep;
                let mut trigger_cron = false;
                let request_context = Arc::new(RequestContext::new(
                    &collection.name,
                    collection.id,
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
                    trigger_cron = hook_result.trigger_cron;
                    drop(ctx);

                    match hook_result.document {
                        crate::api::hooks::DocumentResult::Store(document) => {
                            after_document = document;
                        }
                        crate::api::hooks::DocumentResult::NoUpdate => {
                            return Err(ApiErrors::BadRequestJsonSimpleMsg(
                                "Not accepted for storage".into(),
                            ))
                        }
                        crate::api::hooks::DocumentResult::Err(err) => return Err(err),
                    }
                    events.extend(hook_result.events);
                    mails.extend(hook_result.mails);
                    dbgrants = match hook_result.grants {
                        crate::api::hooks::GrantSettings::Default => DbGrantUpdate::Replace(
                            default_document_grants(collection.oao, collection.id, user.subuuid())
                                .into_iter()
                                .map(|g| GrantForDocument::new(g, document.id))
                                .collect(),
                        ),
                        crate::api::hooks::GrantSettings::Replace(grants) => {
                            DbGrantUpdate::Replace(grants)
                        }
                        crate::api::hooks::GrantSettings::NoChange => DbGrantUpdate::Keep,
                    }
                }

                events.insert(
                    0,
                    dto::Event::new(
                        document_uuid,
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
                    dbgrants,
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
                trigger_cron_ctx
                    .trigger_cron_with_condition(trigger_cron)
                    .await;
                Ok((StatusCode::CREATED, "Document updated".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
}
