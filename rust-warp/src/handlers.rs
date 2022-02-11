use crate::types::WarpPersistenceError;
use std::sync::Arc;
use tracing::{event, instrument, Level};
use user_persist::{
  persistence::{PersistenceError, UserPersistence},
  types::{User, UserKey, UserSearch},
};
use warp::{http::StatusCode, reply, Rejection, Reply};

fn to_warp_error(err: PersistenceError) -> WarpPersistenceError {
  WarpPersistenceError(err.to_string())
}

const USER_MS_TARGET: &str = "user-ms";

type UserPersist = Arc<dyn UserPersistence>;

pub async fn handle_get_user(
  id: UserKey,
  db: UserPersist,
) -> Result<impl Reply, Rejection> {
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "Getting user with id: {id:?}"
  );
  let user = db.get_user(&id).await.map_err(to_warp_error)?;
  event!(target: USER_MS_TARGET, Level::DEBUG, "User: {user:?}");
  match user {
    Some(u) => Ok(reply::json(&u).into_response()),
    None => Ok(reply::with_status("", StatusCode::NOT_FOUND).into_response()),
  }
}

#[instrument(skip(db, search), name = "request-span", target = "user-ms")]
pub async fn handle_search_users(
  search: UserSearch,
  db: UserPersist,
) -> Result<impl Reply, Rejection> {
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "searching with {search:?}"
  );
  let users = db.search_users(&search).await.map_err(to_warp_error)?;
  event!(
    target: USER_MS_TARGET,
    Level::DEBUG,
    "search result: {users:?}"
  );
  Ok(reply::json(&users))
}

pub async fn handle_save_user(
  user: User,
  db: UserPersist,
) -> Result<impl Reply, Rejection> {
  let saved_user = db.save_user(&user).await.map_err(to_warp_error)?;
  Ok(reply::json(&saved_user))
}

pub async fn handle_count_genders(
  db: UserPersist,
) -> Result<impl Reply, Rejection> {
  event!(target: USER_MS_TARGET, Level::DEBUG, "counting users");
  let counts = db.count_genders().await.map_err(to_warp_error)?;
  Ok(reply::json(&counts))
}
