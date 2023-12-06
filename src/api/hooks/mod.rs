pub mod grants;
pub mod staged_delete;

use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use chrono::Duration;
use openapi::models::CollectionItem;
use uuid::Uuid;

use crate::api::{data_service::DataService, dto, ApiErrors};

use super::dto::{Grant, UserWithRoles};

pub enum DocumentResult {
    /// Indicates that the document was modified and should be inserted/updated.
    Store(dto::CollectionDocument),
    /// Indicates that the document was not modified or no document can be created.
    NoUpdate,
    /// Indicates the error that occurred.
    Err(ApiErrors),
}

#[derive(Debug)]
pub enum StoreNewDocumentCollection {
    Name(String),
    Id(Uuid),
}

#[derive(Debug)]
pub enum StoreNewDocumentOwner {
    User(dto::User),
    Callee,
}

#[derive(Debug)]
pub struct StoreNewDocument {
    pub owner: StoreNewDocumentOwner,
    pub collection: StoreNewDocumentCollection,
    pub document: dto::CollectionDocument,
}

#[derive(Debug)]
pub enum StoreDocument {
    New(StoreNewDocument),
    Update { document: dto::CollectionDocument },
}

impl StoreDocument {
    pub fn as_new(n: StoreNewDocument) -> Self {
        StoreDocument::New(n)
    }

    pub fn as_update(document: dto::CollectionDocument) -> Self {
        StoreDocument::Update { document }
    }
}

pub enum GrantSettings {
    Default,
    Replace(Vec<Grant>),
}

pub struct HookSuccessResult {
    pub document: DocumentResult,
    pub grants: GrantSettings,
    pub events: Vec<dto::Event>,
    pub mails: Vec<dto::MailMessage>,
    pub trigger_cron: bool,
}

#[derive(Debug)]
pub struct MultiDocumentsSuccessResult {
    pub documents: Vec<StoreDocument>,
    pub events: Vec<dto::Event>,
    pub mails: Vec<dto::MailMessage>,
    pub trigger_cron: bool,
}

impl HookSuccessResult {
    pub fn empty() -> Self {
        Self {
            document: DocumentResult::NoUpdate,
            grants: GrantSettings::Default,
            events: vec![],
            mails: vec![],
            trigger_cron: false,
        }
    }
}

impl Debug for HookSuccessResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookSuccessResult").finish()
    }
}

pub type EventHookResult = Result<MultiDocumentsSuccessResult, ApiErrors>;
pub type HookResult = Result<HookSuccessResult, ApiErrors>;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum CronDocumentSelector {
    ByFieldEqualsValue { field: String, value: String },
    ByDateFieldOlderThan { field: String, value: Duration },
}

pub struct HookCreateContext {
    document: dto::CollectionDocument,
    data_service: Arc<dyn DataService>,
    context: Arc<RequestContext>,
}

impl HookCreateContext {
    pub fn new(
        document: dto::CollectionDocument,
        data_service: Arc<dyn DataService>,
        context: Arc<RequestContext>,
    ) -> Self {
        Self {
            document,
            data_service,
            context,
        }
    }

    pub fn document(&self) -> &dto::CollectionDocument {
        &self.document
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }

    pub fn context(&self) -> &RequestContext {
        self.context.as_ref()
    }
}

pub struct HookUpdateContext {
    before_document: dto::CollectionDocument,
    after_document: dto::CollectionDocument,
    data_service: Arc<dyn DataService>,
    context: Arc<RequestContext>,
}

impl HookUpdateContext {
    pub fn new(
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
        data_service: Arc<dyn DataService>,
        context: Arc<RequestContext>,
    ) -> Self {
        Self {
            before_document,
            after_document,
            data_service,
            context,
        }
    }

    pub fn before_document(&self) -> &dto::CollectionDocument {
        &self.before_document
    }

    pub fn after_document(&self) -> &dto::CollectionDocument {
        &self.after_document
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }

    pub fn context(&self) -> &RequestContext {
        self.context.as_ref()
    }
}

pub struct HookCreatingEventContext {
    event: dto::Event,
    before_document: dto::CollectionDocument,
    after_document: dto::CollectionDocument,
    data_service: Arc<dyn DataService>,
    context: Arc<RequestContext>,
}

impl HookCreatingEventContext {
    pub fn new(
        event: dto::Event,
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
        data_service: Arc<dyn DataService>,
        context: Arc<RequestContext>,
    ) -> Self {
        Self {
            event,
            before_document,
            after_document,
            data_service,
            context,
        }
    }

    pub fn event(&self) -> &dto::Event {
        &self.event
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }

    pub fn context(&self) -> &RequestContext {
        self.context.as_ref()
    }

