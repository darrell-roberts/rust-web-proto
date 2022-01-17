use crate::extractors::ValidatingJson;
use crate::types::{HandlerError, Persist};
use crate::USER_MS_TARGET;
use axum::extract::{Json, Path};
use axum_macros::debug_handler;
use http::StatusCode;
use serde_json::Value;
use tracing::{event, Level};
use user_persist::types::{UpdateUser, User, UserKey, UserSearch};

pub async fn get_user(
  db: Persist,
  id: Path<UserKey>,
  // claims: AdminAccess,
) -> Result<Json<Option<User>>, HandlerError> {
  // event!(
  //   target: USER_MS_TARGET,
  //   Level::DEBUG,
  //   "Received id: {id:?} with claims: {claims:?}"
  // );

  let user = db.get_user(&id).await?;

  event!(target: USER_MS_TARGET, Level::DEBUG, "db result: {user:?}");

  Ok(Json(user))
}

pub async fn save_user(
  user: ValidatingJson<User>,
  db: Persist,
  // _claims: UserAccess,
) -> Result<Json<User>, HandlerError> {
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "saving user: {user:?}"
  );
  let saved_user = db.save_user(&user).await?;
  Ok(Json(saved_user))
}

pub async fn update_user(
  db: Persist,
  user: ValidatingJson<UpdateUser>,
  // _claims: AdminAccess,
) -> Result<StatusCode, HandlerError> {
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "updating user with {user:?}"
  );
  db.update_user(&user).await?;
  Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn search_users(
  user_search: ValidatingJson<UserSearch>,
  db: Persist,
  // _claims: AdminAccess,
) -> Result<Json<Vec<User>>, HandlerError> {
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "Searching for users with {user_search:?}"
  );
  let results = db.search_users(&user_search).await?;
  Ok(Json(results))
}

pub async fn count_users(
  db: Persist,
  // claims: AdminAccess,
) -> Result<Json<Vec<Value>>, HandlerError> {
  // event!(target: USER_MS_TARGET, Level::DEBUG, "Claims: {claims:?}");
  let counts = db.count_genders().await?;
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "User counts: {counts:?}"
  );
  Ok(Json(counts))
}
