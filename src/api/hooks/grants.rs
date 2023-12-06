use async_trait::async_trait;

use crate::api::{data_service::DataService, dto};

use super::HookResult;

pub struct HookUserGrantContext {
    user: dto::UserWithRoles,
    data_service: std::sync::Arc<dyn DataService>,
}

impl HookUserGrantContext {
    pub fn new(user: dto::UserWithRoles, data_service: std::sync::Arc<dyn DataService>) -> Self {
        Self { user, data_service }
    }
}

pub struct HookDocumentGrantContext {
    collection: dto::Collection,
    document: dto::CollectionDocument,
    data_service: std::sync::Arc<dyn DataService>,
}

impl HookDocumentGrantContext {
    pub fn new(
        collection: dto::Collection,
        document: dto::CollectionDocument,
        data_service: std::sync::Arc<dyn DataService>,
    ) -> Self {
        Self {
            collection,
            document,
            data_service,
        }
    }
}

#[async_trait]
pub trait GrantHook {
    async fn user_grants(&self, context: &HookUserGrantContext) -> HookResult;
    async fn document_grants(&self, context: &HookDocumentGrantContext) -> HookResult;
}
