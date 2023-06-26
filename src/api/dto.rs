use uuid::Uuid;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Collection {
    name: String,
    title: String,
    oao: bool,
    locked: bool,
}

impl Collection {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

impl From<&entity::collection::Model> for Collection {
    fn from(model: &entity::collection::Model) -> Self {
        Self {
            name: model.name.clone(),
            title: model.title.clone(),
            oao: model.oao,
            locked: model.locked,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDocument {
    id: Uuid,
    fields: serde_json::Value,
}

impl std::hash::Hash for CollectionDocument {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.fields.to_string().hash(state);
    }
}

impl CollectionDocument {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn fields(&self) -> &serde_json::Value {
        &self.fields
    }
}

impl From<&entity::collection_document::Model> for CollectionDocument {
    fn from(model: &entity::collection_document::Model) -> Self {
        Self {
            id: model.id,
            fields: model.f.clone(),
        }
    }
}

impl From<openapi::models::CollectionItem> for CollectionDocument {
    fn from(value: openapi::models::CollectionItem) -> Self {
        Self {
            id: value.id,
            fields: value.f,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    category: i32,
    payload: serde_json::Value,
}

impl Event {
    pub fn category(&self) -> i32 {
        self.category
    }
}

impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.payload.to_string().hash(state);
    }
}

impl Event {
    pub fn new(category: i32, payload: serde_json::Value) -> Self {
        Self { category, payload }
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
}
