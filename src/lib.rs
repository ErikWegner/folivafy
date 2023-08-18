use anyhow::{Context, Result};
use migration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::error;

pub mod api;
mod axumext;
pub mod cron;
mod mail;

pub(crate) struct BackgroundTask {
    name: String,
    join_handle: JoinHandle<()>,
    shutdown_signal: oneshot::Sender<()>,
}

impl BackgroundTask {
    pub(crate) fn new(
        name: &str,
        join_handle: JoinHandle<()>,
        shutdown_signal: oneshot::Sender<()>,
    ) -> Self {
        Self {
            name: name.to_string(),
            join_handle,
            shutdown_signal,
        }
    }

    async fn shutdown(self) {
        let _ = self.shutdown_signal.send(()).or_else(|_| {
            error!("Failed to send shutdown signal to {} task", self.name);
            Err(())
        });
        let _ = self.join_handle.await.or_else(|_| {
            error!("Failed to complete {} task", self.name);
            Err(())
        });
    }
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), anyhow::Error> {
    Migrator::up(db, None)
        .await
        .context("Database migration failed")
}

pub async fn drop(db: &DatabaseConnection) -> Result<(), anyhow::Error> {
    Migrator::down(db, Some(1))
        .await
        .context("Database migration failed #1")?;
    Migrator::down(db, Some(1))
        .await
        .context("Database migration failed #2")
}
