use std::env;

use anyhow::Context;

use dotenvy::dotenv;
use sea_orm::{ConnectOptions, Database};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use migration::{Migrator, MigratorTrait};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "folivafy=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Database::connect(
        ConnectOptions::from(env::var("FOLIVAFY_DATABASE").context("FOLIVAFY_DATABASE not set")?)
            .max_connections(50)
            .to_owned(),
    )
    .await
    .context("could not connect to database_url")?;

    Migrator::up(&db, None).await?;

    folivafy::api::serve(db).await?;

    Ok(())
}
