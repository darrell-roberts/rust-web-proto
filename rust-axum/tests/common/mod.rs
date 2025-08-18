//! Re-usable functions for integration tests.
use axum::{body::Body, http::Response};
use serde::Deserialize;
use tracing::debug;

pub mod test_database;
pub mod test_router;

/// JSON mime type.
pub(crate) const MIME_JSON: &str = "application/json";

/// Deserialize the response body into T.
pub async fn body_as<T>(response: Response<Body>) -> T
where
    T: for<'de> Deserialize<'de>,
{
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|b| serde_json::from_slice::<T>(&b).unwrap())
        .unwrap()
}

/// Consume the body and return it as a String.
#[allow(dead_code)]
pub async fn body_as_str(response: Response<Body>) -> String {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|b| String::from_utf8(b.to_vec()).unwrap())
        .unwrap()
}

/// Consume and print the body.
#[allow(dead_code)]
pub async fn dump_result(response: Response<Body>) {
    let body = body_as_str(response).await;
    debug!("result: {body}");
}
