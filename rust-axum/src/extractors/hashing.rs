/*!
Provides hashing validation for payload requests.
*/
use crate::{
  extractors::validator::{JsonValidationError, ValidatingJson},
  security::hashing::HashValidating,
  AppConfig,
};
use async_trait::async_trait;
use axum::{
  body::HttpBody,
  extract::{FromRequest, RequestParts},
  response::{IntoResponse, Response},
  BoxError, Json,
};
use http::StatusCode;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;
use user_persist::Validate;

/// An extractor that applies the following:
/// * Hashing validation
/// * Data validation
/// * Json deserialization
pub struct HashedValidatingJson<T: Validate + HashValidating>(pub T);

#[derive(Error, Debug)]
pub enum HashedValidatingError {
  #[error("Invalid JSON: {0}")]
  Json(#[from] JsonValidationError),
  #[error("Invalid Hash")]
  InvalidHash,
}

impl IntoResponse for HashedValidatingError {
  fn into_response(self) -> Response {
    let body = json!({
      "label": "json_parse.failed",
      "message": self.to_string()
    });
    match self {
      Self::InvalidHash => {
        (StatusCode::UNAUTHORIZED, Json(body)).into_response()
      }
      _ => (StatusCode::BAD_REQUEST, Json(body)).into_response(),
    }
  }
}

#[async_trait]
impl<B, T> FromRequest<B> for HashedValidatingJson<T>
where
  T: Validate + HashValidating,
  B: HttpBody + Send,
  B::Data: Send,
  B::Error: Into<BoxError>,
  T: Validate + DeserializeOwned,
{
  type Rejection = HashedValidatingError;

  async fn from_request(
    req: &mut RequestParts<B>,
  ) -> Result<Self, Self::Rejection> {
    let ValidatingJson(data): ValidatingJson<T> =
      ValidatingJson::from_request(req).await?;

    let prefix = req
      .extensions()
      .get::<Arc<AppConfig>>()
      .expect("No AppConfig. Did you forget to add Extension layer?")
      .hash_prefix();

    if data.is_valid(prefix) {
      Ok(Self(data))
    } else {
      Err(HashedValidatingError::InvalidHash)
    }
  }
}
