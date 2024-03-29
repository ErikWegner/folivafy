use std::{env, str::FromStr, sync::Arc};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use entity::collection;
use lazy_static::lazy_static;
use lettre::{
    transport::smtp::{
        authentication::Credentials,
        client::{Certificate, Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set};
use tracing::{debug, error, info};

use crate::api::{
    db::get_collection_by_name,
    dto::{self, MailMessage},
    hooks::{self, CronDefaultIntervalHook, GrantSettings, HookCronContext, HookResult, Hooks},
    ApiErrors,
};

lazy_static! {
    pub static ref FOLIVAFY_MAIL_COLLECTION_ID: uuid::Uuid =
        uuid::Uuid::parse_str("24297847-b6ba-447f-9c0d-7f1674fba924")
            .expect("Mail collection ID is invalid");
}
static FOLIVAFY_MAIL_COLLECTION_NAME: &str = "folivafy-mail";

#[derive(Debug, Clone)]
pub(crate) enum SmtpConnectionType {
    Starttls,
    Tls,
    Plain,
}

impl FromStr for SmtpConnectionType {
    type Err = ();

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "SMTP" => Ok(SmtpConnectionType::Plain),
            "TLS" => Ok(SmtpConnectionType::Tls),
            "STARTTLS" => Ok(SmtpConnectionType::Starttls),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SmtpClientConfiguration {
    server: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    certificate: Option<Vec<u8>>,
    connection_type: SmtpConnectionType,
    from_address: String,
}

impl SmtpClientConfiguration {
    pub(crate) async fn from_env() -> Result<Self> {
        let connection_type = SmtpConnectionType::from_str(
            &env::var("FOLIVAFY_MAIL_SERVER_TYPE")
                .context("FOLIVAFY_MAIL_SERVER_TYPE is not set")?,
        )
        .map_err(|_| anyhow!("Invalid value for FOLIVAFY_MAIL_SERVER_TYPE"))?;
        let server = env::var("FOLIVAFY_MAIL_SERVER").unwrap_or_else(|_| "localhost".into());
        let port = env::var("FOLIVAFY_MAIL_PORT")
            .unwrap_or_else(|_| {
                match connection_type {
                    SmtpConnectionType::Starttls => "587",
                    SmtpConnectionType::Tls => "465",
                    SmtpConnectionType::Plain => "25",
                }
                .to_string()
            })
            .parse()
            .unwrap_or(465);
        let username = env::var("FOLIVAFY_MAIL_USERNAME").ok();
        let password = env::var("FOLIVAFY_MAIL_PASSWORD").ok();
        let certificate_filename = env::var("FOLIVAFY_MAIL_CERTIFICATE_FILE").ok();
        let certificate = if let Some(filename) = certificate_filename {
            Some(std::fs::read(filename).context("Reading mail server certificate")?)
        } else {
            None
        };
        let from_address =
            env::var("FOLIVAFY_MAIL_FROM_ADDRESS").context("FOLIVAFY_MAIL_FROM_ADDRESS not set")?;
        let i = Self {
            server: server.clone(),
            port,
            username,
            password,
            certificate,
            connection_type,
            from_address,
        };
        debug!("Testing connection to mail server");
        i.transport().test_connection().await.with_context(|| {
            format!("Connection to mail server `{server}` on port {port} failed")
        })?;
        debug!("Connection to mail server established");
        Ok(i)
    }

    pub(crate) fn transport(&self) -> AsyncSmtpTransport<Tokio1Executor> {
        let mut b = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(self.server.clone());
        b = b.port(self.port);
        if let Some(ref username) = self.username {
            let creds =
                Credentials::new(username.clone(), self.password.clone().unwrap_or_default());
            b = b.credentials(creds);
        }
        if let Some(ref certificate) = self.certificate {
            let cert = Certificate::from_pem(certificate).unwrap();
            let tls = TlsParameters::builder(self.server.clone())
                .add_root_certificate(cert)
                .build()
                .unwrap();
            match self.connection_type {
                SmtpConnectionType::Starttls => {
                    b = b.tls(Tls::Required(tls));
                }
                SmtpConnectionType::Tls => {
                    b = b.tls(Tls::Wrapper(tls));
                }
                SmtpConnectionType::Plain => {}
            }
        } else {
            match self.connection_type {
                SmtpConnectionType::Starttls => {
                    let tls_parameters = TlsParameters::new(self.server.clone()).unwrap();
                    b = b.tls(Tls::Required(tls_parameters))
                }
                SmtpConnectionType::Tls => {
                    let tls_parameters = TlsParameters::new(self.server.clone()).unwrap();
                    b = b.tls(Tls::Wrapper(tls_parameters))
                }
                SmtpConnectionType::Plain => {}
            }
        }
        b.timeout(Some(std::time::Duration::from_secs(10))).build()
    }
}

struct Mailer {
    smtp_cfg: SmtpClientConfiguration,
}

impl Mailer {
    fn new(smtp_cfg: SmtpClientConfiguration) -> Self {
        Self { smtp_cfg }
    }
}

#[async_trait]
impl CronDefaultIntervalHook for Mailer {
    async fn on_default_interval(&self, context: &HookCronContext) -> HookResult {
        let document_id = context.after_document().id();
        let mut maildocument =
            serde_json::from_value::<MailMessage>(context.after_document().fields().clone())
                .map_err(|e| {
                    error!("Cannot read mail message ({document_id}) from store: {}", e);
                    ApiErrors::InternalServerError
                })?;
        let email = maildocument
            .build_mail(self.smtp_cfg.from_address.as_ref())
            .map_err(|e| {
                error!("Cannot build message from {document_id}: {}", e);
                ApiErrors::InternalServerError
            })?;
        let mailer = self.smtp_cfg.transport();

        // Send the email
        match mailer.send(email).await {
            Ok(_) => {
                debug!("Email {document_id} sent successfully!");
                maildocument.set_sent();
                let o = dto::CollectionDocument::new(
                    *document_id,
                    serde_json::to_value(maildocument).unwrap(),
                );
                Ok(hooks::HookSuccessResult {
                    document: hooks::DocumentResult::Store(o),
                    grants: GrantSettings::NoChange,
                    events: vec![],
                    mails: vec![],
                    trigger_cron: false,
                })
            }
            Err(e) => {
                error!("Could not send email: {:?}", e);
                Err(ApiErrors::InternalServerError)
            }
        }
    }
}

pub(crate) async fn insert_mail_cron_hook(hooks: &Hooks, db: &DatabaseConnection) -> Result<()> {
    ensure_mail_collection_exists(db).await?;
    let smtp_cfg = SmtpClientConfiguration::from_env().await?;
    let mailer = Arc::new(Mailer::new(smtp_cfg));
    hooks.insert_cron_default_interval_hook(
        "folivafy mailer",
        "folivafy-mail",
        hooks::CronDocumentSelector::ByFieldEqualsValue {
            field: "status".to_string(),
            value: "Pending".to_string(),
        },
        mailer,
    );
    Ok(())
}

async fn ensure_mail_collection_exists(db: &DatabaseConnection) -> Result<(), DbErr> {
    let exists = get_collection_by_name(db, FOLIVAFY_MAIL_COLLECTION_NAME)
        .await
        .is_some();
    if exists {
        debug!("Mail collection exists: {}", FOLIVAFY_MAIL_COLLECTION_NAME);
        return Ok(());
    }
    info!("Creating collection: {}", FOLIVAFY_MAIL_COLLECTION_NAME);
    let collection = collection::ActiveModel {
        id: Set(*FOLIVAFY_MAIL_COLLECTION_ID),
        name: Set(FOLIVAFY_MAIL_COLLECTION_NAME.to_string()),
        title: Set("Folivafy mail".to_string()),
        oao: Set(true),
        ..Default::default()
    };

    entity::collection::Entity::insert(collection)
        .exec(db)
        .await
        .map(|_| Ok(()))?
}
