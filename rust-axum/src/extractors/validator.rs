//! Axum extractor that calls validate on request types that
//! have validation annotations.
use axum::{
    body::Body,
    extract::{rejection::JsonRejection, FromRequest, Json},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, to_value};
use std::ops::Deref;
use thiserror::Error;
use tracing::error;
use user_database::{Validate, ValidationErrors};

/// An extractor that adds value validators to a Json validator.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatingJson<T: Validate>(pub T);

#[derive(Debug, Error)]
pub enum JsonValidationError {
    #[error("Json validation error: `{0}`")]
    JsonError(#[from] JsonRejection),
    #[error("Validation failed: `{0}`")]
    JsonValidation(#[from] ValidationErrors),
}

/// Validation errors for all validations that failed.
#[derive(Debug, Serialize)]
struct ValidationErrorResponse {
    validation_errors: ValidationErrors,
    label: String,
}

/// Uses a Json extractor that adds validation
/// to the extracted type via the Validate trait.
impl<S, T> FromRequest<S> for ValidatingJson<T>
where
    T: Validate + DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = JsonValidationError;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(data): Json<T> = Json::from_request(req, state).await?;
        data.validate()?;
        Ok(Self(data))
    }
}

impl IntoResponse for JsonValidationError {
    fn into_response(self) -> Response {
        error!("Input failed validation: {self}");

        let body = match self {
            Self::JsonError(e) => {
                json!({
                  "label": "json_parse.failed",
                  "message": e.to_string()
                })
            }
            Self::JsonValidation(e) => {
                let validation_response = ValidationErrorResponse {
                    validation_errors: e,
                    label: "validation.failed".to_owned(),
                };
                to_value(&validation_response).unwrap_or_else(|e| json!({"error": e.to_string()}))
            }
        };
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
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
