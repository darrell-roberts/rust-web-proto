//! Generic UserDatabase Trait and types.
use crate::types::{UpdateUser, User, UserKey, UserSearch};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::{fmt::Debug, future::Future};
use thiserror::Error;

/// Type alias for user-database Result.
pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// Abstract our database API so it can be swapped out
/// for any backend.
pub trait UserDatabase: Send + Sync + Debug {
    /// Lookup a user from database storage.
    fn get_user(&self, id: &UserKey) -> impl Future<Output = DatabaseResult<Option<User>>> + Send;
    /// Save a user to database storage.
    fn save_user(&self, user: &User) -> impl Future<Output = DatabaseResult<User>> + Send;
    /// Update a user in database storage.
    fn update_user(&self, user: &UpdateUser) -> impl Future<Output = DatabaseResult<()>> + Send;
    /// Remove a user from database storage.
    fn remove_user(&self, user: &UserKey) -> impl Future<Output = DatabaseResult<()>> + Send;
    /// Search for users with search criteria in `UserSearch` from
    /// database storage.
    fn search_users(
        &self,
        user: &UserSearch,
    ) -> impl Future<Output = DatabaseResult<Vec<User>>> + Send;
    /// Count the number of users grouping by gender.
    fn count_genders(&self) -> impl Future<Output = Result<Vec<Value>, DatabaseError>> + Send;
    /// Download all users as a stream.
    fn download(
        &self,
    ) -> impl Future<
        Output = DatabaseResult<impl Stream<Item = DatabaseResult<User>> + 'static + Send>,
    > + Send;
}

/// Abstract our database API so it can be swapped out
/// for any backend.
#[async_trait]
pub trait UserDatabaseDynSafe: Send + Sync + Debug {
    /// Lookup a user from database storage.
    async fn get_user(&self, id: &UserKey) -> DatabaseResult<Option<User>>;
    /// Save a user to database storage.
    async fn save_user(&self, user: &User) -> DatabaseResult<User>;
    /// Update a user in database storage.
    async fn update_user(&self, user: &UpdateUser) -> DatabaseResult<()>;
    /// Remove a user from database storage.
    async fn remove_user(&self, user: &UserKey) -> DatabaseResult<()>;
    /// Search for users with search criteria in `UserSearch` from
    /// database storage.
    async fn search_users(&self, user: &UserSearch) -> DatabaseResult<Vec<User>>;
    /// Count the number of users grouping by gender.
    async fn count_genders(&self) -> Result<Vec<Value>, DatabaseError>;
}

/// Database errors.
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Mongodb error: `{0}`")]
    MongoError(#[from] mongodb::error::Error),
    #[error("Database Test Failure")]
    TestError,
    #[error("Bson error: `{0}`")]
    BsonError(#[from] mongodb::bson::oid::Error),
}
