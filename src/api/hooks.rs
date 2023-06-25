use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use openapi::models::CollectionItem;
use tokio::sync::mpsc::Sender;

use super::{dto, ApiErrors};

pub struct HookSuccessResult {
    pub document: dto::CollectionDocument,
    pub events: Vec<dto::Event>,
}

type HookResult = Result<HookSuccessResult, ApiErrors>;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum ItemActionType {
    AppendEvent,
    Create,
    Update,
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum ItemActionStage {
    Before,
    After,
}

#[derive(Eq, Hash, PartialEq)]
struct HookData {
    collection_name: String,
    item_action_type: ItemActionType,
    item_action_stage: ItemActionStage,
}

#[derive(Clone)]
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
        let hook_data = HookData {
            collection_name: collection_name.to_string(),
            item_action_type: action,
            item_action_stage: stage,
        };
        let mut m = self.hooks.write().unwrap();
        m.insert(hook_data, tx);
    }

    pub fn execute_hook(
        &self,
        collection_name: &str,
        action: ItemActionType,
        stage: ItemActionStage,
    ) -> Option<Sender<HookContext>> {
        let hook_data = HookData {
            collection_name: collection_name.to_string(),
            item_action_type: action,
            item_action_stage: stage,
        };
        let a = self.hooks.read().unwrap();
        let b = a.get(&hook_data);
        b.cloned()
    }
}

#[derive(Debug)]
pub struct RequestContext {
    #[allow(dead_code)]
    collection_name: String,
}

impl RequestContext {
    pub fn new(collection: entity::collection::Model) -> Self {
        Self {
            collection_name: collection.name,
        }
    }

    #[allow(dead_code)]
    fn collection_name(&self) -> &str {
        self.collection_name.as_ref()
    }
}

#[derive(Clone, Debug)]
pub enum HookContextData {
    DocumentAdding {
        document: CollectionItem, // TODO: use dto::CollectionDocument
    },
    EventAdding {
        document: dto::CollectionDocument,
        collection: dto::Collection,
        event: dto::Event,
    },
}

pub struct HookContext {
    data: Arc<HookContextData>,
    context: RequestContext,
    tx: tokio::sync::oneshot::Sender<HookResult>,
}

impl HookContext {
    pub fn new(
        data: HookContextData,
        context: RequestContext,
        tx: tokio::sync::oneshot::Sender<HookResult>,
    ) -> Self {
        Self {
            data: Arc::new(data),
            context,
            tx,
        }
    }

    pub fn complete(self, result: HookResult) {
        let _ = self.tx.send(result);
    }

    pub fn context(&self) -> &RequestContext {
        &self.context
    }

    pub fn data(&self) -> Arc<HookContextData> {
        self.data.clone()
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
