use anyhow::Context;
use migration::{Migrator, MigratorTrait};
use sea_orm::DatabaseConnection;

pub mod api;
mod axumext;

pub async fn migrate(db: &DatabaseConnection) -> Result<(), anyhow::Error> {
    Migrator::up(db, None)
        .await
        .context("Database migration failed")
}
