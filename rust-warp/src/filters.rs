use super::handlers;
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use tracing::{event, info_span, instrument, Level};
use user_persist::persistence::UserPersistence;
use user_persist::types::UserKey;
use uuid::Uuid;
use warp::Filter;

const FRAMEWORK_TARGET: &str = "ms-framework";

type UserPersist = Arc<dyn UserPersistence>;

fn with_db(
  db: UserPersist,
) -> impl Filter<Extract = (UserPersist,), Error = Infallible> + Clone
{
  warp::any().map(move || db.clone())
}

#[instrument(name = "request-span", target = "ms-framework")]
fn with_req_id(
) -> impl Filter<Extract = (Uuid,), Error = warp::Rejection> + Clone {
  warp::header::optional::<String>("x-request-id").map(
    |req_id_header: Option<String>| {
      let req_id = req_id_header
        .map(|s| Uuid::parse_str(&s).unwrap_or_else(|_| Uuid::new_v4()));

      req_id.unwrap_or_else(Uuid::new_v4)
    },
  )
}

fn test_wrapper<F, T>(
  filter: F,
) -> impl Filter<Extract = impl warp::Reply, Error = Infallible> + Clone + Send + Sync // + 'static
where
  F: Filter<Extract = (T,), Error = Infallible>
    + Clone
    + Send
    + Sync,
    // + 'static,
  F::Extract: warp::Reply,
  // T: std::fmt::Debug
{
  warp::any()
    .map(|| warp::header::optional::<String>("Host"))
    // .untuple_one()
    .map(|_h| {
      event!(target: FRAMEWORK_TARGET, Level::DEBUG, "Before filter");
    })
    .untuple_one()
    .and(filter)
    .map(|res| {
      event!(target: FRAMEWORK_TARGET, Level::DEBUG, "After filter");
      res
    })
}

// fn add_req_id_header (uuid: Uuid) -> Result<impl warp::Reply, warp::Rejection> {
//     Ok(warp::reply::with_header(warp::reply::reply, "x-request-id", uuid.to_string()))
// }

#[derive(Serialize)]
struct ErrorMessage {
  label: &'static str,
  message: String,
}

#[instrument(skip_all, name = "request-span", target = "user-ms")]
pub fn user(
  db: UserPersist,
) -> impl Filter<Extract = impl warp::Reply, Error = Infallible> + Clone {
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
      info_span!(target: FRAMEWORK_TARGET, "request-span", method = %req.method(), path = %req.path())
    }))
    .map(|reply| {
      warp::reply::with_header(reply, "x-request-id", "abc")
    })
    .recover(handle_rejection)
    .with(warp::wrap_fn(test_wrapper))
}

async fn handle_rejection(
  _err: warp::Rejection,
) -> Result<impl warp::Reply, Infallible> {
  let error_message = ErrorMessage {
    label: "blah",
    message: String::from("hello"),
  };
  let json = warp::reply::json(&error_message);
  Ok(warp::reply::with_status(
    json,
    warp::http::StatusCode::BAD_REQUEST,
  ))
}

pub fn get_user(
  db: UserPersist,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path!(UserKey)
    .and(warp::get())
    .and(with_db(db))
    .and(with_req_id())
    .and_then(handlers::handle_get_user)
}

pub fn search_users(
  db: UserPersist,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path("search")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_db(db))
    .and(with_req_id())
    .and_then(handlers::handle_search_users)
}

pub fn save_user(
  db: UserPersist,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::post()
    .and(warp::body::json())
    .and(with_db(db))
    .and_then(handlers::handle_save_user)
}

pub fn count_genders(
  db: UserPersist,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path("counts")
    .and(with_db(db))
    .and(with_req_id())
    .and_then(handlers::handle_count_genders)
}
