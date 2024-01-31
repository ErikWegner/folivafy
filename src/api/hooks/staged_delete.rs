use async_trait::async_trait;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Duration};
use jwt_authorizer::JwtClaims;
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::api::auth::User;
use crate::api::db::{get_unlocked_collection_by_name, FieldFilter, ListDocumentGrants};
use crate::api::list_documents::{
    generic_list_documents, parse_pfilter, DeletedDocuments, GenericListDocumentsParams,
    ListDocumentParams,
};
use crate::api::types::Pagination;
use crate::api::{
    db::{DELETED_AT_FIELD, DELETED_BY_FIELD},
    dto::UserWithRoles,
    hooks::StoreDocument,
    ApiErrors, CATEGORY_DOCUMENT_DELETE, CATEGORY_DOCUMENT_RECOVER,
};
use crate::axumext::extractors::ValidatedQueryParams;
use crate::models::CollectionItemsList;

use super::{
    CronDefaultIntervalHook, CronDocumentSelector, EventCreatingHook, EventHookResult,
    HookCreatedEventContext, HookCreatingEventContext, HookCronContext, HookResult,
    HookSuccessResult, Hooks, MultiDocumentsSuccessResult,
};

pub fn add_staged_delete_hook(
    hooks: &mut Hooks,
    collection: &str,
    stage1days: u16,
    stage2days: u16,
) {
    debug!("Adding staged_delete_hook {collection},{stage1days},{stage2days}");
    let sd = Arc::new(StagedDelete { stage1days });
    hooks.put_event_hook(collection.to_string(), CATEGORY_DOCUMENT_DELETE, sd.clone());
    hooks.put_event_hook(
        collection.to_string(),
        CATEGORY_DOCUMENT_RECOVER,
        sd.clone(),
    );
    let job_name = format!("{collection} staged_delete");
    let document_selector = CronDocumentSelector::ByDateFieldOlderThan {
        field: DELETED_AT_FIELD.to_string(),
        value: Duration::days((stage1days + stage2days) as i64),
    };
    hooks.insert_cron_default_interval_hook(&job_name, collection, document_selector, sd);
}

struct StagedDelete {
    stage1days: u16,
}

impl StagedDelete {
    async fn delete_document_event(&self, context: &HookCreatingEventContext) -> EventHookResult {
        debug!(
            "Try to set delete flag to document {} {:?})",
            context.before_document.id(),
            context.context,
        );
        if context.before_document().is_deleted() {
            info!("Document already deleted");
            return Err(ApiErrors::BadRequest(
                "Document already deleted".to_string(),
            ));
        }
        if !has_remover_role(context.context().user(), &context.context().collection_name) {
            info!(
                "User {} ({}) is not a remover of collection {}",
                context.context().user_name(),
                context.context().user_id(),
                context.context().collection_name
            );
            return Err(ApiErrors::PermissionDenied);
        }

        let mut after_document = context.after_document().clone();
        after_document.set_field(
            DELETED_AT_FIELD,
            Value::String(chrono::Local::now().to_rfc3339()),
        );
        after_document.set_field(
            DELETED_BY_FIELD,
            json!({
                "id": context.context().user_id(),
                "title": context.context().user_name(),
            }),
        );
        Ok(MultiDocumentsSuccessResult {
            documents: vec![StoreDocument::as_update(after_document)],
            events: vec![context.event().clone()],
            mails: vec![],
            trigger_cron: false,
            grants: crate::api::hooks::GrantSettingsOnEvents::NoChange,
        })
    }

