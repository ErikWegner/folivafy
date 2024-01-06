use async_trait::async_trait;
use uuid::Uuid;

use crate::api::dto::Grant;
use crate::api::grants::GrantCollection;
use crate::api::{data_service::DataService, dto, ApiErrors};

pub type HookResult = Result<Vec<dto::Grant>, ApiErrors>;

pub struct HookUserGrantContext {
    user: dto::UserWithRoles,
    data_service: std::sync::Arc<dyn DataService>,
    default_grants: Vec<Grant>,
}

impl HookUserGrantContext {
    pub fn new(
        user: dto::UserWithRoles,
        default_grants: Vec<Grant>,
        data_service: std::sync::Arc<dyn DataService>,
    ) -> Self {
        Self {
            user,
            data_service,
            default_grants,
        }
    }

    pub fn user(&self) -> &dto::UserWithRoles {
        &self.user
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }

    pub fn default_grants(&self) -> &Vec<Grant> {
        &self.default_grants
    }
}

pub struct HookDocumentGrantContext {
    collection: GrantCollection,
    document: dto::CollectionDocument,
    author_id: Uuid,
    data_service: std::sync::Arc<dyn DataService>,
}

impl HookDocumentGrantContext {
    pub fn new(
        collection: GrantCollection,
        document: dto::CollectionDocument,
        author_id: Uuid,
        data_service: std::sync::Arc<dyn DataService>,
    ) -> Self {
        Self {
            collection,
            document,
            author_id,
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

    pub fn author_id(&self) -> Uuid {
        self.author_id
    }
}

#[async_trait]
pub trait GrantHook {
    async fn user_grants(&self, context: &HookUserGrantContext) -> HookResult;
    async fn document_grants(&self, context: &HookDocumentGrantContext) -> HookResult;
}
