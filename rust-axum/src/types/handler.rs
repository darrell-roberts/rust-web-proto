//! Types for handler functions.
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
use user_database::database::DatabaseError;

/// Common error type for handlers.
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Database error: `{0}`")]
    DatabaseError(#[from] DatabaseError),
    #[error("Resource not found")]
    ResourceNotFound,
    #[error("Http response error: {0}")]
    Http(#[from] http::Error),
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> Response {
        let error_message = format!("{self}");

        event!(Level::ERROR, "Server error: {error_message}");

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

/// Type alias for generic database.
pub type Database<T> = Extension<Arc<T>>;
