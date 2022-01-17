use async_trait::async_trait;
use axum::{
  body::{boxed, Full, HttpBody},
  extract::{rejection::JsonRejection, FromRequest, Json, RequestParts},
  response::{IntoResponse, Response},
  BoxError,
};
use http::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, to_string};
use std::ops::Deref;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatingJson<T: Validate>(pub T);

#[derive(Debug, Error)]
pub enum JsonValidationError {
  #[error("Json validation error")]
  JsonError(#[from] JsonRejection),
  #[error("Validation failed")]
  JsonValidation(#[from] ValidationErrors),
}

#[derive(Debug, Serialize)]
struct ValidationErrorResponse {
  validation_errors: ValidationErrors,
  label: String,
}

/// Uses a Json extractor and adds validation
/// to the extracted type via the Validate trait.
#[async_trait]
impl<B, T> FromRequest<B> for ValidatingJson<T>
where
  B: HttpBody + Send,
  B::Data: Send,
  B::Error: Into<BoxError>,
  T: Validate + DeserializeOwned,
{
  type Rejection = JsonValidationError;

  async fn from_request(
    req: &mut RequestParts<B>,
  ) -> Result<Self, Self::Rejection> {
    let data: Json<T> = Json::from_request(req).await?;
    data.validate()?;
    Ok(Self(data.0))
  }
}

impl IntoResponse for JsonValidationError {
  fn into_response(self) -> Response {
    let body = match self {
      Self::JsonError(e) => {
        let response = json!({
          "label": "json_parse.failed",
          "message": e.to_string()
        });
        boxed(Full::from(to_string(&response).unwrap_or_default()))
      }
      Self::JsonValidation(e) => {
        let validation_response = ValidationErrorResponse {
          validation_errors: e,
          label: "validation.failed".to_owned(),
        };
        boxed(Full::from(
          to_string(&validation_response).unwrap_or_default(),
        ))
      }
    };

    Response::builder()
      .status(StatusCode::UNPROCESSABLE_ENTITY)
      .header("Content-Type", "application/json")
      .body(body)
      .unwrap()
  }
}

impl<T: Validate> Deref for ValidatingJson<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T: Validate> From<T> for ValidatingJson<T> {
  fn from(inner: T) -> Self {
    Self(inner)
  }
}
