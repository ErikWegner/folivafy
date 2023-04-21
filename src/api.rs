mod list_collections;
mod types;

use axum::{routing::get, Router};
use sea_orm::DatabaseConnection;

use self::list_collections::api_list_collections;

#[derive(Clone)]
pub(crate) struct ApiContext {
    db: DatabaseConnection,
}

pub async fn serve(_db: DatabaseConnection) -> anyhow::Result<()> {
    Ok(())
}

pub fn api_routes(db: DatabaseConnection) -> Router {
    Router::new()
        .route("/collections", get(api_list_collections))
        .with_state(ApiContext { db })
}
