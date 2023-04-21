use std::ops::Deref;

use axum::{
    async_trait,
    extract::{rejection::QueryRejection, FromRequestParts, Query},
    http::request::Parts,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum::{Json, RequestPartsExt};
use serde::de::DeserializeOwned;
use serde_json::json;
use validator::Validate;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ValidatedQueryParams<T>(pub T);

fn map_rejection(err: QueryRejection) -> Response {
    match err {
        QueryRejection::FailedToDeserializeQueryString(inner) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": format!("Failed to parse query string: {}", inner)
            })),
        )
            .into_response(),
        _ => todo!(),
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedQueryParams<T>
where
    T: DeserializeOwned + Validate + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = parts.extract::<Query<T>>().await.map_err(map_rejection)?;
        let validate_result = query.validate();
        if let Err(err) = validate_result {
            Err((StatusCode::BAD_REQUEST, Json(err)).into_response())
        } else {
            Ok(ValidatedQueryParams(query))
        }
    }
}

impl<T> Deref for ValidatedQueryParams<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
