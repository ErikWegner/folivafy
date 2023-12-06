use async_trait::async_trait;

use crate::api::dto;

use super::HookResult;
pub struct HookUserGrantContext {
    user: dto::UserWithRoles,
}

impl HookUserGrantContext {
    pub fn new(user: dto::UserWithRoles) -> Self {
        Self { user }
    }
}

pub struct HookDocumentGrantContext {
    document: dto::CollectionDocument,
}

#[async_trait]
pub trait GrantHook {
    async fn user_grants(&self, context: &HookUserGrantContext) -> HookResult;
    async fn document_grants(&self, context: &HookDocumentGrantContext) -> HookResult;
}