    pub fn before_document(&self) -> &dto::CollectionDocument {
        &self.before_document
    }

    pub fn after_document(&self) -> &dto::CollectionDocument {
        &self.after_document
    }
}

pub struct HookCreatedEventContext {
    event: dto::Event,
    data_service: Arc<dyn DataService>,
    context: Arc<RequestContext>,
}

impl HookCreatedEventContext {
    pub fn new(
        event: dto::Event,
        data_service: Arc<dyn DataService>,
        context: Arc<RequestContext>,
    ) -> Self {
        Self {
            event,
            data_service,
            context,
        }
    }

    pub fn event(&self) -> &dto::Event {
        &self.event
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }

    pub fn context(&self) -> &RequestContext {
        self.context.as_ref()
    }
}

pub struct HookCronContext {
    before_document: dto::CollectionDocument,
    after_document: dto::CollectionDocument,
    data_service: Arc<dyn DataService>,
}

impl HookCronContext {
    pub fn new(
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
        data_service: Arc<dyn DataService>,
    ) -> Self {
        Self {
            before_document,
            after_document,
            data_service,
        }
    }

    pub fn before_document(&self) -> &dto::CollectionDocument {
        &self.before_document
    }

    pub fn after_document(&self) -> &dto::CollectionDocument {
        &self.after_document
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }
}

#[async_trait]
pub trait DocumentCreatingHook {
    async fn on_creating(&self, context: &HookCreateContext) -> HookResult;
    async fn on_created(&self, context: &HookCreateContext) -> HookResult;
}

#[async_trait]
pub trait DocumentUpdatingHook {
    async fn on_updating(&self, context: &HookUpdateContext) -> HookResult;
    async fn on_updated(&self, context: &HookUpdateContext) -> HookResult;
}

#[async_trait]
pub trait EventCreatingHook {
    async fn on_creating(&self, context: &HookCreatingEventContext) -> EventHookResult;
    async fn on_created(&self, context: &HookCreatedEventContext) -> HookResult;
}

