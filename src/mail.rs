use std::{env, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
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
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use crate::{
    api::{
        db::get_collection_by_name,
        dto::{self, MailMessage},
        hooks::{self, Hooks},
    },
    BackgroundTask,
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
        let from_address = env::var("FOLIVAFY_MAIL_FROM_ADDRESS")?;
        let i = Self {
            server,
            port,
            username,
            password,
            certificate,
            connection_type,
            from_address,
        };
        debug!("Testing connection to mail server");
        i.transport().test_connection().await?;
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

async fn process_hook(
    context: &hooks::HookContext,
    smtp_cfg: &SmtpClientConfiguration,
) -> Result<dto::CollectionDocument> {
    match *context.data() {
        hooks::HookContextData::Cron {
            before_document: _,
            ref after_document,
        } => {
            let mut maildocument =
                serde_json::from_value::<MailMessage>(after_document.fields().clone())
                    .with_context(|| format!("Invalid mail document {}", after_document.id()))?;
            let email = maildocument
                .build_mail(smtp_cfg.from_address.as_ref())
                .context("Prepare email")?;
            let mailer = smtp_cfg.transport();

            // Send the email
            match mailer.send(email).await {
                Ok(_) => {
                    debug!("Email sent successfully!");
                    maildocument.set_sent();
                    Ok(dto::CollectionDocument::new(
                        *after_document.id(),
                        serde_json::to_value(maildocument).unwrap(),
                    ))
                }
                Err(e) => bail!("Could not send email: {:?}", e),
            }
        }
        _ => {
            error!("Unsupported hook type");
            bail!("Unsupported hook type");
        }
    }
}

pub(crate) async fn insert_mail_cron_hook(
    hooks: &mut Hooks,
    db: &DatabaseConnection,
) -> Result<BackgroundTask> {
    ensure_mail_collection_exists(db).await?;
    let (shutdown_mail_signal, mut shutdown_mail_recv) = oneshot::channel::<()>();
    let (tx, mut rx) = mpsc::channel::<hooks::HookContext>(1);

    let smtp_cfg = SmtpClientConfiguration::from_env().await?;
    let join_handle = tokio::spawn(async move {
        debug!("Mail job started");
        loop {
            tokio::select! {
                _ = &mut shutdown_mail_recv => {
                    debug!("Mail job shutdown signal received");
                    break;
                }
                r = rx.recv() => {
                    if let Some(ctx) = r {
                        debug!("Mail job hook received");
                        let phr = process_hook(&ctx, &smtp_cfg).await;
                        ctx.complete(
                            match phr {
                                Err(mailerr) => {
                                    error!("Error occured {:?}", mailerr);
                                    Ok(
                                        hooks::HookSuccessResult {
                                            document: hooks::DocumentResult::NoUpdate,
                                            events: vec![],
                                            mails: vec![],
                                        }
                                    )
                                },
                                Ok(o) => Ok(
                                    hooks::HookSuccessResult {
                                        document: hooks::DocumentResult::Store(o),
                                        events: vec![],
                                        mails: vec![],
                                    }
                                )
                            }
                        );
                    }
                }
            }
        }
        debug!("Mail job stopped");
    });
    hooks.insert_cron_default_interval(
        "folivafy mailer",
        "folivafy-mail",
        hooks::CronDocumentSelector::ByFieldEqualsValue {
            field: "status".to_string(),
            value: "Pending".to_string(),
        },
        tx,
    );
    Ok(BackgroundTask::new(
        "folivafy mailer",
        join_handle,
        shutdown_mail_signal,
    ))
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
