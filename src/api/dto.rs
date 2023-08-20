use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Collection {
    name: String,
    title: String,
    oao: bool,
    locked: bool,
}

impl Collection {
    pub fn new(name: String, title: String, oao: bool, locked: bool) -> Self {
        Self {
            name,
            title,
            oao,
            locked,
        }
    }

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
    pub fn new(id: Uuid, fields: serde_json::Value) -> Self {
        Self { id, fields }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn fields(&self) -> &serde_json::Value {
        &self.fields
    }

    pub fn set_field(&mut self, key: &str, value: serde_json::Value) {
        self.fields[key] = value;
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
    document_id: uuid::Uuid,
    category: i32,
    payload: serde_json::Value,
}

impl Event {
    pub fn new(document_id: Uuid, category: i32, payload: serde_json::Value) -> Self {
        Self {
            document_id,
            category,
            payload,
        }
    }

    pub fn document_id(&self) -> Uuid {
        self.document_id
    }

    pub fn category(&self) -> i32 {
        self.category
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MailMessageStatus {
    Pending,
    Sent,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MailMessage {
    to: String,
    subject: String,
    body_text: String,
    body_html: String,
    status: MailMessageStatus,
}

impl MailMessage {
    pub fn builder() -> MailMessageBuilder<(), (), ()> {
        MailMessageBuilder::new()
    }
}

pub struct MailMessageBuilder<T, S, B> {
    to: T,
    subject: S,
    body_text: B,
    body_html: B,
}

impl<T, S, B> MailMessageBuilder<T, S, B>
where
    T: std::default::Default + Copy,
    S: std::default::Default + Copy,
    B: std::default::Default + Copy,
{
    pub fn new() -> Self {
        Self {
            to: T::default(),
            subject: S::default(),
            body_text: B::default(),
            body_html: B::default(),
        }
    }

    pub fn to(&self, to: impl Into<String>) -> MailMessageBuilder<String, S, B> {
        MailMessageBuilder {
            to: to.into(),
            subject: self.subject,
            body_text: self.body_text,
            body_html: self.body_html,
        }
    }

    pub fn subject(&self, subject: impl Into<String>) -> MailMessageBuilder<T, String, B> {
        MailMessageBuilder {
            to: self.to,
            subject: subject.into(),
            body_text: self.body_text,
            body_html: self.body_html,
        }
    }

    pub fn body(
        &self,
        body_text: impl Into<String>,
        body_html: impl Into<String>,
    ) -> MailMessageBuilder<T, S, String> {
        MailMessageBuilder {
            to: self.to,
            subject: self.subject,
            body_text: body_text.into(),
            body_html: body_html.into(),
        }
    }
}

impl MailMessageBuilder<String, String, String> {
    pub fn build(self) -> MailMessage {
        MailMessage {
            to: self.to,
            subject: self.subject,
            body_text: self.body_text,
            body_html: self.body_html,
            status: MailMessageStatus::Pending,
        }
    }
}

impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.payload.to_string().hash(state);
    }
}
