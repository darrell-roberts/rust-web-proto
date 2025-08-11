use serde::{Deserialize, Serialize};
use user_database::database::DatabaseError;
use warp::reject::Reject;

#[derive(Debug, Serialize, Deserialize)]
pub struct WarpDatabaseError(pub String);

impl Reject for WarpDatabaseError {}

impl From<DatabaseError> for WarpDatabaseError {
    fn from(err: DatabaseError) -> Self {
        WarpDatabaseError(err.to_string())
    }
}
