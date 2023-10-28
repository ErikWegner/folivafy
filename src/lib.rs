use anyhow::{anyhow, bail, Context, Result};
use api::hooks::{staged_delete::add_staged_delete_hook, Hooks};
use migration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::{debug, error};

pub mod api;
mod axumext;
pub mod cron;
mod mail;
mod monitoring;

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
        debug!("Shutting down background task: {}", self.name);
        if self.shutdown_signal.send(()).is_err() {
            error!("Failed to send shutdown signal to {} task", self.name);
        }
        if (self.join_handle.await).is_err() {
            error!("Failed to complete {} task", self.name);
        }
    }
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), anyhow::Error> {
    Migrator::up(db, None)
        .await
        .context("Database migration failed")
}

pub async fn danger_drop_database_tables(db: &DatabaseConnection) -> Result<(), anyhow::Error> {
    Migrator::down(db, Some(1))
        .await
        .context("Database migration failed #1")?;
    Migrator::down(db, Some(1))
        .await
        .context("Database migration failed #2")
}

pub fn register_staged_delete_handler(mut hooks: Hooks) -> Result<Hooks, anyhow::Error> {
    debug!("register_staged_delete_handler");
    let rv = std::env::var("FOLIVAFY_ENABLE_DELETION");
    if let Ok(v) = rv {
        let v = v.trim();
        if !v.is_empty() {
            let v: Vec<&str> = v
                .strip_prefix('(')
                .ok_or_else(|| {
                    anyhow!("FOLIVAFY_ENABLE_DELETION must start with an opening parenthesis.")
                })?
                .strip_suffix(')')
                .ok_or_else(|| {
                    anyhow!("FOLIVAFY_ENABLE_DELETION must end with a closing parenthesis.")
                })?
                .split("),(")
                .collect();
            for s in v {
                debug!("Processing {s}");
                let p: Vec<&str> = s.split(',').collect();
                if p.len() != 3 {
                    bail!("Invalid value {s} inside FOLIVAFY_ENABLE_DELETION");
                }
                let collection_name = p[0];
                let days_stage_1: u16 = p[1]
                    .parse()
                    .map_err(|s| anyhow!("Invalid 1st number for {collection_name}: {s}"))?;
                let days_stage_2: u16 = p[2]
                    .parse()
                    .map_err(|s| anyhow!("Invalid 2nd number for {collection_name}: {s}"))?;
                add_staged_delete_hook(&mut hooks, collection_name, days_stage_1, days_stage_2);
            }
        }
    }

    Ok(hooks)
}
