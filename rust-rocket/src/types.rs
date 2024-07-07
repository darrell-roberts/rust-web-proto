use crate::{fairings::RequestId, FRAMEWORK_TARGET};
use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use rocket::{
    http::{ContentType, Header, Status},
    request::{FromParam, Request},
    response::{Responder, Response},
    serde::{json::serde_json::to_string, Deserialize, Serialize},
};
use std::io::Cursor;
use thiserror::Error;
use tracing::{event, Level};
use user_persist::{persistence::PersistenceError, types::UserKey, Validate};

pub const USER_MS_TARGET: &str = "user-ms";

/// Newtype wrapper for bson `ObjectId`
pub struct UserKeyReq(pub UserKey);

// Similar to a type class instance
impl<'a> FromParam<'a> for UserKeyReq {
    // similar to an associated type family.
    type Error = mongodb::bson::oid::Error;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        let object_id = ObjectId::parse_str(param)?;
        Ok(UserKeyReq(UserKey(object_id.to_string())))
    }
}

/// Rocket Json request guard that applies validations
/// using `Validate` trait.
#[derive(Debug)]
pub struct JsonValidation<T: Validate>(pub T);

/// Models error response sent back to the
/// caller when any errors are returned.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ErrorResponder<'a> {
    label: &'a str,
    message: String,
}

impl From<PersistenceError> for ErrorResponder<'static> {
    fn from(err: PersistenceError) -> Self {
        ErrorResponder {
            message: err.to_string(),
            label: "persistence.error",
        }
    }
}

/// Error responder to set a status of 422 and as JSON error resonse.
impl<'r> Responder<'r, 'static> for ErrorResponder<'static> {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = to_string(&self).unwrap_or_default();
        let req_id = req
            .local_cache(|| RequestId(None))
            .0
            .unwrap_or_default()
            .to_string();
        Response::build()
            .header(ContentType::JSON)
            .header(Header::new("X-Request-Id", req_id))
            .status(Status::UnprocessableEntity)
            .sized_body(json.len(), Cursor::new(json))
            .ok()
    }
}

/// Enumeration of Roles
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum Role {
    Admin,
    User,
}

/// Type for claims in the JWT token used for
/// authorizing requests.
#[derive(Deserialize, Serialize, Debug)]
pub struct JWTClaims {
    /// Subjet. This is the user identifier.
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
    NoAuthorizationHeader,
    #[error("Invalid JWT length")]
    InvalidJwtLength {
        #[from]
        source: hmac::digest::InvalidLength,
    },
    #[error("Verification failed Invalid JWT")]
    VerificationFailed {
        #[from]
        source: jwt::Error,
    },
    #[error("Invalid role")]
    InvalidRole,
    #[error("JWT has expired")]
    Expired,
}

impl JWTClaims {
    /// Method that checks if the JWT has expired.
    /// This is has a max age of 5 minutes.
    pub fn check_expired(self) -> Result<Self, JWTError> {
        let exp = DateTime::from_timestamp(self.exp, 0).ok_or(JWTError::Expired)?;
        let now = Utc::now();
        let exp_minutes = (exp - now).num_minutes();

        event!(
          target: FRAMEWORK_TARGET,
          Level::DEBUG,
          "Jwt expires in: {exp_minutes} minutes"
        );

        if exp_minutes <= 0 {
            Err(JWTError::Expired)
        } else {
            Ok(self)
        }
    }
}

/// JWT Claims when the role is User
#[derive(Debug)]
pub struct UserAccess(#[allow(dead_code)] pub JWTClaims);

/// JWT Claims when the role is Admin
#[derive(Debug)]
pub struct AdminAccess(#[allow(dead_code)] pub JWTClaims);
