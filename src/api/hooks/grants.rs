use async_trait::async_trait;

use crate::api::grants::GrantCollection;
use crate::api::{data_service::DataService, dto, ApiErrors};

pub type HookResult = Result<Vec<dto::Grant>, ApiErrors>;

pub struct HookUserGrantContext {
    user: dto::UserWithRoles,
    data_service: std::sync::Arc<dyn DataService>,
}

impl HookUserGrantContext {
    pub fn new(user: dto::UserWithRoles, data_service: std::sync::Arc<dyn DataService>) -> Self {
        Self { user, data_service }
    }

    pub fn user(&self) -> &dto::UserWithRoles {
        &self.user
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }
}

pub struct HookDocumentGrantContext {
    collection: GrantCollection,
    document: dto::CollectionDocument,
    data_service: std::sync::Arc<dyn DataService>,
}

impl HookDocumentGrantContext {
    pub fn new(
        collection: GrantCollection,
        document: dto::CollectionDocument,
        data_service: std::sync::Arc<dyn DataService>,
    ) -> Self {
        Self {
            collection,
            document,
            data_service,
        }
    }

    pub fn collection(&self) -> &GrantCollection {
        &self.collection
    }

    pub fn document(&self) -> &dto::CollectionDocument {
        &self.document
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }
}

#[async_trait]
pub trait GrantHook {
    async fn user_grants(&self, context: &HookUserGrantContext) -> HookResult;
    async fn document_grants(&self, context: &HookDocumentGrantContext) -> HookResult;
}
