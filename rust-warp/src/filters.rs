//! Filter functions
use crate::handlers;
use serde_json::json;
use std::{convert::Infallible, sync::Arc};
use tracing::{debug, info_span};
use user_database::{database::UserDatabase, types::UserKey};
use uuid::Uuid;
use warp::Filter;

type Database<T> = Arc<T>;

/// Provides the Database API
fn with_db<P>(db: Database<P>) -> impl Filter<Extract = (Database<P>,), Error = Infallible> + Clone
where
    P: UserDatabase,
{
    warp::any().map(move || db.clone())
}

fn test_wrapper<F, T>(
    filter: F,
) -> impl Filter<Extract = impl warp::Reply, Error = Infallible> + Clone + Send + Sync
where
    F: Filter<Extract = (T,), Error = Infallible> + Clone + Send + Sync,
    F::Extract: warp::Reply,
{
    warp::any()
        .map(|| warp::header::optional::<String>("Host"))
        .map(|_h| {
            debug!("Before filter");
        })
        .untuple_one()
        .and(filter)
        .map(|res| {
            debug!("After filter");
            res
        })
}

/// Top level filter for the User API.
pub fn user<P>(
    db: Database<P>,
) -> impl Filter<Extract = impl warp::Reply, Error = Infallible> + Clone
where
    P: UserDatabase,
{
    let base_path = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("user"));

    let routes = base_path.and(
        get_user(db.clone())
            .or(search_users(db.clone()))
            .or(save_user(db.clone()))
            .or(count_genders(db)),
    );

    routes
        .with(warp::filters::compression::gzip())
        .with(warp::trace(|req| {
            let headers = req.request_headers();
            let req_id = headers
                .get("x-request-id")
                .and_then(|v| v.to_str().ok().map(String::from))
                .unwrap_or_else(|| Uuid::new_v4().to_string());
            info_span!("request-span", %req_id, method = %req.method(), path = %req.path())
        }))
        // .map(|reply| {
        //   warp::reply::with_header(reply, "x-request-id", "abc")
        // })
        .recover(handle_rejection)
        .with(warp::wrap_fn(test_wrapper))
}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let error_body = json!({
      "label": "error",
      "message": format!("{err:?}"),
    });
    let json = warp::reply::json(&error_body);
    Ok(warp::reply::with_status(
        json,
        warp::http::StatusCode::BAD_REQUEST,
    ))
}

pub fn get_user<P>(
    db: Database<P>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    P: UserDatabase,
{
    warp::path!(UserKey)
        .and(warp::get())
        .and(with_db(db))
        .and_then(handlers::handle_get_user)
}

pub fn search_users<P>(
    db: Database<P>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    P: UserDatabase,
{
    warp::path("search")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_db(db))
        .and_then(handlers::handle_search_users)
}

pub fn save_user<P>(
    db: Database<P>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    P: UserDatabase,
{
    warp::post()
        .and(warp::body::json())
        .and(with_db(db))
        .and_then(handlers::handle_save_user)
}

pub fn count_genders<P>(
    db: Database<P>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    P: UserDatabase,
{
    warp::path("counts")
        .and(with_db(db))
        .and_then(handlers::handle_count_genders)
}
