use axum::{body::Body, http::Response, Router};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rust_axum::{
    arguments::AppConfig,
    build_app,
    types::jwt::{JWTClaims, Role},
};
use serde::Deserialize;
use std::sync::{Arc, Once};
use test_persist::TestPersistence;
use tracing::debug;
use tracing_subscriber::EnvFilter;

pub mod test_persist;

static INIT: Once = Once::new();
pub const TEST_TARGET: &str = "test";
pub const MIME_JSON: &str = "application/json";

// Setup tracing first.
fn init_log() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_target(true)
            .init();
    });
}

static SECRET: &[u8] = "TEST_SECRET".as_bytes();

/// Build test Router.
pub fn app(persistence: Option<Arc<TestPersistence>>) -> Router {
    init_log();
    let persist = match persistence {
        Some(p) => p,
        None => Arc::new(TestPersistence::new()),
    };
    build_app(persist, AppConfig::new(SECRET))
}

/// Add an authorization header token value for given role.
pub fn add_jwt(role: Role) -> String {
    format!("Bearer {}", test_jwt(role))
}

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
    debug!(target: TEST_TARGET, "result: {body}");
}

/// Creates a test JWT for the given role.
pub fn test_jwt(role: Role) -> String {
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
