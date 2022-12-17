use crate::{
  types::jwt::{AdminAccess, AuthError, JWTClaims, Role, UserAccess},
  AppConfig,
};
use async_trait::async_trait;
use axum::{
  extract::{FromRequestParts, TypedHeader},
  headers::{authorization::Bearer, Authorization},
  http::request::Parts,
};
use jsonwebtoken::{decode, Validation};
use std::sync::Arc;

#[async_trait]
impl<S> FromRequestParts<S> for JWTClaims
where
  S: Send + Sync,
{
  type Rejection = AuthError;

  async fn from_request_parts(
    req: &mut Parts,
    state: &S,
  ) -> Result<Self, Self::Rejection> {
    extract_jwt(req, state).await
  }
}

#[async_trait]
/// Extractor that enforces access for an Amdin role.
impl<S> FromRequestParts<S> for AdminAccess
where
  S: Send + Sync,
{
  type Rejection = AuthError;

  async fn from_request_parts(
    req: &mut Parts,
    state: &S,
  ) -> Result<Self, Self::Rejection> {
    match extract_jwt(req, state).await? {
      claims if claims.role == Role::Admin => Ok(Self(claims)),
      JWTClaims { role, .. } => Err(AuthError::RoleNotPermitted(role)),
    }
  }
}

#[async_trait]
/// Extractor that enforces access for a User role.
impl<S> FromRequestParts<S> for UserAccess
where
  S: Send + Sync,
{
  type Rejection = AuthError;

  async fn from_request_parts(
    req: &mut Parts,
    state: &S,
  ) -> Result<Self, Self::Rejection> {
    match extract_jwt(req, state).await? {
      claims if claims.role == Role::User => Ok(Self(claims)),
      JWTClaims { role, .. } => Err(AuthError::RoleNotPermitted(role)),
    }
  }
}

/// Parse the JWT from the request header.
async fn extract_jwt<S>(
  req: &mut Parts,
  state: &S,
) -> Result<JWTClaims, AuthError>
where
  S: Send + Sync,
{
  let TypedHeader(Authorization(bearer)) =
    TypedHeader::<Authorization<Bearer>>::from_request_parts(req, state)
      .await
      .map_err(|_| AuthError::MissingAuth)?;
  let key = req
    .extensions
    .get::<Arc<AppConfig>>()
    .map(|config| config.jwt_decoding_key())
    .expect("Missing Extension(Arc<AppConfig>)");

  decode::<JWTClaims>(bearer.token(), key, &Validation::default())
    .map(|t| t.claims)
    .map_err(|_| AuthError::InvalidToken)
}
