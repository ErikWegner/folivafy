use std::time::SystemTime;

use crate::api::{auth, db::DELETED_AT_FIELD, CATEGORY_DOCUMENT_UPDATES};
use anyhow::Context;
use lettre::{
    message::{MultiPart, SinglePart},
    Message,
};
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
pub struct Grant {
    realm: String,
    grant_id: Uuid,
    view: bool,
}

impl Grant {
    pub fn new(realm: String, grant_id: Uuid, view: bool) -> Self {
        Self {
            realm,
            grant_id,
            view,
        }
    }

    pub fn realm(&self) -> &str {
        self.realm.as_ref()
    }

    pub fn grant_id(&self) -> Uuid {
        self.grant_id
    }

    pub fn view(&self) -> bool {
        self.view
    }
}

impl From<&entity::grant::Model> for Grant {
    fn from(value: &entity::grant::Model) -> Self {
        Self {
            realm: value.realm.clone(),
            grant_id: value.grant,
            view: value.view,
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

    pub fn is_deleted(&self) -> bool {
        let field = self.fields.get(DELETED_AT_FIELD);
        if let Some(field) = field {
            let s = field.as_str();
            if let Some(s) = s {
                return !s.is_empty();
            }
        }

        false
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
pub struct ExistingEvent {
    document_id: uuid::Uuid,
    category: i32,
    payload: serde_json::Value,
    user_id: Uuid,
    timestamp: i64,
}

impl ExistingEvent {
    pub fn new(
        document_id: uuid::Uuid,
        category: i32,
        payload: serde_json::Value,
        user_id: Uuid,
        timestamp: i64,
    ) -> Self {
        Self {
            document_id,
            category,
            payload,
            user_id,
            timestamp,
        }
    }

    pub fn is_create_event(&self) -> bool {
        self.category == CATEGORY_DOCUMENT_UPDATES
            && self
                .payload
                .get("new")
                .map(|jv| jv.as_bool().unwrap_or(false))
                .unwrap_or(false)
    }

    pub fn category(&self) -> i32 {
        self.category
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn document_id(&self) -> Uuid {
        self.document_id
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
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

impl From<&entity::event::Model> for ExistingEvent {
    fn from(model: &entity::event::Model) -> Self {
        Self {
            document_id: model.document_id,
            category: model.category_id,
            payload: model.payload.clone(),
            user_id: model.user,
            timestamp: model.timestamp.unwrap_or_default().timestamp(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MailMessageStatus {
    Pending,
    Sent(u64),
    Failed(u64),
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
    pub fn builder() -> MailMessageBuilder {
        MailMessageBuilder::new()
    }

    pub fn build_mail(&self, from: &str) -> anyhow::Result<lettre::Message> {
        Message::builder()
            .from(from.parse().context("From")?)
            .to(self.to.parse().context("Recipient")?)
            .subject(self.subject.clone())
            .multipart(
                MultiPart::mixed().multipart(
                    MultiPart::alternative()
                        .singlepart(SinglePart::plain(self.body_text.clone()))
                        .multipart(
                            MultiPart::related()
                                .singlepart(SinglePart::html(self.body_html.clone())),
                        ),
                ),
            )
            .context("Build mail")
    }

    pub fn set_sent(&mut self) {
        self.status = MailMessageStatus::Sent(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    pub fn to(&self) -> &str {
        self.to.as_ref()
    }

    pub fn subject(&self) -> &str {
        self.subject.as_ref()
    }
}

pub struct MailMessageBuilder {
    to: Option<String>,
    subject: Option<String>,
    body_text: Option<String>,
    body_html: Option<String>,
}

impl MailMessageBuilder {
    pub fn new() -> Self {
        Self {
            to: None,
            subject: None,
            body_text: None,
            body_html: None,
        }
    }

    pub fn set_to(mut self, to: &str) -> Self {
        self.to = Some(to.into());
        self
    }

    pub fn set_subject(mut self, subject: &str) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn set_body(mut self, body_text: &str, body_html: &str) -> Self {
        self.body_text = Some(body_text.into());
        self.body_html = Some(body_html.into());
        self
    }
}

impl Default for MailMessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MailMessageBuilder {
    pub fn build(self) -> Result<MailMessage, String> {
        if let (Some(to), Some(subject), Some(body_text), Some(body_html)) =
            (self.to, self.subject, self.body_text, self.body_html)
        {
            Ok(MailMessage {
                to,
                subject,
                body_text,
                body_html,
                status: MailMessageStatus::Pending,
            })
        } else {
            Err("Recipient, subject and body are required".to_string())
        }
    }
}

impl std::hash::Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.payload.to_string().hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct User {
    id: Uuid,
    name: String,
}

impl User {
    pub fn new(id: Uuid, name: String) -> Self {
        Self { id, name }
    }

    pub(crate) fn read_from(auth_user: &auth::User) -> Self {
        Self {
            id: auth_user.subuuid(),
            name: auth_user.preferred_username().to_string(),
        }
    }

    pub fn read_from_user_with_roles(user: &UserWithRoles) -> Self {
        Self {
            id: user.id,
            name: user.name.clone(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Debug)]
pub struct UserWithRoles {
    id: Uuid,
    name: String,
    roles: Vec<String>,
}

impl UserWithRoles {
    pub(crate) fn read_from(auth_user: &auth::User) -> Self {
        Self {
            id: auth_user.subuuid(),
            name: auth_user.preferred_username().to_string(),
            roles: auth_user.roles().iter().map(|r| r.to_string()).collect(),
        }
    }

    /// Returns the id of this [`User`].
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Returns a reference to the name of this [`User`].
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Check if the user has the specified role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
}
