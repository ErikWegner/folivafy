use std::collections::HashMap;

use reqwest::StatusCode;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use self::user_service::User;

use super::dto::{self, ExistingEvent};

mod document_service;
mod event_service;
pub(crate) mod user_service;

pub(crate) struct ClientCredentials {
    pub(crate) token_url: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
}

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
}

pub struct DataService {
    db: DatabaseConnection,
    document_service: document_service::DocumentService,
    event_service: event_service::DocumentEventService,
    user_service: user_service::UserService,
}

impl DataService {
    pub(crate) fn new(db: &DatabaseConnection, user_service: user_service::UserService) -> Self {
        Self {
            db: db.clone(),
            document_service: document_service::DocumentService::new(),
            event_service: event_service::DocumentEventService::new(),
            user_service,
        }
    }

    pub async fn get_document_events(
        &self,
        document_id: Uuid,
    ) -> anyhow::Result<Vec<ExistingEvent>> {
        self.event_service
            .get_document_events_newest_first(&self.db, document_id)
            .await
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> anyhow::Result<User> {
        self.user_service.get_user_by_id(user_id).await
    }

    pub async fn get_document(
        &self,
        collection_name: &str,
        document_id: Uuid,
    ) -> Option<dto::CollectionDocument> {
        self.document_service
            .get_document(&self.db, collection_name, document_id)
            .await
    }
}

pub(crate) async fn get_token(client_credentials: &ClientCredentials) -> anyhow::Result<String> {
    debug!("Fetching token from {}", client_credentials.token_url);
    let mut form_data = HashMap::new();

    form_data.insert("grant_type", "client_credentials");
    form_data.insert("client_id", client_credentials.client_id.as_str());
    form_data.insert("client_secret", client_credentials.client_secret.as_str());
    form_data.insert("scope", "openid");

    let client = reqwest::Client::new();
    let res = client
        .post(client_credentials.token_url.clone())
        .form(&form_data)
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await;
    match res {
        Ok(o) => {
            if o.status() != StatusCode::OK {
                anyhow::bail!(o.text().await.unwrap());
            }
            let token_response = o.json::<TokenResponse>().await;
            match token_response {
                Ok(tokendata) => Ok(tokendata.access_token),
                Err(e) => anyhow::bail!(e.to_string()),
            }
        }
        Err(e) => anyhow::bail!(e.to_string()),
    }
}
