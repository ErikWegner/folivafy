use tokio::sync::{mpsc, oneshot};
use tracing::debug;

use crate::{api::hooks::Hooks, BackgroundTask};

async fn cron(_db: sea_orm::DatabaseConnection, hooks: Hooks) {
    debug!("Running cron tasks");
    for (hookdata, _listener) in hooks.get_cron_hooks() {
        debug!("Running cron task: {:?}", hookdata);
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
