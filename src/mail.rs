use tokio::sync::{mpsc, oneshot};
use tracing::debug;

use crate::{
    api::hooks::{self, Hooks},
    BackgroundTask,
};

pub(crate) fn insert_mail_cron_hook(hooks: &mut Hooks) -> BackgroundTask {
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
        "folivafy-mail",
        "folivafy-mail",
        hooks::CronDocumentSelector::ByFieldEqualsValue {
            field: "status".to_string(),
            value: "pending".to_string(),
        },
        tx,
    );
    BackgroundTask::new("mail", join_handle, shutdown_mail_signal)
}
