use axum::{
  body::{BoxBody, HttpBody},
  http::Response,
  Router,
};
use rust_axum::{
  arguments::{test_jwt, AppConfig},
  build_app,
  types::jwt::Role,
};
use serde::Deserialize;
use std::{
  fmt::Debug,
  sync::{Arc, Once},
};
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
  build_app(persist, AppConfig::test(SECRET))
}

/// Add an authorization header token value for given role.
pub fn add_jwt(role: Role) -> String {
  format!("Bearer {}", test_jwt(&AppConfig::test(SECRET), role))
}

pub async fn body_as<T>(response: Response<BoxBody>) -> T
where
  T: for<'de> Deserialize<'de>,
{
  hyper::body::to_bytes(response.into_body())
    .await
    .map(|b| serde_json::from_slice::<T>(&b.to_vec()).unwrap())
    .unwrap()
}

/// Consume the body and return it as a String.
pub async fn body_as_str<T>(response: Response<T>) -> String
where
  T: HttpBody,
  T::Error: Debug,
{
  hyper::body::to_bytes(response.into_body())
    .await
    .map(|b| String::from_utf8(b.to_vec()).unwrap())
    .unwrap()
}

/// Consume and print the body.
pub async fn dump_result<T>(response: Response<T>)
where
  T: HttpBody,
  T::Error: Debug,
{
  let body = body_as_str(response).await;
  debug!(target: TEST_TARGET, "result: {body}");
}
