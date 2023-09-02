use axum::{routing::get, Router};

pub struct HealthMonitor {}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn is_healthy(&self) -> bool {
        true
    }
}

pub(crate) fn health_routes(monitor: std::sync::Arc<HealthMonitor>) -> Router {
    Router::new().route("/up", get(|| async { "OK" })).route(
        "/health",
        get(move || async move {
            if monitor.is_healthy() {
                (axum::http::StatusCode::OK, "OK")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "UNHEALTHY")
            }
        }),
    )
}
