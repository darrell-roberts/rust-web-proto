use actix_web::{body, http, HttpResponse, ResponseError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;
use user_database::database::DatabaseError;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("Database error")]
    DatabaseError(#[from] DatabaseError),
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::SERVICE_UNAVAILABLE
    }

    fn error_response(&self) -> HttpResponse<body::BoxBody> {
        let body = serde_json::to_string(&format!("{self}")).unwrap_or_default();
        HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body(body)
    }
}

// Roles via JWT claims
/// Enumeration of Roles
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy)]
pub enum Role {
    Admin,
    User,
}

/// Type for claims in the JWT token used for
/// authorizing requests.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct JWTClaims {
    /// Subject. This is the user identifier.
    pub sub: String,
    // Roles for the subject.
    pub role: Role,
    /// Expiration date time in unix epoch.
    pub exp: i64,
}

/// Error type for all errors that
/// can occur when deserializing and
/// validating a JWT.
#[derive(Debug, Error)]
pub enum JWTError {
    #[error("No auth header")]
    NoAutorizationHeader,
    #[error("Invalid JWT length")]
    InvalidJwtLength(#[from] hmac::digest::InvalidLength),
    #[error("Verification failed Invalid JWT")]
    VerificationFailed(#[from] jwt::Error),
    #[error("Invalid role")]
    InvalidRole,
    #[error("JWT has expired")]
    Expired,
    #[error("Actix web error")]
    ActixError(#[from] actix_web::Error),
}

impl JWTClaims {
    /// Method that checks if the JWT has expired.
    /// This is has a max age of 5 minutes.
    pub fn check_expired(self) -> Result<Self, JWTError> {
        let exp = DateTime::from_timestamp(self.exp, 0).ok_or(JWTError::Expired)?;
        let now = Utc::now();
        let exp_minutes = (exp - now).num_minutes();

        debug!("Jwt expires in: {exp_minutes} minutes");

        if exp_minutes <= 0 {
            Err(JWTError::Expired)
        } else {
            Ok(self)
        }
    }
}

/// JWT Claims when the role is User
#[derive(Debug, Clone)]
pub struct UserAccess(pub JWTClaims);

/// JWT Claims when the role is Admin
#[derive(Debug, Clone)]
pub struct AdminAccess(pub JWTClaims);
