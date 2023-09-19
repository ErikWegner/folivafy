use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use openapi::models::CollectionItem;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use super::{data_service::DataService, dto, ApiErrors};

pub enum DocumentResult {
    /// Indicates that the document was modified and should be inserted/updated.
    Store(dto::CollectionDocument),
    /// Indicates that the document was not modified or no document can be created.
    NoUpdate,
    /// Indicates the error that occurred.
    Err(ApiErrors),
}

pub struct HookSuccessResult {
    pub document: DocumentResult,
    pub events: Vec<dto::Event>,
    pub mails: Vec<dto::MailMessage>,
    pub trigger_cron: bool,
}

impl Debug for HookSuccessResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookSuccessResult").finish()
    }
}

type HookResult = Result<HookSuccessResult, ApiErrors>;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum ItemActionType {
    AppendEvent { category: i32 },
    Create,
    CronDefaultInterval,
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum ItemActionStage {
    Before,
    After,
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub enum CronDocumentSelector {
    ByFieldEqualsValue { field: String, value: String },
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub(crate) enum HookData {
    ItemHook {
        collection_name: String,
        item_action_type: ItemActionType,
        item_action_stage: ItemActionStage,
    },
    CronDefaultIntervalHook {
        job_name: String,
        collection_name: String,
        document_selector: CronDocumentSelector,
    },
}

pub struct HookCreateContext {
    document: dto::CollectionDocument,
    data_service: Arc<DataService>,
    context: Arc<RequestContext>,
}

impl HookCreateContext {
    pub fn new(
        document: dto::CollectionDocument,
        data_service: Arc<DataService>,
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

    pub fn data_service(&self) -> &DataService {
        self.data_service.as_ref()
    }

    pub fn context(&self) -> &RequestContext {
        self.context.as_ref()
    }
}

pub struct HookUpdateContext {
    before_document: dto::CollectionDocument,
    after_document: dto::CollectionDocument,
    data_service: Arc<DataService>,
    context: Arc<RequestContext>,
}

impl HookUpdateContext {
    pub fn new(
        before_document: dto::CollectionDocument,
        after_document: dto::CollectionDocument,
        data_service: Arc<DataService>,
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

    pub fn data_service(&self) -> &DataService {
        self.data_service.as_ref()
    }

    pub fn context(&self) -> &RequestContext {
        self.context.as_ref()
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

#[derive(Clone)]
pub struct HooksN {
    create: Option<Arc<dyn DocumentCreatingHook + Send + Sync>>,
    updates: Option<Arc<dyn DocumentUpdatingHook + Send + Sync>>,
}

impl HooksN {
    pub fn new() -> Self {
        Self {
            create: None,
            updates: None,
        }
    }

    pub fn create(&self) -> Option<&Arc<dyn DocumentCreatingHook + Send + Sync>> {
        self.create.as_ref()
    }

    pub fn updates(&self) -> Option<&Arc<dyn DocumentUpdatingHook + Send + Sync>> {
        self.updates.as_ref()
    }
}

#[derive(Clone)]
#[deprecated(note = "use HooksN instead")]
pub struct Hooks {
    hooks: Arc<RwLock<HashMap<HookData, Sender<HookContext>>>>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

impl Hooks {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn insert(
        &mut self,
        collection_name: &str,
        action: ItemActionType,
        stage: ItemActionStage,
        tx: Sender<HookContext>,
    ) {
        let hook_data = HookData::ItemHook {
            collection_name: collection_name.to_string(),
            item_action_type: action,
            item_action_stage: stage,
        };
        let mut m = self.hooks.write().unwrap();
        m.insert(hook_data, tx);
    }

    /// Insert a cron job running at the default interval.
    pub fn insert_cron_default_interval(
        &mut self,
        job_name: &str,
        collection_name: &str,
        selector: CronDocumentSelector,
        tx: Sender<HookContext>,
    ) {
        let hook_data = HookData::CronDefaultIntervalHook {
            job_name: job_name.to_string(),
            collection_name: collection_name.to_string(),
            document_selector: selector,
        };
        let mut m = self.hooks.write().unwrap();
        m.insert(hook_data, tx);
    }

    pub fn get_registered_hook(
        &self,
        collection_name: &str,
        action: ItemActionType,
        stage: ItemActionStage,
    ) -> Option<Sender<HookContext>> {
        let hook_data = HookData::ItemHook {
            collection_name: collection_name.to_string(),
            item_action_type: action,
            item_action_stage: stage,
        };
        let a = self.hooks.read().unwrap();
        let b = a.get(&hook_data);
        b.cloned()
    }

    pub fn split_cron_hooks(self) -> (Hooks, Hooks) {
        let mut a = self.hooks.write().unwrap();

        let mut cronhooks = HashMap::new();
        let mut requesthooks = HashMap::new();
        for (k, v) in a.drain() {
            match k {
                HookData::ItemHook {
                    collection_name: _,
                    item_action_type: _,
                    item_action_stage: _,
                } => requesthooks.insert(k, v),
                HookData::CronDefaultIntervalHook {
                    job_name: _,
                    collection_name: _,
                    document_selector: _,
                } => cronhooks.insert(k, v),
            };
        }
        (
            Hooks {
                hooks: Arc::new(RwLock::new(requesthooks)),
            },
            Hooks {
                hooks: Arc::new(RwLock::new(cronhooks)),
            },
        )
    }

    pub(crate) fn get_cron_hooks(&self) -> HashMap<HookData, Sender<HookContext>> {
        let a = self.hooks.read().unwrap();
        let mut hooks = HashMap::new();
        for (k, v) in a.iter() {
            match k {
                HookData::ItemHook {
                    collection_name: _,
                    item_action_type: _,
                    item_action_stage: _,
                } => {}
                HookData::CronDefaultIntervalHook {
                    job_name: _,
                    collection_name: _,
                    document_selector: _,
                } => {
                    hooks.insert(k.clone(), v.clone());
                }
            };
        }
        hooks
    }
}

#[derive(Debug)]
pub struct RequestContext {
    #[allow(dead_code)]
    collection_name: String,
    user_id: Uuid,
    user_name: String,
}

impl RequestContext {
    pub fn new(collection_name: &str, user_id: Uuid, user_name: &str) -> Self {
        Self {
            collection_name: collection_name.to_string(),
            user_id,
            user_name: user_name.to_string(),
        }
    }

    #[allow(dead_code)]
    fn collection_name(&self) -> &str {
        self.collection_name.as_ref()
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn user_name(&self) -> &str {
        self.user_name.as_ref()
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
    data_service: Arc<DataService>,
}

impl HookContext {
    pub fn new(
        data: HookContextData,
        context: RequestContext,
        tx: tokio::sync::oneshot::Sender<HookResult>,
        data_service: Arc<DataService>,
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

    pub fn data_service(&self) -> &DataService {
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