#[async_trait]
pub trait CronDefaultIntervalHook {
    async fn on_default_interval(&self, context: &HookCronContext) -> HookResult;
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct HookCollection {
    collection_name: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct HookCollectionCategory {
    collection_name: String,
    category: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CronDefaultIntervalHookData {
    job_name: String,
    collection_name: String,
    document_selector: CronDocumentSelector,
}

impl CronDefaultIntervalHookData {
    pub fn job_name(&self) -> &str {
        self.job_name.as_ref()
    }

    pub fn collection_name(&self) -> &str {
        self.collection_name.as_ref()
    }

    pub fn document_selector(&self) -> &CronDocumentSelector {
        &self.document_selector
    }
}

#[derive(Clone)]
pub struct Hooks {
    create_hooks: Arc<RwLock<HashMap<HookCollection, Arc<dyn DocumentCreatingHook + Send + Sync>>>>,
    update_hooks: Arc<RwLock<HashMap<HookCollection, Arc<dyn DocumentUpdatingHook + Send + Sync>>>>,
    event_hooks:
        Arc<RwLock<HashMap<HookCollectionCategory, Arc<dyn EventCreatingHook + Send + Sync>>>>,
    cron_default_interval_hooks: Arc<
        RwLock<
            HashMap<CronDefaultIntervalHookData, Arc<dyn CronDefaultIntervalHook + Send + Sync>>,
        >,
    >,
    grant_hooks: Arc<RwLock<HashMap<String, Arc<dyn grants::GrantHook + Send + Sync>>>>,
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            create_hooks: Arc::new(RwLock::new(HashMap::new())),
            update_hooks: Arc::new(RwLock::new(HashMap::new())),
            event_hooks: Arc::new(RwLock::new(HashMap::new())),
            cron_default_interval_hooks: Arc::new(RwLock::new(HashMap::new())),
            grant_hooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn put_create_hook(
        &self,
        collection_name: String,
        hook: Arc<dyn DocumentCreatingHook + Send + Sync>,
    ) {
        self.create_hooks
            .write()
            .unwrap()
            .insert(HookCollection { collection_name }, hook);
    }

    pub fn get_create_hook(
        &self,
        collection_name: &str,
    ) -> Option<Arc<dyn DocumentCreatingHook + Send + Sync>> {
        let key = HookCollection {
            collection_name: collection_name.to_string(),
        };
        let map = self.create_hooks.read().unwrap();
        let value = map.get(&key);
        value.cloned()
    }

    pub fn put_update_hook(
        &self,
        collection_name: String,
        hook: Arc<dyn DocumentUpdatingHook + Send + Sync>,
    ) {
        self.update_hooks
            .write()
            .unwrap()
            .insert(HookCollection { collection_name }, hook);
    }

    pub fn get_update_hook(
        &self,
        collection_name: &str,
    ) -> Option<Arc<dyn DocumentUpdatingHook + Send + Sync>> {
        let key = HookCollection {
            collection_name: collection_name.to_string(),
        };
        let map = self.update_hooks.read().unwrap();
        let value = map.get(&key);
        value.cloned()
    }

    pub fn put_event_hook(
        &self,
        collection_name: String,
        category: i32,
        hook: Arc<dyn EventCreatingHook + Send + Sync>,
    ) {
        self.event_hooks.write().unwrap().insert(
            HookCollectionCategory {
                collection_name,
                category,
            },
            hook,
        );
    }

    pub fn get_event_hook(
        &self,
        collection_name: &str,
        category: i32,
    ) -> Option<Arc<dyn EventCreatingHook + Send + Sync>> {
        let key = HookCollectionCategory {
            collection_name: collection_name.to_string(),
            category,
        };
        let map = self.event_hooks.read().unwrap();
        let value = map.get(&key);
        value.cloned()
    }

    pub fn insert_cron_default_interval_hook(
        &self,
        job_name: &str,
        collection_name: &str,
        document_selector: CronDocumentSelector,
        hook: Arc<dyn CronDefaultIntervalHook + Send + Sync>,
    ) {
        let key = CronDefaultIntervalHookData {
            job_name: job_name.to_string(),
            collection_name: collection_name.to_string(),
            document_selector,
        };
        let mut map = self.cron_default_interval_hooks.write().unwrap();
        map.insert(key, hook);
    }

    pub fn get_cron_default_interval_hooks(
        &self,
    ) -> Vec<(
        CronDefaultIntervalHookData,
        Arc<dyn CronDefaultIntervalHook + Send + Sync>,
    )> {
        self.cron_default_interval_hooks
            .read()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct RequestContext {
    #[allow(dead_code)]
    collection_name: String,
    user: UserWithRoles,
}

impl RequestContext {
    pub fn new(collection_name: &str, user: UserWithRoles) -> Self {
        Self {
            collection_name: collection_name.to_string(),
            user,
        }
    }

    #[allow(dead_code)]
    fn collection_name(&self) -> &str {
        self.collection_name.as_ref()
    }

    pub fn user_id(&self) -> Uuid {
        self.user.id()
    }

    pub fn user_name(&self) -> &str {
        self.user.name()
    }

    pub fn user(&self) -> &UserWithRoles {
        &self.user
    }
}

#[derive(Clone, Debug)]
pub enum HookContextData {
    DocumentAdding {
        document: CollectionItem, // TODO: use dto::CollectionDocument
    },
    DocumentUpdating {
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
    },
    EventAdding {
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
        collection: dto::Collection,
        event: dto::Event,
    },
    EventAdded {
        collection: dto::Collection,
        event: dto::Event,
    },
    Cron {
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
    },
}

impl HookContextData {
    pub fn after_document(&self) -> Option<&dto::CollectionDocument> {
        match self {
            HookContextData::DocumentAdding { document: _ } => None,
            HookContextData::DocumentUpdating {
                before_document: _,
                after_document,
            } => Some(after_document),
            HookContextData::EventAdding {
                before_document: _,
                after_document,
                collection: _,
                event: _,
            } => Some(after_document),
            HookContextData::EventAdded {
                collection: _,
                event: _,
            } => None,
            HookContextData::Cron {
                before_document: _,
                after_document,
            } => Some(after_document),
        }
    }
}

pub struct HookContext {
    data: Arc<HookContextData>,
    context: Arc<RequestContext>,
    tx: tokio::sync::oneshot::Sender<HookResult>,
    data_service: Arc<dyn DataService>,
}

impl HookContext {
    pub fn new(
        data: HookContextData,
        context: RequestContext,
        tx: tokio::sync::oneshot::Sender<HookResult>,
        data_service: Arc<dyn DataService>,
    ) -> Self {
        Self {
            data: Arc::new(data),
            context: Arc::new(context),
            tx,
            data_service,
        }
    }

    pub fn complete(self, result: HookResult) {
        let _ = self.tx.send(result);
    }

    pub fn context(&self) -> Arc<RequestContext> {
        self.context.clone()
    }

    pub fn data(&self) -> Arc<HookContextData> {
        self.data.clone()
    }

    pub fn data_service(&self) -> &dyn DataService {
        self.data_service.as_ref()
    }
}

impl Debug for HookContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookContext")
            .field("data", &self.data)
            .field("context", &self.context)
            .finish()
    }
}
