use async_trait::async_trait;
use chrono::Duration;
use serde_json::{json, Value};
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use jwt_authorizer::JwtClaims;
use sea_orm::DatabaseConnection;
use tracing::{debug, info, warn};

use crate::api::{db::{DELETED_AT_FIELD, DELETED_BY_FIELD}, dto::UserWithRoles, hooks::StoreDocument, ApiErrors, CATEGORY_DOCUMENT_DELETE};
use crate::api::auth::User;
use crate::api::db::get_unlocked_collection_by_name;
use crate::api::list_documents::ListDocumentParams;
use crate::api::types::Pagination;
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
    let sd = Arc::new(StagedDelete {});
    hooks.put_event_hook(collection.to_string(), CATEGORY_DOCUMENT_DELETE, sd.clone());
    let job_name = format!("{collection} staged_delete");
    let document_selector = CronDocumentSelector::ByDateFieldOlderThan {
        field: DELETED_AT_FIELD.to_string(),
        value: Duration::days((stage1days + stage2days) as i64),
    };
    hooks.insert_cron_default_interval_hook(&job_name, collection, document_selector, sd);
}

struct StagedDelete {}

// CreateEventHook: sets the deleted date
#[async_trait]
impl EventCreatingHook for StagedDelete {
    async fn on_creating(&self, context: &HookCreatingEventContext) -> EventHookResult {
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
    let role_name = format!("C_{}_REMOVER", collection_name.to_ascii_uppercase());
    user.has_role(&role_name)
}

pub(crate) async fn get_recoverables(
    State(db): State<DatabaseConnection>,
    Path(collection_name): Path<String>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    ValidatedQueryParams(list_params): ValidatedQueryParams<ListDocumentParams>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let extra_fields = list_params.extra_fields.unwrap_or("title".to_string());
    let collection = get_unlocked_collection_by_name(&db, &collection_name).await
        .ok_or_else(|| ApiErrors::NotFound(collection_name.clone()))?;

    let user_is_permitted = user.is_collection_admin(&collection_name) || (
        user.is_collection_remover(&collection_name) && user.is_collection_reader(&collection_name)
        );
    if !user_is_permitted {
        warn!("User {} is not permitted for get_recoverables", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let mut extra_fields: Vec<String> = extra_fields.split(',').map(|s| s.to_string()).collect();
    let title = "title".to_string();
    if !extra_fields.contains(&title) {
        extra_fields.push(title);
    }

    todo!("Impl")
}
