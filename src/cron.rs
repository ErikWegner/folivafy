use anyhow::Result;
use lazy_static::lazy_static;
use sea_orm::{DatabaseTransaction, DbErr, EntityTrait, TransactionTrait};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{self},
    oneshot,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    api::{
        data_service::DataService,
        db::{get_collection_by_name, save_document_events_mails, ListDocumentParams},
        dto,
        hooks::{HookCronContext, HookSuccessResult, Hooks},
        types::Pagination,
        ApiErrors,
    },
    BackgroundTask,
};

lazy_static! {
    pub static ref CRON_USER_ID: Uuid = Uuid::parse_str("cdf5c014-a59a-409e-a40a-56644cd6bad5")
        .expect("System Timer Uuid is not valid");
}

#[allow(dead_code)]
static CRON_USER_NAME: &str = "System Timer";

struct CronResult {
    trigger_cron: bool,
}

async fn cron(
    db: sea_orm::DatabaseConnection,
    hooks: &Hooks,
    data_service: Arc<crate::api::data_service::DataService>,
) -> CronResult {
    debug!("Running cron tasks");
    let mut trigger_cron = false;
    let cron_limit = 100;
    let pagination = Pagination::new(cron_limit, 0);
    let l = hooks.get_cron_default_interval_hooks();
    for (hookdata, listener) in l {
        let job_name = hookdata.job_name().to_string();
        let collection_name = hookdata.collection_name();
        let document_selector = hookdata.document_selector();

        debug!("Running cron task: {job_name}");
        let collection = get_collection_by_name(&db, collection_name).await;
        if let Some(collection) = collection {
            let mut counter = cron_limit;
            let (total, mut items) = super::api::db::list_documents(
                &db,
                ListDocumentParams {
                    collection: collection.id,
                    exact_title: None,
                    oao_access: crate::api::db::CollectionDocumentVisibility::PublicAndUserIsReader,
                    extra_fields: vec!["title".to_string()],
                    sort_fields: None,
                    filters: vec![document_selector.clone().into()],
                    pagination: pagination.clone(),
                },
            )
            .await
            .unwrap_or_default();
            items.reverse();
            info!("{job_name} found {total} documents, processing up to {cron_limit}");
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
                let id = item["id"].as_str().unwrap_or("").to_string();
                let uuid = Uuid::parse_str(&id);
                if uuid.is_err() {
                    error!("{job_name} failed to parse id {id}");
                    continue;
                }
                let uuid = uuid.unwrap();
                debug!("Running cron task: {job_name} for document {id}");
                //     select document for update
                let loop_job_name = job_name.clone();
                let loop_collection_name = collection_name.to_string();
                let loop_data_service = data_service.clone();
                let loop_listener = listener.clone();
                let cr = db
                        .transaction::<_, CronResult, ApiErrors>(|txn| {
                            Box::pin(async move {
                                let document = select_document_for_update(uuid, txn).await?;
                                if document.is_none() {
                                    info!("Document vanished while running cron task: {loop_job_name} for document {id} in {loop_collection_name}");
                                    return Ok(CronResult { trigger_cron: false });
                                }
                                // TODO: Check document is still matching filters
                                // call cron handler
                                let document = document.unwrap();
                                let before_document: dto::CollectionDocument = (&document).into();
                                let after_document: dto::CollectionDocument = (&document).into();
                                let context = HookCronContext::new(before_document, after_document, loop_data_service);
                                let result = loop_listener.on_default_interval(&context).await?;
                                let trigger_cron = result.trigger_cron;
                                // modified document? update document and save events
                                check_modifications_and_update(txn, result).await?;
                                Ok(CronResult { trigger_cron })
                            })
                        })
                        .await;
                if let Ok(cr) = cr {
                    trigger_cron = cr.trigger_cron || trigger_cron;
                }
            }
        } else {
            error!("Could not find collection: {collection_name}");
        }
    }
    CronResult { trigger_cron }
}

pub(crate) fn setup_cron(
    db: sea_orm::DatabaseConnection,
    hooks: Arc<Hooks>,
    cron_interval: std::time::Duration,
    data_service: Arc<DataService>,
) -> (BackgroundTask, tokio::sync::mpsc::Sender<()>) {
    let mut interval = tokio::time::interval(cron_interval);
    debug!("cron_interval: {:?}", cron_interval);
    let (immediate_cron_signal, mut immediate_cron_recv) = mpsc::channel::<()>(1);
    let (shutdown_cron_signal, mut shutdown_cron_recv) = oneshot::channel::<()>();
    let loop_immediate_cron_signal = immediate_cron_signal.clone();
    let loop_data_service1 = data_service.clone();
    let loop_data_service2 = data_service.clone();
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
                    let r = cron(loopdb.clone(), &hooks, loop_data_service1.clone()).await;
                    if r.trigger_cron {
                        debug!("Triggering cron task");
                        let _ = loop_immediate_cron_signal.send(()).await;
                    }
                }
                _ = immediate_cron_recv.recv() => {
                    debug!("Immediate cron signal received");
                    let r = cron(loopdb.clone(), &hooks, loop_data_service2.clone()).await;
                    if r.trigger_cron {
                        debug!("Triggering cron task");
                        let _ = loop_immediate_cron_signal.send(()).await;
                    }
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
