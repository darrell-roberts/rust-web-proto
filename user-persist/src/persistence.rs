/*!
Generic UserPersistence Trait and types.
*/
use crate::types::{UpdateUser, User, UserKey, UserSearch};
use serde_json::Value;
use std::fmt::Debug;
use thiserror::Error;

/// Type alias for user-persist Result.
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Abstract our persistence API so it can be swapped out
/// for any backend.
#[async_trait::async_trait]
pub trait UserPersistence: Send + Sync + Debug {
    /// Lookup a user from persistent storage.
    async fn get_user(&self, id: &UserKey) -> PersistenceResult<Option<User>>;
    /// Save a user to persistent storage.
    async fn save_user(&self, user: &User) -> PersistenceResult<User>;
    /// Update a user in persistent storage.
    async fn update_user(&self, user: &UpdateUser) -> PersistenceResult<()>;
    /// Remove a user from persistent storage.
    async fn remove_user(&self, user: &UserKey) -> PersistenceResult<()>;
    /// Search for users with search criteria in `UserSearch` from
    /// persistent storage.
    async fn search_users(&self, user: &UserSearch) -> PersistenceResult<Vec<User>>;
    /// Count the number of users grouping by gender.
    async fn count_genders(&self) -> Result<Vec<Value>, PersistenceError>;
}

/// Enumeration of persistence errors.
#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Mongodb error: `{0}`")]
    MongoError(#[from] mongodb::error::Error),
    #[error("Persistence Test Failure")]
    TestError,
    #[error("Bson error: `{0}`")]
    BsonError(#[from] mongodb::bson::oid::Error),
}
