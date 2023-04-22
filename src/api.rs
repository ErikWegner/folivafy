mod auth;
mod create_collection;
mod list_collections;
mod types;

use std::{
    env,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use anyhow::Context;
use axum::{
    body::Bytes,
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use jwt_authorizer::{JwtAuthorizer, Validation};
use sea_orm::{DatabaseConnection, DbErr};
use thiserror::Error;
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{error, Span};

use self::{
    auth::{cert_loader, User},
    create_collection::api_create_collection,
    list_collections::api_list_collections,
};

#[derive(Clone)]
pub(crate) struct ApiContext {
    db: DatabaseConnection,
}

#[derive(Error, Debug)]
pub(crate) enum ApiErrors {
    #[error("Internal server error")]
    InternalServerError,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl IntoResponse for ApiErrors {
    fn into_response(self) -> axum::response::Response {
        match self {
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
            DbErr::RecordNotFound(t) => ApiErrors::NotFound(t),
            _ => {
                error!("Database error: {:?}", value);
                ApiErrors::InternalServerError
            }
        }
    }
}

pub async fn serve(db: DatabaseConnection) -> anyhow::Result<()> {
    // build our application with a route
    let app = api_routes(db)
        .await?
        // `TraceLayer` is provided by tower-http so you have to add that as a dependency.
        // It provides good defaults but is also very customizable.
        //
        // See https://docs.rs/tower-http/0.1.1/tower_http/trace/index.html for more details.
        .layer(TraceLayer::new_for_http())
        // If you want to customize the behavior using closures here is how
        //
        // This is just for demonstration, you don't need to add this middleware twice
        .layer(
            TraceLayer::new_for_http()
                .on_request(|_request: &Request<_>, _span: &Span| {
                    // ...
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                    // ..
                })
                .on_eos(
                    |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    },
                )
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        // ...
                    },
                ),
        );

    // run it
    let addr = SocketAddr::new(IpAddr::from_str("::")?, 3000);
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("error running server")
}

async fn api_routes(db: DatabaseConnection) -> anyhow::Result<Router> {
    let issuer = env::var("FOLIVAFY_JWT_ISSUER").context("FOLIVAFY_JWT_ISSUER is not set")?;

    let pem_text = cert_loader(&issuer).await?;
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
            .with_state(ApiContext { db })
            .layer(jwt_auth.layer().await.unwrap()),
    ))
}
