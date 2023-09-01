use std::sync::{Arc, RwLock};

use anyhow::Context;

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{debug, info};
use uuid::Uuid;

use crate::BackgroundTask;

use super::ClientCredentials;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    id: String,
    email: Option<String>,
    #[serde(rename = "firstName")]
    first_name: Option<String>,
    #[serde(rename = "lastName")]
    last_name: Option<String>,
}

impl User {
    pub fn email(&self) -> Option<String> {
        self.email.as_ref().cloned()
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn first_name(&self) -> Option<String> {
        self.first_name.as_ref().cloned()
    }

    pub fn last_name(&self) -> Option<String> {
        self.last_name.as_ref().cloned()
    }
}

pub struct UserService {
    userinfo_url: String,
    auth_token: Arc<RwLock<Option<String>>>,
    danger_accept_invalid_certs: bool,
}

impl UserService {
    pub(crate) async fn new_from_env() -> anyhow::Result<(UserService, BackgroundTask)> {
        let client_id =
            std::env::var("USERDATA_CLIENT_ID").context("USERDATA_CLIENT_ID not defined")?;
        let client_secret = std::env::var("USERDATA_CLIENT_SECRET")
            .context("USERDATA_CLIENT_SECRET not defined")?;
        let token_url =
            std::env::var("USERDATA_TOKEN_URL").context("USERDATA_TOKEN_URL not defined")?;
        let userinfo_url =
            std::env::var("USERDATA_USERINFO_URL").context("USERDATA_USERINFO_URL")?;
        let auth_token = Arc::new(RwLock::new(None));

        let client_credentials = ClientCredentials {
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
            token_url: token_url.clone(),
        };
        let danger_accept_invalid_certs =
            std::env::var("IPASERVICE_DANGEROUS_ACCEPT_INVALID_CERTS")
                .unwrap_or_default()
                .eq_ignore_ascii_case("true");

        let thread_auth_token = auth_token.clone();
        let (shutdown_signal, mut shutdown_recv) = oneshot::channel::<()>();
        let join_handle = tokio::spawn(async move {
            let thread_credentials = client_credentials;
            let mut skip = 0;

            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
            loop {
                tokio::select! {
                    _ = &mut shutdown_recv => break,
                    _ = interval.tick() => {
                        if skip > 0 {
                            skip -= 1;
                        } else {
                            let token_response = super::get_token(&thread_credentials).await;
                            if let Ok(token_response) = token_response {
                                {
                                    let mut token = thread_auth_token.write().unwrap();
                                    *token = Some(token_response.clone());
                                }
                                skip = 11;
                            } else {
                                info!("Failed to get token, retrying in 15 seconds");
                                skip = 0;
                            }
                        }
                    }
                }
            }
        });

        Ok((
            UserService {
                userinfo_url,
                auth_token,
                danger_accept_invalid_certs,
            },
            BackgroundTask {
                name: "user-service".to_string(),
                join_handle,
                shutdown_signal,
            },
        ))
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> anyhow::Result<User> {
        let token;
        {
            let auth_token = self.auth_token.read().unwrap();
            token = (*auth_token).as_ref().cloned();
        }

        let token = token.ok_or_else(|| anyhow::anyhow!("No auth token"))?;
        let url = self.userinfo_url.replace("{id}", &id.to_string());
        debug!("Getting user info from {}", url);

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(self.danger_accept_invalid_certs)
            .build()?;

        let response = client
            .get(&url)
            .bearer_auth(&token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get user info: {:?}", e))?
            .json::<User>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse user info: {:?}", e))?;

        debug!("Got user info {:?}", response);
        Ok(response)
    }
}
