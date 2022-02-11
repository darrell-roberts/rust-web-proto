use crate::common::FRAMEWORK_TARGET;
use crate::types::{AdminAccess, JWTClaims, JWTError, Role, UserAccess};
use actix_service::{Service, Transform};
use actix_web::{
  body::{BoxBody, MessageBody},
  dev::{Payload, ServiceRequest, ServiceResponse},
  http::StatusCode,
  FromRequest, HttpMessage, HttpRequest, HttpResponse, ResponseError,
};
use chrono::{Duration, Utc};
use futures::{
  future::{ready, Ready},
  Future,
};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use sha2::Sha256;
use std::{clone::Clone, pin::Pin, rc::Rc};
use thiserror::Error;
use tracing::{event, Level};

#[derive(Debug)]
pub struct JwtAuth(Rc<Inner>);

#[derive(Debug, Clone)]
struct Inner {
  // Secret for validating JWT signatures.
  secret: Vec<u8>,
}

pub struct JwtMiddleware<S> {
  service: S,
  inner: Rc<Inner>,
}

impl Default for JwtAuth {
  fn default() -> Self {
    JwtAuth(Rc::new(Inner {
      secret: TEST_JWT_SECRET.to_owned(),
    }))
  }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
  S: Service<
    ServiceRequest,
    Response = ServiceResponse<B>,
    Error = actix_web::Error,
  >,
  B: MessageBody,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = actix_web::Error;
  type Transform = JwtMiddleware<S>;
  type InitError = ();
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(JwtMiddleware {
      service,
      inner: self.0.clone(),
    }))
  }
}

impl<S, B> Service<ServiceRequest> for JwtMiddleware<S>
where
  S: Service<
    ServiceRequest,
    Response = ServiceResponse<B>,
    Error = actix_web::Error,
  >,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<B>;
  type Error = actix_web::Error;
  type Future =
    Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

  actix_service::forward_ready!(service);

  fn call(&self, req: ServiceRequest) -> Self::Future {
    match self.extract_jwt(&req) {
      Ok(claims) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::DEBUG,
          "parsed claims: {claims:?}"
        );
        req.extensions_mut().insert::<JWTClaims>(claims);
      }
      Err(e) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::ERROR,
          "JWT parse failed: {e}"
        );
        return Box::pin(async move { Err(actix_web::Error::from(e)) });
      }
    }

    let fut = self.service.call(req);

    Box::pin(async move {
      let res = fut.await?;
      Ok(res)
    })
  }
}

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
  #[error("IO error")]
  IO {
    #[from]
    source: std::io::Error,
  },
}

type HmacSha256 = Hmac<Sha256>;

pub const TEST_JWT_SECRET: &[u8] = b"TEST_SECRET";

impl<S> JwtMiddleware<S> {
  /// Extract the Authorization header and parse a JWT from
  /// the Bearer <Token> header value.
  fn extract_jwt(&self, req: &ServiceRequest) -> Result<JWTClaims, JWTError> {
    match req
      .headers()
      .get("Authorization")
      .map(|s| s.to_str().unwrap_or(""))
      .map(|s| &s[7..]) // Drop "Bearer "
    {
      Some(jwt_token) => {
        event!(
          target: FRAMEWORK_TARGET,
          Level::DEBUG,
          "{} {} jwt_token: {jwt_token}",
          req.method(),
          req.uri()
        );

        let key = HmacSha256::new_from_slice(&self.inner.secret)?;
        let claims: JWTClaims = jwt_token.verify_with_key(&key)?;

        Ok(claims.check_expired()?)
      }
      None => Err(JWTError::NoAutorizationHeader),
    }
  }
}

/// Create a test JWT with a given role. Token expires in
/// 5 minutes.
pub fn create_test_jwt(role: Role) -> Result<String, JWTError> {
  let key = HmacSha256::new_from_slice(TEST_JWT_SECRET).unwrap();
  let expiration = Utc::now() + Duration::minutes(5);
  let claims = JWTClaims {
    sub: "somebody".to_owned(),
    role,
    exp: expiration.timestamp(),
  };
  Ok(claims.sign_with_key(&key)?)
}

// Attach a claim to a handler without any role
// restrictions.
impl FromRequest for JWTClaims {
  type Error = JWTError;
  type Future = Ready<Result<Self, Self::Error>>;

  fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
    let result = match req.extensions().get::<JWTClaims>() {
      Some(c) => Ok(c.clone()),
      None => Err(JWTError::NoAutorizationHeader),
    };
    ready(result)
  }
}

/// Enforce a handler to have an Admin role as defined in
/// The JWT claims.
impl FromRequest for AdminAccess {
  type Error = JWTError;
  type Future = Ready<Result<Self, Self::Error>>;

  fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
    let result = match req.extensions().get::<JWTClaims>() {
      Some(c) if c.role == Role::Admin => Ok(AdminAccess(c.clone())),
      _ => Err(JWTError::InvalidRole),
    };
    ready(result)
  }
}

/// Enforce a handler to have a User role as defined in
/// the JWT claims.
impl FromRequest for UserAccess {
  type Error = JWTError;
  type Future = Ready<Result<Self, Self::Error>>;

  fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
    let result = match req.extensions().get::<JWTClaims>() {
      Some(c) if c.role == Role::User => Ok(UserAccess(c.clone())),
      _ => Err(JWTError::InvalidRole),
    };
    ready(result)
  }
}

impl ResponseError for JWTError {
  fn status_code(&self) -> StatusCode {
    StatusCode::FORBIDDEN
  }

  fn error_response(&self) -> HttpResponse<BoxBody> {
    HttpResponse::build(StatusCode::FORBIDDEN).body("no access")
  }
}
