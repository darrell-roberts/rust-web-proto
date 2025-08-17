use crate::types::WarpDatabaseError;
use std::sync::Arc;
use tracing::{debug, instrument};
use user_database::{
    database::{DatabaseError, UserDatabase},
    types::{User, UserKey, UserSearch},
};
use warp::{http::StatusCode, reply, Rejection, Reply};

fn to_warp_error(err: DatabaseError) -> WarpDatabaseError {
    WarpDatabaseError(err.to_string())
}

pub async fn handle_get_user<P>(id: UserKey, db: Arc<P>) -> Result<impl Reply, Rejection>
where
    P: UserDatabase,
{
    debug!("Getting user with id: {id:?}");
    let user = db.get_user(&id).await.map_err(to_warp_error)?;
    debug!("User: {user:?}");
    match user {
        Some(u) => Ok(reply::json(&u).into_response()),
        None => Ok(reply::with_status("", StatusCode::NOT_FOUND).into_response()),
    }
}

#[instrument(skip(db, search), name = "request-span", target = "user-ms")]
pub async fn handle_search_users<P>(search: UserSearch, db: Arc<P>) -> Result<impl Reply, Rejection>
where
    P: UserDatabase,
{
    debug!("searching with {search:?}");
    let users = db.search_users(&search).await.map_err(to_warp_error)?;
    debug!("search result: {users:?}");
    Ok(reply::json(&users))
}

pub async fn handle_save_user<P>(user: User, db: Arc<P>) -> Result<impl Reply, Rejection>
where
    P: UserDatabase,
{
    let saved_user = db.save_user(&user).await.map_err(to_warp_error)?;
    Ok(reply::json(&saved_user))
}

pub async fn handle_count_genders<P>(db: Arc<P>) -> Result<impl Reply, Rejection>
where
    P: UserDatabase,
{
    debug!("counting users");
    let counts = db.count_genders().await.map_err(to_warp_error)?;
    Ok(reply::json(&counts))
}
