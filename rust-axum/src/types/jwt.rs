/*!
JWT types and trait implementations.
*/
use crate::USER_MS_TARGET;
use axum::response::{IntoResponse, Json, Response};
use chrono::{DateTime, NaiveDateTime, Utc};
use http::StatusCode;
use jsonwebtoken::DecodingKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
  convert::Infallible,
  fmt::{self, Display, Formatter},
  ops::Deref,
  str::FromStr,
};
use thiserror::Error;
use tracing::{event, Level};

/// Type for claims in the JWT token used for
/// authorizing requests.
#[derive(Deserialize, Serialize, Debug)]
pub struct JWTClaims {
  /// Subject. This is the user identifier.
  pub sub: String,
  // Roles for the subject.
  pub role: Role,
  /// Expiration date time in unix epoch.
  pub exp: i64,
}

impl Display for JWTClaims {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let expire = DateTime::<Utc>::from_utc(
      NaiveDateTime::from_timestamp_opt(self.exp, 0).ok_or(fmt::Error)?,
      Utc,
    );
    write!(f, "sub: {}, role: {}, exp: {}", self.sub, self.role, expire)
  }
}

/// Sum Type for Roles
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum Role {
  Admin,
  User,
}

impl Display for Role {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Role::Admin => "Admin",
        Role::User => "User",
      }
    )
  }
}

/// JWT Claims when the role is User
#[derive(Debug)]
pub struct UserAccess(pub JWTClaims);

/// JWT Claims when the role is Admin
#[derive(Debug)]
pub struct AdminAccess(pub JWTClaims);

impl Display for UserAccess {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Display for AdminAccess {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Error type for authorization failures.
#[derive(Debug, Error)]
pub enum AuthError {
  #[error("Missing authorization")]
  MissingAuth,
  #[error("Invalid token")]
  InvalidToken,
  #[error("Role `{0}` is not permitted access")]
  RoleNotPermitted(Role),
}

impl IntoResponse for AuthError {
  fn into_response(self) -> Response {
    event!(
      target: USER_MS_TARGET,
      Level::ERROR,
      "Autorization failed: {self}"
    );
    let body = Json(json!({
        "error": "not authorized",
    }));
    (StatusCode::FORBIDDEN, body).into_response()
  }
}

/// JWT secret key for decoding.
#[derive(Clone)]
pub struct JwtSecretKey(pub DecodingKey);

impl FromStr for JwtSecretKey {
  type Err = Infallible;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(Self(DecodingKey::from_secret(s.as_bytes())))
  }
}

impl Deref for JwtSecretKey {
  type Target = DecodingKey;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
