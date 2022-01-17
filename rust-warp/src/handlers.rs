use super::types::WarpPersistenceError;
use std::sync::Arc;
use tracing::{event, instrument, Level};
use user_persist::persistence::{UserPersistence, PersistenceError};
use user_persist::types::{User, UserSearch, UserKey};
use uuid::Uuid;
use warp::reply;

fn to_warp_error(err: PersistenceError) -> WarpPersistenceError {
  WarpPersistenceError(err.to_string())
}

const USER_MS_TARGET: &str = "user-ms";

type UserPersist = Arc<dyn UserPersistence>;

#[instrument(skip(db, id), name = "request-span", target = "user-ms")]
pub async fn handle_get_user(
  id: UserKey,
  db: UserPersist,
  req_id: Uuid,
) -> Result<impl warp::Reply, warp::Rejection> {
  event!(target: "user-ms", Level::DEBUG, %req_id, "Getting user with id: {id:?}");
  let user = db.get_user(&id).await.map_err(to_warp_error)?;
  Ok(reply::json(&user))
}

#[instrument(skip(db, search), name = "request-span", target = "user-ms")]
pub async fn handle_search_users(
  search: UserSearch,
  db: UserPersist,
  req_id: Uuid,
) -> Result<impl warp::Reply, warp::Rejection> {
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
  Ok(warp::reply::json(&users))
}

pub async fn handle_save_user(
  user: User,
  db: UserPersist,
) -> Result<impl warp::Reply, warp::Rejection> {
  let saved_user = db.save_user(&user).await.map_err(to_warp_error)?;
  Ok(warp::reply::json(&saved_user))
}

#[instrument(skip(db), name = "request-span", target = "user-ms")]
pub async fn handle_count_genders(
  db: UserPersist,
  req_id: Uuid,
) -> Result<impl warp::Reply, warp::Rejection> {
  event!(target: USER_MS_TARGET, Level::DEBUG, "counting users");
  let counts = db.count_genders().await.map_err(to_warp_error)?;
  Ok(warp::reply::json(&counts))
}
