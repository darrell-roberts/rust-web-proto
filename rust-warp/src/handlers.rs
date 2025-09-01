//! Handlers.
use crate::types::WarpDatabaseError;
use futures::{stream, StreamExt as _, TryStreamExt as _};
use std::{future, sync::Arc};
use tracing::{debug, error, instrument};
use user_database::{
    database::{DatabaseError, UserDatabase},
    types::{User, UserKey, UserSearch},
};
use warp::{
    http::{self, StatusCode},
    reject::reject,
    reply::{self},
    Rejection, Reply,
};

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

#[instrument(skip_all, name = "request-span", target = "user-ms")]
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

pub async fn download_users<P>(db: Arc<P>) -> Result<impl Reply, Rejection>
where
    P: UserDatabase,
{
    let header = stream::iter(std::iter::once(vec![b'[']));
    let footer = stream::iter(std::iter::once(vec![b']']));

    let body = db
        .download()
        .await
        .inspect_err(|err| error!("Failed to read user record {err}"))
        .filter_map(|r| future::ready(r.ok()))
        .enumerate()
        .filter_map(|(index, u)| async move {
            serde_json::to_vec(&u)
                .map(|bytes| {
                    if index > 0 {
                        Vec::from_iter([b','].into_iter().chain(bytes))
                    } else {
                        bytes
                    }
                })
                .ok()
        });

    let _response = http::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(header.chain(body).chain(footer).boxed())
        .unwrap();

    // Ok(response)
    // Warp does not support streaming body anymore :-(
    // https://github.com/seanmonstar/warp/issues/1136
    Err::<String, _>(reject())
}
