use serde::{Deserialize, Serialize};
use user_persist::persistence::PersistenceError;
use warp::reject::Reject;

#[derive(Debug, Serialize, Deserialize)]
pub struct WarpPersistenceError(pub String);

impl Reject for WarpPersistenceError {}

impl From<PersistenceError> for WarpPersistenceError {
  fn from(err: PersistenceError) -> Self {
    WarpPersistenceError(err.to_string())
  }
}
