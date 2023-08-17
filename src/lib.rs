use anyhow::Context;
use migration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;

pub mod api;
mod axumext;
pub mod cron;

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
