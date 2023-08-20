use anyhow::Result;
use lazy_static::lazy_static;
use sea_orm::{DatabaseTransaction, DbErr, EntityTrait, TransactionTrait};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    api::{
        db::{get_collection_by_name, save_document_events_mails},
        dto,
        hooks::{HookContext, HookContextData, HookSuccessResult, Hooks, RequestContext},
        types::Pagination,
        ApiErrors,
    },
    BackgroundTask,
};

lazy_static! {
    pub static ref CRON_USER_ID: Uuid = Uuid::parse_str("cdf5c014-a59a-409e-a40a-56644cd6bad5")
        .expect("System Timer Uuid is not valid");
}
static CRON_USER_NAME: &str = "System Timer";

async fn cron(db: sea_orm::DatabaseConnection, hooks: Hooks) {
    debug!("Running cron tasks");
    let cron_limit = 100;
    let pagination = Pagination::new(1, 0);
    for (hookdata, listener) in hooks.get_cron_hooks() {
        if let crate::api::hooks::HookData::CronDefaultIntervalHook {
            job_name,
            collection_name,
            document_selector,
        } = hookdata
        {
            debug!("Running cron task: {job_name}");
            let collection = get_collection_by_name(&db, &collection_name).await;
            if let Some(collection) = collection {
                let mut counter = cron_limit;
                let (total, mut items) = super::api::db::list_documents(
                    &db,
                    collection.id,
                    None,
                    crate::api::db::CollectionDocumentVisibility::PublicAndUserIsReader,
                    "'title'".to_string(),
                    None,
                    vec![document_selector.clone().into()],
                    &pagination,
                )
                .await
                .unwrap_or_default();
                items.reverse();
                debug!("{job_name} found {total} documents, processing up to {cron_limit}");
                loop {
                    if counter == 0 {
                        break;
                    }
                    counter -= 1;

                    // for each result-id:
                    let item = items.pop();
                    if item.is_none() {
                        break;
                    }
                    let item = item.unwrap();
                    let id = item["id"].to_string();
                    let uuid = Uuid::parse_str(&id);
                    if uuid.is_err() {
                        error!("{job_name} failed to parse id {id}");
                        continue;
                    }
                    let uuid = uuid.unwrap();
                    debug!("Running cron task: {job_name} for document {id}");
                    //     select document for update
                    let tx_job_name = job_name.clone();
                    let tx_collection_name = collection_name.clone();
                    let tx_listener = listener.clone();
                    let _ = db
                        .transaction::<_, (), ApiErrors>(|txn| {
                            Box::pin(async move {
                                let document = select_document_for_update(uuid, txn).await?;
                                if document.is_none() {
                                    info!("Document vanished while running cron task: {tx_job_name} for document {id}");
                                    return Ok(());
                                }
                                // TODO: Check document is still matching filters
                                // call cron handler
                                let document = document.unwrap();
                                let before_document: dto::CollectionDocument = (&document).into();
                                let after_document: dto::CollectionDocument = (&document).into();
                                let result = run_hook(&tx_collection_name, before_document, after_document,tx_listener).await?;
                                // modified document? update document and save events
                                check_modifications_and_update(txn, result).await
                            })
                        })
                        .await;
                }
            } else {
                error!("Could not find collection: {collection_name}");
            }
        } else {
            debug!("Unknown hook data type: {:?}", hookdata);
        }
    }
}

pub(crate) fn setup_cron(
    db: sea_orm::DatabaseConnection,
    hooks: Hooks,
    cron_interval: std::time::Duration,
) -> (BackgroundTask, tokio::sync::mpsc::Sender<()>) {
    let mut interval = tokio::time::interval(cron_interval);
    debug!("cron_interval: {:?}", cron_interval);
    let (immediate_cron_signal, mut immediate_cron_recv) = mpsc::channel::<()>(1);
    let (shutdown_cron_signal, mut shutdown_cron_recv) = oneshot::channel::<()>();
    let join_handle = tokio::spawn(async move {
        debug!("Delaying cron start");
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
        debug!("Cron started");
        let loopdb = db;
        loop {
            tokio::select! {
                _ = &mut shutdown_cron_recv => {
                    debug!("Cron shutdown signal received");
                    break;
                }
                _ = interval.tick() => {
                    debug!("Cron tick");
                    cron(loopdb.clone(), hooks.clone()).await
                }
                _ = immediate_cron_recv.recv() => {
                    debug!("Immediate cron signal received");
                    cron(loopdb.clone(), hooks.clone()).await
                }
            }
        }
        debug!("Cron exited");
    });
    (
        BackgroundTask::new("cron", join_handle, shutdown_cron_signal),
        immediate_cron_signal,
    )
}

use entity::collection_document::Entity as Documents;
pub(crate) async fn select_document_for_update(
    unchecked_document_id: uuid::Uuid,
    txn: &DatabaseTransaction,
) -> Result<Option<entity::collection_document::Model>, DbErr> {
    Documents::find()
        .from_raw_sql(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            r#"SELECT * FROM "collection_document" WHERE "id" = $1 FOR UPDATE"#,
            [unchecked_document_id.into()],
        ))
        .one(txn)
        .await
}

async fn run_hook(
    collection_name: &str,
    before_document: dto::CollectionDocument,
    after_document: dto::CollectionDocument,
    hook_processor: Sender<HookContext>,
) -> Result<HookSuccessResult, ApiErrors> {
    let (tx, rx) = oneshot::channel::<Result<HookSuccessResult, ApiErrors>>();

    let cdctx = HookContext::new(
        HookContextData::Cron {
            before_document,
            after_document,
        },
        RequestContext::new(collection_name, *CRON_USER_ID, CRON_USER_NAME),
        tx,
    );
    let _ = hook_processor
        .send(cdctx)
        .await
        .map_err(|_| ApiErrors::InternalServerError)?;
    rx.await.map_err(|_| ApiErrors::InternalServerError)?
}

async fn check_modifications_and_update(
    txn: &DatabaseTransaction,
    result: HookSuccessResult,
) -> Result<(), ApiErrors> {
    let mut document = None;
    match result.document {
        crate::api::hooks::DocumentResult::Store(new_document) => document = Some(new_document),
        crate::api::hooks::DocumentResult::NoUpdate => {}
        crate::api::hooks::DocumentResult::Err(e) => return Err(e),
    }
    save_document_events_mails(
        txn,
        &CRON_USER_ID,
        document,
        None,
        result.events,
        result.mails,
    )
    .await
    .map_err(|e| {
        error!("Update document error: {:?}", e);
        ApiErrors::InternalServerError
    })?;
    Ok(())
}