    async fn recover_document_event(&self, context: &HookCreatingEventContext) -> EventHookResult {
        debug!(
            "Try to recover document {} {:?})",
            context.before_document.id(),
            context.context,
        );
        if !context.before_document().is_deleted() {
            info!("Document is not in deleted state");
            return Err(ApiErrors::BadRequest(
                "Document is not in deleted stage".to_string(),
            ));
        }

        let collection_name = context.context().collection_name.clone();
        let document_id = context.before_document.id();
        let deleted_at = DateTime::parse_from_rfc3339(
            context
                .before_document()
                .fields()
                .get(DELETED_AT_FIELD)
                .ok_or_else(|| {
                    error!("Missing deleted_at field in document {document_id}");
                    ApiErrors::InternalServerError
                })?
                .as_str()
                .ok_or_else(|| {
                    error!("Deleted_at field is not a string in document {document_id}");
                    ApiErrors::InternalServerError
                })?,
        )
        .map_err(|e| {
            error!(
                "Error parsing deleted_at field: {} for document {document_id}",
                e
            );
            ApiErrors::InternalServerError
        })?;
        let number_of_days = chrono::Local::now()
            .signed_duration_since(deleted_at)
            .num_days();
        debug!(
            "Number of days since document {document_id} was deleted: {}",
            number_of_days
        );

        // check permissions
        let user_is_allowed = if number_of_days <= self.stage1days.into() {
            has_remover_role(context.context().user(), &collection_name)
        } else {
            let role_name = format!("C_{}_ADMIN", collection_name.to_ascii_uppercase());
            context.context().user().has_role(&role_name)
        };
        if !user_is_allowed {
            info!(
                "User {} ({}) is not a remover of collection {}",
                context.context().user_name(),
                context.context().user_id(),
                context.context().collection_name
            );
            return Err(ApiErrors::PermissionDenied);
        }
        let mut after_document = context.after_document().clone();
        after_document.remove_field(DELETED_AT_FIELD);
        after_document.remove_field(DELETED_BY_FIELD);
        Ok(MultiDocumentsSuccessResult {
            documents: vec![StoreDocument::as_update(after_document)],
            events: vec![context.event().clone()],
            mails: vec![],
            trigger_cron: false,
            grants: crate::api::hooks::GrantSettingsOnEvents::NoChange,
        })
    }
}

// CreateEventHook: sets the deleted date
#[async_trait]
impl EventCreatingHook for StagedDelete {
    async fn on_creating(&self, context: &HookCreatingEventContext) -> EventHookResult {
        match context.event.category() {
            CATEGORY_DOCUMENT_DELETE => self.delete_document_event(context).await,
            CATEGORY_DOCUMENT_RECOVER => self.recover_document_event(context).await,
            _ => {
                warn!("Unknown event category {}", context.event.category());
                Err(ApiErrors::BadRequest("Event not accepted".to_string()))
            }
        }
    }

    async fn on_created(&self, _context: &HookCreatedEventContext) -> HookResult {
        Ok(HookSuccessResult::empty())
    }
}

// CronHook: Checks for items and removes them
#[async_trait]
impl CronDefaultIntervalHook for StagedDelete {
    async fn on_default_interval(&self, context: &HookCronContext) -> HookResult {
        debug!("Found deleted document {}", context.before_document.id());
        Ok(HookSuccessResult::empty())
    }
}

fn has_remover_role(user: &UserWithRoles, collection_name: &str) -> bool {
    let remove_role_name = format!("C_{}_REMOVER", collection_name.to_ascii_uppercase());
    user.has_role(&remove_role_name)
}

pub(crate) async fn get_recoverables(
    State(db): State<DatabaseConnection>,
    Path(collection_name): Path<String>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    ValidatedQueryParams(list_params): ValidatedQueryParams<ListDocumentParams>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let collection = get_unlocked_collection_by_name(&db, &collection_name)
        .await
        .ok_or_else(|| ApiErrors::NotFound(collection_name.clone()))?;

    let user_is_permitted = user.is_collection_admin(&collection_name)
        || (user.is_collection_remover(&collection_name)
            && user.is_collection_reader(&collection_name));
    if !user_is_permitted {
        warn!(
            "User {} is not permitted for get_recoverables",
            user.name_and_sub()
        );
        return Err(ApiErrors::PermissionDenied);
    }

    let grants = ListDocumentGrants::IgnoredForAdmin;
    let mut request_filters = parse_pfilter(list_params.pfilter);
    if let Some(title) = list_params.exact_title {
        request_filters.push(FieldFilter::ExactFieldMatch {
            field_name: "title".to_string(),
            value: title,
        });
    }

    generic_list_documents(
        &db,
        collection.id,
        DeletedDocuments::LimitToDeletedDocuments,
        GenericListDocumentsParams::builder()
            .sort_fields(list_params.sort_fields.clone())
            .extra_fields(list_params.extra_fields.clone())
            .filter(if request_filters.is_empty() {
                None
            } else {
                Some(request_filters.into())
            })
            .build(),
        grants,
        pagination,
    )
    .await
}
