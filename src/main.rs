use std::env;

use anyhow::Context;

use dotenvy::dotenv;
use folivafy::{api::hooks::Hooks, migrate};
use sea_orm::{ConnectOptions, Database};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    migrate(&db).await?;

    let cron_interval = std::time::Duration::from_secs(
        60 * std::cmp::max(
            1,
            env::var("FOLIVAFY_CRON_INTERVAL")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .with_context(|| "could not parse FOLIVAFY_CRON_INTERVAL")?,
        ),
    );
    folivafy::api::serve(db, Hooks::new(), cron_interval).await?;

    Ok(())
}
