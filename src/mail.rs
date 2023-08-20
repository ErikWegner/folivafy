use anyhow::Result;
use entity::collection;
use lazy_static::lazy_static;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

use crate::{
    api::{
        db::get_collection_by_name,
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

pub(crate) async fn insert_mail_cron_hook(
    hooks: &mut Hooks,
    db: &DatabaseConnection,
) -> Result<BackgroundTask> {
    ensure_mail_collection_exists(db).await?;
    let (shutdown_mail_signal, mut shutdown_mail_recv) = oneshot::channel::<()>();
    let (tx, mut rx) = mpsc::channel::<hooks::HookContext>(1);
    let join_handle = tokio::spawn(async move {
        debug!("Mail job started");
        loop {
            tokio::select! {
                _ = &mut shutdown_mail_recv => {
                    debug!("Mail job shutdown signal received");
                    break;
                }
                _ = rx.recv() => {
                    debug!("Mail job hook received");
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
            value: "pending".to_string(),
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
