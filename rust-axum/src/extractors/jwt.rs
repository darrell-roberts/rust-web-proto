use crate::{
  types::jwt::{AdminAccess, AuthError, JWTClaims, Role, UserAccess},
  AppConfig,
};
use async_trait::async_trait;
use axum::{
  body::HttpBody,
  extract::{FromRequest, RequestParts, TypedHeader},
  headers::{authorization::Bearer, Authorization},
  BoxError,
};
use jsonwebtoken::{decode, Validation};
use std::sync::Arc;

#[async_trait]
impl<B> FromRequest<B> for JWTClaims
where
  B: HttpBody + Send,
  B::Data: Send,
  B::Error: Into<BoxError>,
{
  type Rejection = AuthError;

  async fn from_request(
    req: &mut RequestParts<B>,
  ) -> Result<Self, Self::Rejection> {
    extract_jwt(req).await
  }
}

#[async_trait]
/// Extractor that enforces access for an Amdin role.
impl<B> FromRequest<B> for AdminAccess
where
  B: HttpBody + Send,
  B::Data: Send,
  B::Error: Into<BoxError>,
{
  type Rejection = AuthError;

  async fn from_request(
    req: &mut RequestParts<B>,
  ) -> Result<Self, Self::Rejection> {
    match extract_jwt(req).await? {
      claims if claims.role == Role::Admin => Ok(Self(claims)),
      JWTClaims { role, .. } => Err(AuthError::RoleNotPermitted(role)),
    }
  }
}

#[async_trait]
/// Extractor that enforces access for a User role.
impl<B> FromRequest<B> for UserAccess
where
  B: HttpBody + Send,
  B::Data: Send,
  B::Error: Into<BoxError>,
{
  type Rejection = AuthError;

  async fn from_request(
    req: &mut RequestParts<B>,
  ) -> Result<Self, Self::Rejection> {
    match extract_jwt(req).await? {
      claims if claims.role == Role::User => Ok(Self(claims)),
      JWTClaims { role, .. } => Err(AuthError::RoleNotPermitted(role)),
    }
  }
}

/// Parse the JWT from the request header.
async fn extract_jwt<B: HttpBody + Send>(
  req: &mut RequestParts<B>,
) -> Result<JWTClaims, AuthError> {
  let TypedHeader(Authorization(bearer)) =
    TypedHeader::<Authorization<Bearer>>::from_request(req)
      .await
      .map_err(|_| AuthError::MissingAuth)?;

  let key = req
    .extensions()
    .get::<Arc<AppConfig>>()
    .map(|config| config.jwt_decoding_key())
    .expect("Missing Extension(Arc<AppConfig>)");

  decode::<JWTClaims>(bearer.token(), key, &Validation::default())
    .map(|t| t.claims)
    .map_err(|_| AuthError::InvalidToken)
}
