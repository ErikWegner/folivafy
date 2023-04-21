use anyhow::Context;

use sea_orm::{ConnectOptions, Database};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use migration::{Migrator, MigratorTrait};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "folivafy=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::connect(
        ConnectOptions::from("postgresql://postgres:postgres@db/postgres")
            .max_connections(50)
            .to_owned(),
    )
    .await
    .context("could not connect to database_url")?;

    Migrator::up(&db, None).await?;

    folivafy::api::serve(db).await?;

    Ok(())
}
