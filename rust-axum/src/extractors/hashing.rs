/*!
Provides hashing validation for payload requests.
*/
use crate::{
    extractors::validator::{JsonValidationError, ValidatingJson},
    security::hashing::HashValidating,
    AppConfig,
};
use axum::{
    body::Body,
    extract::FromRequest,
    response::{IntoResponse, Response},
    Json,
};
use http::{Request, StatusCode};
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
            Self::InvalidHash => (StatusCode::UNAUTHORIZED, Json(body)).into_response(),
            _ => (StatusCode::BAD_REQUEST, Json(body)).into_response(),
        }
    }
}

impl<S, T> FromRequest<S> for HashedValidatingJson<T>
where
    T: Validate + HashValidating + DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = HashedValidatingError;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let config = req
            .extensions()
            .get::<Arc<AppConfig>>()
            .expect("No AppConfig. Did you forget to add Extension layer?")
            .clone();

        let ValidatingJson(data): ValidatingJson<T> =
            ValidatingJson::from_request(req, state).await?;

        if data.is_valid(config.hash_prefix()) {
            Ok(Self(data))
        } else {
            Err(HashedValidatingError::InvalidHash)
        }
    }
}
