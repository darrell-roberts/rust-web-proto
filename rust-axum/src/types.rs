use crate::USER_MS_TARGET;
use axum::body::{boxed, Full};
use axum::extract::{Extension};
use axum::response::{self, IntoResponse};
use http::StatusCode;
use serde_json::{json, to_string};
use std::sync::Arc;
use thiserror::Error;
use tracing::{event, Level};
use user_persist::persistence::{PersistenceError, UserPersistence};
use serde::{Serialize, Deserialize};

#[derive(Debug, Error)]
pub enum HandlerError {
  #[error("Persistence error")]
  PersistenceError(#[from] PersistenceError),
}

impl IntoResponse for HandlerError {
  fn into_response(self) -> response::Response {
    let error_message = match self {
      Self::PersistenceError(e) => match e {
        PersistenceError::MongoError(e) => e.to_string(),
        _ => "Server error".to_owned(),
      },
    };

    event!(
      target: USER_MS_TARGET,
      Level::ERROR,
      "Server error: {error_message}"
    );

    let body = json!({
      "label": "server.error",
      "message": error_message
    });
    let resp = to_string(&body).unwrap_or_default();
    response::Response::builder()
      .status(StatusCode::INTERNAL_SERVER_ERROR)
      .header("Content-Type", "application/json")
      .body(boxed(Full::from(resp)))
      .unwrap()
  }
}

pub type Persist = Extension<Arc<dyn UserPersistence>>;

//// JWT

/// Type for claims in the JWT token used for
/// authorizing requests.
#[derive(Deserialize, Serialize, Debug)]
pub struct JWTClaims {
  /// Subjet. This is the user identifiier.
  pub sub: String,
  // Roles for the subject.
  pub role: Role,
  /// Expiration date time in unix epoch.
  pub exp: i64,
}

/// Enumeration of Roles
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum Role {
  Admin,
  User,
}

/// JWT Claims when the role is User
#[derive(Debug)]
pub struct UserAccess(pub JWTClaims);

/// JWT Claims when the role is Admin
#[derive(Debug)]
pub struct AdminAccess(pub JWTClaims);
