use anyhow::Result;
use lazy_static::lazy_static;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::sync::Arc;
use tokio::sync::{oneshot, watch};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::api::data_service::DataService;
use crate::api::db::list_documents;
use crate::api::db::ListDocumentGrants::IgnoredForCron;
use crate::api::dto::GrantForDocument;
use crate::api::grants::{hook_or_default_document_grants, GrantCollection};
use crate::{
    api::{
        data_service::FolivafyDataService,
        db::{
            get_collection_by_name, save_document_events_mails, DbGrantUpdate, DbListDocumentParams,
        },
        dto,
        hooks::{HookCronContext, HookSuccessResult, Hooks},
        select_document_for_update,
        types::Pagination,
        ApiErrors,
    },
    BackgroundTask,
};

pub(crate) type ImmediateCronSender = watch::Sender<()>;

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
    data_service: Arc<FolivafyDataService>,
) -> CronResult {
    debug!("Running cron tasks");
    let mut trigger_cron = false;
    let cron_limit = 100;
    let pagination = Pagination::new(cron_limit, 0);
    let l = hooks.get_cron_default_interval_hooks();
    let tasks_count = l.len();
    let mut task_counter = 0;
    for (hookdata, listener) in l {
        let job_name = hookdata.job_name().to_string();
        let collection_name = hookdata.collection_name();
        let document_selector = hookdata.document_selector();

        task_counter += 1;
        debug!("Running cron task: {job_name} ({task_counter}/{tasks_count})");
        let collection = get_collection_by_name(&db, collection_name).await;
        if collection.is_none() {
            error!("Could not find collection: {collection_name}");
            continue;
        }
        let collection = collection.unwrap();
        let mut counter = cron_limit;
        let dbparams = DbListDocumentParams::builder()
            .collection(collection.id)
            .grants(IgnoredForCron)
            .extra_fields(vec!["title".to_string()])
            .sort_fields(None)
            .filters(vec![document_selector.clone().into()].into())
            .pagination(pagination.clone())
            .include_author_id(false)
            .build();
        let (total, mut items) = list_documents(&db, &dbparams).await.unwrap_or_default();
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
            let dto_collection =
                GrantCollection::new(collection.name.clone(), collection.id, collection.oao);
            let trx_hooks = hooks.clone();
            let trx_id = id.clone();
            let cr = db
                        .transaction::<_, CronResult, ApiErrors>(|txn| {
                            Box::pin(async move {
                                let document = select_document_for_update(uuid, txn).await?;
                                if document.is_none() {
                                    info!("Document vanished while running cron task: {loop_job_name} for document {trx_id} in {loop_collection_name}");
                                    return Ok(CronResult { trigger_cron: false });
                                }
                                // TODO: Check document is still matching filters
                                // call cron handler
                                let document = document.unwrap();
                                let before_document: dto::CollectionDocument = (&document).into();
                                let after_document: dto::CollectionDocument = (&document).into();
                                let context = HookCronContext::new(before_document, after_document, loop_data_service.clone());
                                let result = loop_listener.on_default_interval(&context).await?;
                                let trigger_cron = result.trigger_cron;
                                // modified document? update document and save events
                                check_modifications_and_update(
                                    txn,
                                    result,
                                    &trx_hooks,
                                    dto_collection,
                                    (&document).into(),
                                    loop_data_service,
                                    document.owner
                                ).await?;
                                Ok(CronResult { trigger_cron })
                            })
                        })
                        .await;
            if let Ok(cr) = cr {
                debug!("OK for cron task: {job_name} for document {id}");
                trigger_cron = cr.trigger_cron || trigger_cron;
            } else {
                error!("Failed for cron task: {job_name} for document {id}");
            }
        }
    }
    CronResult { trigger_cron }
}

pub(crate) fn setup_cron(
    db: sea_orm::DatabaseConnection,
    hooks: Arc<Hooks>,
    cron_interval: std::time::Duration,
    data_service: Arc<FolivafyDataService>,
) -> (BackgroundTask, ImmediateCronSender) {
    let mut interval = tokio::time::interval(cron_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    debug!("cron_interval: {:?}", cron_interval);
    let (immediate_cron_signal, mut immediate_cron_recv) = watch::channel(());
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
                        let _ = loop_immediate_cron_signal.send(());
                    }
                }
                _ = immediate_cron_recv.changed() => {
                    debug!("Immediate cron signal received");
                    let r = cron(loopdb.clone(), &hooks, loop_data_service2.clone()).await;
                    if r.trigger_cron {
                        debug!("Triggering cron task");
                        let _ = loop_immediate_cron_signal.send(());
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

async fn check_modifications_and_update(
    txn: &DatabaseTransaction,
    result: HookSuccessResult,
    hooks: &Hooks,
    collection: GrantCollection,
    cron_base_document: dto::CollectionDocument,
    data_service: std::sync::Arc<dyn DataService>,
    author_id: Uuid,
) -> Result<(), ApiErrors> {
    let mut document = None;
    match result.document {
        crate::api::hooks::DocumentResult::Store(new_document) => document = Some(new_document),
        crate::api::hooks::DocumentResult::NoUpdate => {}
        crate::api::hooks::DocumentResult::Err(e) => return Err(e),
    }
    let dbgrants = match result.grants {
        crate::api::hooks::GrantSettings::Default => {
            // Default:
            let grants = hook_or_default_document_grants(
                hooks,
                collection,
                cron_base_document.clone(),
                data_service,
                author_id,
            )
            .await?;
            let grants_for_document = grants
                .iter()
                .map(|g| GrantForDocument::new(g.clone(), *cron_base_document.id()))
                .collect();
            DbGrantUpdate::Replace(grants_for_document)
        }
        crate::api::hooks::GrantSettings::NoChange => DbGrantUpdate::Keep,
        crate::api::hooks::GrantSettings::Replace(grants) => DbGrantUpdate::Replace(grants),
    };
    let cron_user = dto::User::new(*CRON_USER_ID, CRON_USER_NAME.to_string());
    save_document_events_mails(
        txn,
        &cron_user,
        document,
        None,
        result.events,
        dbgrants,
        result.mails,
    )
    .await
    .map_err(|e| {
        error!("Update document error: {:?}", e);
        ApiErrors::InternalServerError
    })?;
    Ok(())
}
