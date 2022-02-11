use crate::{
  extractors::{hashing::HashedValidatingJson, validator::ValidatingJson},
  security::hashing::{HashableVector, HashingResponse},
  types::{
    handler::{HandlerError, Persist},
    jwt::{AdminAccess, UserAccess},
  },
  AppConfig, USER_MS_TARGET,
};
use axum::{
  extract::{Extension, Json, Path},
  response::IntoResponse,
};
use futures::stream::{self, StreamExt};
use http::{Response, StatusCode};
use hyper::Body;
use serde_json::{to_string, Value};
use std::sync::Arc;
use tracing::debug;
use user_persist::{
  mongo_persistence::MongoPersistence,
  types::{UpdateUser, User, UserKey, UserSearch},
};

type HandlerResult<T> = Result<T, HandlerError>;
type AppCfg = Extension<Arc<AppConfig>>;

/// Get user handler.
pub async fn get_user(
  db: Persist,
  Path(id): Path<UserKey>,
  claims: AdminAccess,
  Extension(app_config): AppCfg,
) -> impl IntoResponse {
  debug!(
    target: USER_MS_TARGET,
    "Received id: {id} with claims: {claims}"
  );

  let user = db.get_user(&id).await?;

  debug!(
    target: USER_MS_TARGET,
    "db result: {}",
    match user {
      Some(ref u) => format!("{}", u),
      None => "No User".to_owned(),
    }
  );

  user
    .map(|u| HashingResponse::new(app_config, u))
    .ok_or(HandlerError::ResourceNotFound)
}

/// Save user handler.
pub async fn save_user(
  ValidatingJson(user): ValidatingJson<User>,
  db: Persist,
  _claims: UserAccess,
  Extension(app_config): AppCfg,
) -> impl IntoResponse {
  debug!(target: USER_MS_TARGET, "saving user: {user}");
  db.save_user(&user)
    .await
    .map_err(HandlerError::from)
    .map(|u| HashingResponse::new(app_config, u))
}

/// Update user handler.
pub async fn update_user(
  db: Persist,
  HashedValidatingJson(user): HashedValidatingJson<UpdateUser>,
  _claims: AdminAccess,
) -> HandlerResult<StatusCode> {
  debug!(target: USER_MS_TARGET, "updating user with {user}");
  db.update_user(&user)
    .await
    .map(|_| StatusCode::OK)
    .map_err(HandlerError::from)
}

/// Search users handler.
pub async fn search_users(
  ValidatingJson(user_search): ValidatingJson<UserSearch>,
  db: Persist,
  claims: AdminAccess,
  Extension(app_config): AppCfg,
) -> impl IntoResponse {
  debug!(
    target: USER_MS_TARGET,
    "Searching for users with {user_search} and claims {claims}"
  );
  db.search_users(&user_search)
    .await
    .map(|v| HashableVector::new(app_config, v))
    .map_err(HandlerError::from)
    .into_response()
}

/// Delete user handler.
pub async fn delete_user(
  db: Persist,
  Path(id): Path<UserKey>,
  _claims: AdminAccess,
) -> impl IntoResponse {
  match db.remove_user(&id).await {
    Ok(_) => (StatusCode::OK).into_response(),
    Err(e) => HandlerError::from(e).into_response(),
  }
}

/// Count users handler.
pub async fn count_users(
  db: Persist,
  claims: AdminAccess,
) -> HandlerResult<Json<Vec<Value>>> {
  debug!(target: USER_MS_TARGET, "Claims: {claims}");
  let counts = db.count_genders().await?;
  debug!(target: USER_MS_TARGET, "User counts: {counts:?}");
  Ok(Json(counts))
}

// This gets a stream of MongoUser types that are
// streamed from the mongodb cursor. The stream is
// transformed to it's JSON form and wrapped in a
// StreamBody resulting in a Stream from mongodb back
// to http client.

/// Download users handler
pub async fn download_users(
  db: Extension<Arc<MongoPersistence>>,
  claims: AdminAccess,
) -> HandlerResult<impl IntoResponse> {
  debug!(target: USER_MS_TARGET, "Streaming users for {claims}");

  // Chain my stream with a header and footer
  // in order to reconstitute a json array for
  // the mongodb stream of documents returned.
  let header = stream::iter(vec![Ok("[".to_string())]);
  let footer = stream::iter(vec![Ok("]".to_string())]);

  let stream = db
    .download()
    .await?
    .filter_map(|r| async { r.ok() })
    .map(|u| to_string(&u).map(|s| format!("{s},")));

  let response_stream = header.chain(stream).chain(footer);

  Ok(
    Response::builder()
      .status(StatusCode::OK)
      .header("Content-Type", "application/json")
      .body(Body::wrap_stream(response_stream))
      .unwrap(),
  )
}
