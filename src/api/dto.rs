#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Collection {
    id: String,
    name: String,
}

impl Collection {
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct CollectionItem {
    id: String,
    collection: Collection,
}

impl CollectionItem {
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn collection(&self) -> &Collection {
        &self.collection
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Event {
    payload: serde_json::Value,
}

impl Event {
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
}
