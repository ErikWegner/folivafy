mod list_collections;
mod types;

use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use anyhow::Context;
use axum::{
    body::Bytes,
    http::{HeaderMap, Request},
    response::Response,
    routing::get,
    Router,
};
use sea_orm::DatabaseConnection;
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::Span;

use self::list_collections::api_list_collections;

#[derive(Clone)]
pub(crate) struct ApiContext {
    db: DatabaseConnection,
}

pub async fn serve(db: DatabaseConnection) -> anyhow::Result<()> {
    // build our application with a route
    let app = api_routes(db)
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

pub fn api_routes(db: DatabaseConnection) -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/collections", get(api_list_collections))
            .with_state(ApiContext { db }),
    )
}
