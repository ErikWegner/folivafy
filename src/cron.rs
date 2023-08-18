use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

use crate::{
    api::{
        db::{get_collection_by_name, FieldFilter},
        hooks::Hooks,
        types::Pagination,
    },
    BackgroundTask,
};

async fn cron(db: sea_orm::DatabaseConnection, hooks: Hooks) {
    debug!("Running cron tasks");
    for (hookdata, _listener) in hooks.get_cron_hooks() {
        if let crate::api::hooks::HookData::CronDefaultIntervalHook {
            job_name,
            collection_name,
            document_selector,
        } = hookdata
        {
            debug!("Running cron task: {job_name}");
            let collection = get_collection_by_name(&db, &collection_name).await;
            if let Some(collection) = collection {
                let pagination = Pagination::new(100, 0);
                // list up to max 100 documents from the databasse
                let (_, items) = super::api::db::list_documents(
                    &db,
                    collection.id,
                    None,
                    crate::api::db::CollectionDocumentVisibility::PublicAndUserIsReader,
                    "".to_string(),
                    None,
                    vec![document_selector.into()],
                    &pagination,
                )
                .await
                .unwrap_or_default();
            // for each result-id:
            //     select document for update
            //     call cron handler
            //     modified document? update document
            //     events? save events
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
