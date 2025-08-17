use crate::common::test_router::SECRET;
use axum::{body::Body, http::Response};
use chrono::{Duration, Utc};
use http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, Request,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use rust_axum::types::jwt::{JWTClaims, Role};
use serde::Deserialize;
use tracing::debug;

pub mod test_database;
pub mod test_router;

/// JSON mime type.
pub(crate) const MIME_JSON: &str = "application/json";

/// Add an authorization header token value for given role.
pub fn add_jwt(role: Role) -> String {
    format!("Bearer {}", test_jwt(role))
}

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

/// Creates a test JWT for the given role.
fn test_jwt(role: Role) -> String {
    let expiration = Utc::now() + Duration::minutes(25);
    let test_claims = JWTClaims {
        sub: "droberts".to_owned(),
        role,
        exp: expiration.timestamp(),
    };
    encode(
        &Header::default(),
        &test_claims,
        &EncodingKey::from_secret(SECRET),
    )
    .unwrap()
}

/// Common test get request.
#[allow(dead_code)]
pub(crate) fn get(uri: &str, role: Role) -> Result<Request<Body>, http::Error> {
    Request::builder()
        .uri(uri)
        .header(AUTHORIZATION, add_jwt(role))
        .body(Body::empty())
}

/// Common test post request.
#[allow(dead_code)]
pub(crate) fn post(
    uri: &str,
    role: Role,
    body: impl Into<Body>,
) -> Result<Request<Body>, http::Error> {
    Request::builder()
        .uri(uri)
        .method(Method::POST)
        .header(CONTENT_TYPE, MIME_JSON)
        .header(AUTHORIZATION, add_jwt(role))
        .body(body.into())
}

/// Common test put request.
#[allow(dead_code)]
pub(crate) fn put(
    uri: &str,
    role: Role,
    body: impl Into<Body>,
) -> Result<Request<Body>, http::Error> {
    Request::builder()
        .uri(uri)
        .method(Method::PUT)
        .header(CONTENT_TYPE, MIME_JSON)
        .header(AUTHORIZATION, add_jwt(role))
        .body(body.into())
}
