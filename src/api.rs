mod auth;
mod create_collection;
mod create_document;
mod create_event;
mod db;
pub mod dto;
mod get_document;
pub mod hooks;
mod list_collections;
mod list_documents;
mod types;
mod update_document;
pub use entity::collection::Model as Collection;
use entity::collection_document::Entity as Documents;
pub use openapi::models::CollectionItem;
use tokio::signal;

use std::{
    env,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use anyhow::Context;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use jwt_authorizer::{JwtAuthorizer, Validation};
use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait};
use serde::Serialize;
use thiserror::Error;
use tower_http::trace::TraceLayer;
use tracing::error;

use self::{
    auth::{cert_loader, User},
    create_collection::api_create_collection,
    create_document::api_create_document,
    create_event::api_create_event,
    get_document::api_read_document,
    hooks::Hooks,
    list_collections::api_list_collections,
    list_documents::api_list_document,
    update_document::api_update_document,
};

pub static CATEGORY_DOCUMENT_UPDATES: i32 = 1;

#[derive(Clone)]
pub(crate) struct ApiContext {
    db: DatabaseConnection,
    hooks: Hooks,
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum ApiErrors {
    #[error("Internal server error")]
    InternalServerError,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized")]
    PermissionDenied,
}

impl IntoResponse for ApiErrors {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiErrors::PermissionDenied => (
                StatusCode::UNAUTHORIZED,
                ApiErrors::PermissionDenied.to_string(),
            ),
            ApiErrors::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            ),
            ApiErrors::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiErrors::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        }
        .into_response()
    }
}

impl From<DbErr> for ApiErrors {
    fn from(value: DbErr) -> Self {
        match value {
            DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error)) => match error {
                sqlx::error::Error::Database(e) => {
                    let code: String = e.code().unwrap_or_default().to_string();

                    error!("Database runtime error: {}", e);
                    ApiErrors::BadRequest(format!("Cannot append event, code {})", code))
                }
                _ => {
                    error!("Database runtime error: {}", error);
                    ApiErrors::InternalServerError
                }
            },
            DbErr::RecordNotFound(t) => ApiErrors::NotFound(t),
            _ => {
                error!("Database error: {:?}", value);
                ApiErrors::InternalServerError
            }
        }
    }
}

#[derive(Serialize, Debug)]
struct ValidationErrors {
    errors: Vec<String>,
}

impl From<validator::ValidationErrors> for ApiErrors {
    fn from(err: validator::ValidationErrors) -> Self {
        ApiErrors::BadRequest(serde_json::to_string(&err).unwrap_or("Validation error".to_owned()))
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}

pub async fn serve(
    db: DatabaseConnection,
    hooks: Hooks,
    cron_interval: std::time::Duration,
) -> anyhow::Result<()> {
    let (requesthooks, cronhooks) = hooks.split_cron_hooks();
    let (join_handle, _immediate_cron_signal, shutdown_cron_signal) =
        crate::cron::setup_cron(db.clone(), cronhooks, cron_interval);
    // build our application with a route
    let app = api_routes(db, requesthooks)
        .await?
        // `TraceLayer` is provided by tower-http so you have to add that as a dependency.
        // It provides good defaults but is also very customizable.
        //
        // See https://docs.rs/tower-http/0.1.1/tower_http/trace/index.html for more details.
        .layer(TraceLayer::new_for_http());

    // run it
    let addr = SocketAddr::new(
        IpAddr::from_str("::")?,
        std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .context("Cannot parse PORT")?,
    );

    tracing::info!("listening on {}", addr);
    axum::Server::try_bind(&addr)
        .context("Cannot start server")?
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("error running server")?;
    shutdown_cron_signal
        .send(())
        .map_err(|_| anyhow::anyhow!("Failed to send cron shutdown signal"))?;
    join_handle
        .await
        .with_context(|| "Failed to complete cron background tasks".to_string())
}

async fn api_routes(db: DatabaseConnection, hooks: Hooks) -> anyhow::Result<Router> {
    let issuer = env::var("FOLIVAFY_JWT_ISSUER").context("FOLIVAFY_JWT_ISSUER is not set")?;
    let danger_accept_invalid_certs = env::var("FOLIVAFY_DANGEROUS_ACCEPT_INVALID_CERTS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    let pem_text = cert_loader(&issuer, danger_accept_invalid_certs).await?;
    let validation = Validation::new().iss(&[issuer]).leeway(5);
    let jwt_auth: JwtAuthorizer<User> =
        JwtAuthorizer::from_rsa_pem_text(pem_text.as_str()).validation(validation);

    Ok(Router::new().nest(
        "/api",
        Router::new()
            .route(
                "/collections",
                get(api_list_collections).post(api_create_collection),
            )
            .route(
                "/collections/:collection_name",
                get(api_list_document)
                    .post(api_create_document)
                    .put(api_update_document),
            )
            .route(
                "/collections/:collection_name/:document_id",
                get(api_read_document),
            )
            .route("/events", post(api_create_event))
            .with_state(ApiContext { db, hooks })
            .layer(jwt_auth.layer().await.unwrap()),
    ))
}

pub(crate) async fn select_document_for_update(
    unchecked_document_id: uuid::Uuid,
    txn: &DatabaseTransaction,
) -> Result<Option<entity::collection_document::Model>, DbErr> {
    Documents::find()
        .from_raw_sql(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            r#"SELECT * FROM "collection_document" WHERE "id" = $1 FOR UPDATE"#,
            [unchecked_document_id.into()],
        ))
        .one(txn)
        .await
}
