/*!
Types for handler functions.
*/
use crate::USER_MS_TARGET;
use axum::{
    extract::Extension,
    response::{IntoResponse, Response},
    Json,
};
use http::StatusCode;
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;
use tracing::{event, Level};
use user_persist::persistence::PersistenceError;

/// Common error type for handlers.
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Persistence error: `{0}`")]
    PersistenceError(#[from] PersistenceError),
    #[error("Resource not found")]
    ResourceNotFound,
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        let error_message = format!("{self}");

        event!(
          target: USER_MS_TARGET,
          Level::ERROR,
          "Server error: {error_message}"
        );

        let body = json!({
          "label": "server.error",
          "message": error_message
        });

        (
            match self {
                Self::ResourceNotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Json(body),
        )
            .into_response()
    }
}

/// Type alias for UserPersistence Trait object.
pub type Persist<T> = Extension<Arc<T>>;
