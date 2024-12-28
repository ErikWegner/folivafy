use std::time::SystemTime;

use crate::api::{auth, db::DELETED_AT_FIELD, CATEGORY_DOCUMENT_UPDATES};
use crate::cron::CRON_USER_ID;
use crate::models::CollectionItem;
use anyhow::Context;
use lettre::message::Attachment;
use lettre::{
    message::{MultiPart, SinglePart},
    Message,
};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;
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

    pub fn author_grant(user_id: Uuid) -> Self {
        Self {
            realm: "author".to_string(),
            grant_id: user_id,
            view: true,
        }
    }

    pub fn read_all_collection(collection_id: Uuid) -> Self {
        Self {
            realm: "read-all-collection".to_string(),
            grant_id: collection_id,
            view: true,
        }
    }

    pub fn cron_access() -> Self {
        Self {
            realm: "cron-access".to_string(),
            grant_id: *CRON_USER_ID,
            view: true,
        }
    }

    pub fn is_cron_access(&self) -> bool {
        self.realm == "cron-access" && self.grant_id == *CRON_USER_ID
    }

    pub fn read_collection(collection_id: Uuid) -> Self {
        Self {
            realm: "read-collection".to_string(),
            grant_id: collection_id,
            view: true,
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

impl PartialEq<&entity::grant::Model> for &Grant {
    fn eq(&self, other: &&entity::grant::Model) -> bool {
        self.realm == other.realm && self.grant_id == other.grant && self.view == other.view
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

#[derive(Debug)]
pub struct GrantForDocument {
    grant: Grant,
    document_id: Uuid,
}

impl GrantForDocument {
    pub fn new(grant: Grant, document_id: Uuid) -> Self {
        Self { grant, document_id }
    }

    pub fn grant(&self) -> &Grant {
        &self.grant
    }

    pub fn document_id(&self) -> Uuid {
        self.document_id
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
    /// Creates a new CollectionDocument.
    pub fn new(id: Uuid, fields: serde_json::Value) -> Self {
        Self { id, fields }
    }

    /// Returns the id of the document.
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Returns a reference to the fields of the document.
    pub fn fields(&self) -> &serde_json::Value {
        &self.fields
    }

    /// Set a field in the document.
    pub fn set_field(&mut self, key: &str, value: serde_json::Value) {
        self.fields[key] = value;
    }

    /// Remove the field with the given key from the document.
    pub fn remove_field(&mut self, key: &str) {
        let _ = self.fields.as_object_mut().and_then(|obj| obj.remove(key));
    }

    /// Returns `true` if the document has been marked as deleted, `false` otherwise.
    pub fn is_deleted(&self) -> bool {
        // The field that stores the deletion timestamp.
        let field = self.fields.get(DELETED_AT_FIELD);

        // Extract the deletion timestamp from the document's fields.
        if let Some(field) = field {
            // The deletion timestamp is stored as a string.
            let s = field.as_str();

            // Check if the timestamp is empty.
            if let Some(s) = s {
                return !s.is_empty();
            }
        }

        // The document has not been marked as deleted.
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

impl From<CollectionItem> for CollectionDocument {
    fn from(value: CollectionItem) -> Self {
        Self {
            id: value.id,
            fields: value.f,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingEvent {
    event_id: i32,
    document_id: uuid::Uuid,
    category: i32,
    payload: serde_json::Value,
    user_id: Uuid,
    timestamp: i64,
}

impl ExistingEvent {
    pub fn new(
        event_id: i32,
        document_id: uuid::Uuid,
        category: i32,
        payload: serde_json::Value,
        user_id: Uuid,
        timestamp: i64,
    ) -> Self {
        Self {
            event_id,
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

    pub fn event_id(&self) -> i32 {
        self.event_id
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
            event_id: model.id,
            document_id: model.document_id,
            category: model.category_id,
            payload: model.payload.clone(),
            user_id: model.user,
            timestamp: model.timestamp.unwrap_or_default().and_utc().timestamp(),
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
    bcc: Option<String>,
    subject: String,
    body_text: String,
    body_html: String,
    status: MailMessageStatus,
    #[serde(default)]
    attachments: Vec<MailMessageAttachment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TypedBuilder)]
pub struct MailMessageAttachment {
    filename: String,
    mime_type: String,
    data: Vec<u8>,
}

impl MailMessage {
    pub fn builder() -> MailMessageBuilder {
        MailMessageBuilder::new()
    }

    pub fn build_mail(&self, from: &str) -> anyhow::Result<lettre::Message> {
        let mut m = MultiPart::mixed().multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(self.body_text.clone()))
                .multipart(
                    MultiPart::related().singlepart(SinglePart::html(self.body_html.clone())),
                ),
        );
        for attachment in self.attachments.iter() {
            m = m.singlepart(
                Attachment::new(attachment.filename.clone()).body(
                    attachment.data.clone(),
                    attachment
                        .mime_type
                        .parse()
                        .context("Attachment mime type")?,
                ),
            );
        }
        let mut b = Message::builder()
            .from(from.parse().context("From")?)
            .to(self.to.parse().context("Recipient")?)
            .subject(self.subject.clone());
        if let Some(bcc) = self.bcc.as_ref() {
            b = b.bcc(bcc.parse().context("Bcc")?);
        }

        b.multipart(m).context("Build mail")
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
    bcc: Option<String>,
    subject: Option<String>,
    body_text: Option<String>,
    body_html: Option<String>,
    attachments: Vec<MailMessageAttachment>,
}

impl MailMessageBuilder {
    pub fn new() -> Self {
        Self {
            to: None,
            bcc: None,
            subject: None,
            body_text: None,
            body_html: None,
            attachments: Vec::new(),
        }
    }

    pub fn set_to(mut self, to: &str) -> Self {
        self.to = Some(to.into());
        self
    }

    /// Set the bcc field of the mail message.
    ///
    /// # Parameters
    ///
    /// * `bcc` - The bcc field, as a string.
    ///
    /// # Returns
    ///
    /// A reference to the `MailMessageBuilder` object.
    pub fn set_bcc(mut self, bcc: &str) -> Self {
        self.bcc = Some(bcc.into());
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

    pub fn add_attachment(mut self, attachment: MailMessageAttachment) -> Self {
        self.attachments.push(attachment);
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
                bcc: self.bcc,
                subject,
                body_text,
                body_html,
                status: MailMessageStatus::Pending,
                attachments: self.attachments,
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
    pub fn new(id: Uuid, name: String, roles: Vec<String>) -> Self {
        Self { id, name, roles }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_access() {
        let cron_grant = Grant::cron_access();
        assert!(cron_grant.is_cron_access());
    }
}
