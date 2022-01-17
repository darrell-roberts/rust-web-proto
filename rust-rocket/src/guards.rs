use crate::fairings::RequestId;
use crate::types::{
  AdminAccess, JWTClaims, JWTError, JsonValidation, Role, UserAccess,
};
use crate::{FRAMEWORK_TARGET, TEST_JWT_SECRET};
use hmac::{Hmac, Mac};
use jwt::VerifyWithKey;
use rocket::data::{self, Data, FromData, Limits};
use rocket::http::Status;
use rocket::request::{self, local_cache, FromRequest, Request};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tracing::{event, Level};
use validator::Validate;

#[derive(Debug, Error)]
pub enum JsonValidationError {
  #[error("Validation failed")]
  ValidationFailed {
    #[from]
    source: validator::ValidationErrors,
  },
  #[error("Parsing failed")]
  ParseError {
    #[from]
    source: serde_json::Error,
  },
  #[error("Payload too large")]
  TooLarge,
  #[error("IO error")]
  IO {
    #[from]
    source: std::io::Error,
  },
}

#[derive(Serialize, Debug)]
pub struct UserErrorMessage(pub String);

/// A Json Data Guard that runs valiation on the deserialized types via
/// the valiation crate. The validation crate requires the derserialized
/// type have the `Validate` trait.
#[rocket::async_trait]
impl<'r, T> FromData<'r> for JsonValidation<T>
where
  T: Deserialize<'r> + Validate,
{
  type Error = JsonValidationError;

  async fn from_data(
    req: &'r Request<'_>,
    data: Data<'r>,
  ) -> data::Outcome<'r, Self> {
    let limit = req.limits().get("json").unwrap_or(Limits::JSON);
    let req_id = req.local_cache(|| RequestId(None));
    let string = match data.open(limit).into_string().await {
      Ok(s) if s.is_complete() => s.into_inner(),
      Ok(_) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::ERROR,
          %req_id,
          "Payload limit exceeded"
        );

        req.local_cache(|| {
          Some(UserErrorMessage("payload limit exceeded".to_owned()))
        });

        return data::Outcome::Failure((
          Status::PayloadTooLarge,
          JsonValidationError::TooLarge,
        ));
      }
      Err(e) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::ERROR,
          %req_id,
          "IO Error {} {} {e}",
          req.method(),
          req.uri()
        );

        req.local_cache(|| Some(UserErrorMessage(e.to_string())));

        return data::Outcome::Failure((
          Status::InternalServerError,
          JsonValidationError::IO { source: e },
        ));
      }
    };

    let string = local_cache!(req, string);

    match serde_json::from_str::<T>(string)
      .map_err(|e| JsonValidationError::ParseError { source: e })
    {
      Ok(t) => match t.validate() {
        Ok(_) => data::Outcome::Success(JsonValidation(t)),
        Err(e) => {
          event!(
            target: FRAMEWORK_TARGET,
            Level::ERROR,
            %req_id,
            "Validation failed {} {}: {e}",
            req.method(),
            req.uri()
          );

          req.local_cache(|| Some(e.clone()));
          data::Outcome::Failure((
            Status::BadRequest,
            JsonValidationError::ValidationFailed { source: e },
          ))
        }
      },
      Err(e) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::ERROR,
          %req_id,
          "Deserialization failed {} {} : {e} {string}",
          req.method(),
          req.uri()
        );

        req.local_cache(|| Some(UserErrorMessage(e.to_string())));
        data::Outcome::Failure((Status::InternalServerError, e))
      }
    }
  }
}

// Request guards for access control. Role is extracted
// from a jwt claim and converted to a type.

type HmacSha256 = Hmac<Sha256>;

fn extract_jwt(req: &'_ Request<'_>) -> Result<JWTClaims, JWTError> {
  let req_id = req.local_cache(|| RequestId(None));
  match req.headers().get_one("Authorization").map(|s| &s[7..]) {
    Some(jwt_token) => {
      event!(
        target: FRAMEWORK_TARGET,
        Level::DEBUG,
        %req_id,
        "{} {} jwt_token: {jwt_token}",
        req.method(),
        req.uri()
      );

      let key = HmacSha256::new_from_slice(TEST_JWT_SECRET)?;

      let claims: JWTClaims = jwt_token.verify_with_key(&key)?;

      Ok(claims.check_expired()?)
    }
    None => Err(JWTError::NoAutorizationHeader),
  }
}

// Parse and validate a JWT token.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for JWTClaims {
  type Error = JWTError;

  async fn from_request(
    req: &'r Request<'_>,
  ) -> request::Outcome<Self, Self::Error> {
    match extract_jwt(req) {
      Ok(j) => request::Outcome::Success(j),
      Err(e) => request::Outcome::Failure((Status::Forbidden, e)),
    }
  }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserAccess {
  type Error = JWTError;

  async fn from_request(
    req: &'r Request<'_>,
  ) -> request::Outcome<Self, Self::Error> {
    let req_id = req.local_cache(|| RequestId(None));
    match extract_jwt(req) {
      Ok(j) if j.role == Role::User => request::Outcome::Success(UserAccess(j)),
      Ok(_) => {
        request::Outcome::Failure((Status::Forbidden, JWTError::InvalidRole))
      }
      Err(e) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::WARN,
          %req_id,
          "failed user access for {} {} {e}",
          req.method(),
          req.uri()
        );

        request::Outcome::Failure((Status::Forbidden, e))
      }
    }
  }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminAccess {
  type Error = JWTError;

  async fn from_request(
    req: &'r Request<'_>,
  ) -> request::Outcome<Self, Self::Error> {
    let req_id = req.local_cache(|| RequestId(None));
    match extract_jwt(req) {
      Ok(j) if j.role == Role::Admin => {
        request::Outcome::Success(AdminAccess(j))
      }
      Ok(_) => {
        request::Outcome::Failure((Status::Forbidden, JWTError::InvalidRole))
      }
      Err(e) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::WARN,
          %req_id,
          "failed admin access for {} {}",
          req.method(),
          req.uri()
        );
        request::Outcome::Failure((Status::Forbidden, e))
      }
    }
  }
}
