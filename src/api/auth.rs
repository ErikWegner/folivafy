use anyhow::{anyhow, Result};
use reqwest::Url;
use serde::Deserialize;
use tracing::debug;

#[derive(Deserialize)]
struct OpenIdConfiguration {
    jwks_uri: String,
}

#[derive(Deserialize)]
struct CertsX5CResponse {
    r#use: String,
    x5c: Vec<String>,
}

#[derive(Deserialize)]
struct CertsResponse {
    keys: Vec<CertsX5CResponse>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct RealmAccess {
    roles: Vec<String>,
}

// struct representing the authorized caller, deserializable from JWT claims
#[derive(Debug, Deserialize, Clone)]
pub(crate) struct User {
    sub: String,
    preferred_username: String,
    realm_access: RealmAccess,
}

impl User {
    pub(crate) fn is_collections_administrator(&self) -> bool {
        self.realm_access
            .roles
            .contains(&"A_FOLIVAFY_COLLECTION_EDITOR".to_string())
    }
    pub(crate) fn name_and_sub(&self) -> String {
        format!("{} ({})", self.preferred_username, self.sub)
    }
}

/// Workaround for  https://github.com/Keats/jsonwebtoken/issues/252 not handling RSA-OAEP
pub async fn cert_loader(issuer: &str) -> Result<String> {
    debug!("Loading certificates from {}", issuer);

    let mut url = Url::parse(issuer).map_err(|e| anyhow!("Invalid issuer {}", e.to_string()))?;

    url.path_segments_mut()
        .map_err(|_| anyhow!("Issuer URL error! ('{issuer}' cannot be a base)"))?
        .pop_if_empty()
        .extend(&[".well-known", "openid-configuration"]);

    let discovery_endpoint = url.to_string();

    let openid_configuration = reqwest::get(&discovery_endpoint)
        .await
        .map_err(|e| {
            anyhow!(
                "Endpoint {} could not be loaded: {:?}",
                discovery_endpoint,
                e
            )
        })?
        .json::<OpenIdConfiguration>()
        .await
        .map_err(|e| {
            anyhow!(
                "Could not parse response from {}: {:?}",
                discovery_endpoint,
                e
            )
        })?;
    let certs_uri = openid_configuration.jwks_uri;
    let certs_response = reqwest::get(&certs_uri)
        .await
        .map_err(|e| {
            anyhow!(
                "Certificates could not be loaded from {}: {:?}",
                certs_uri,
                e
            )
        })?
        .json::<CertsResponse>()
        .await
        .map_err(|e| anyhow!("Could not parse response from {}: {:?}", certs_uri, e))?;
    let certs_key = certs_response
        .keys
        .iter()
        .find_map(|f| {
            if f.r#use == "sig" {
                Some(format!(
                    "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                    f.x5c[0]
                ))
            } else {
                None
            }
        })
        .expect("No verification key provided");
    Ok(certs_key)
}
