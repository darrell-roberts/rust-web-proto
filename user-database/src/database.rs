//! Generic UserDatabase Trait and types.
use crate::types::{UpdateUser, User, UserKey, UserSearch};
use futures::Stream;
use serde_json::Value;
use std::{fmt::Debug, future::Future, pin::Pin};
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
    ) -> impl Future<Output = impl Stream<Item = DatabaseResult<User>> + 'static + Send> + '_ + Send;
}

/// Abstract our database API so it can be swapped out
/// for any backend.
pub trait UserDatabaseDynSafe: Send + Sync + Debug {
    /// Lookup a user from database storage.
    fn get_user<'a>(
        &'a self,
        id: &'a UserKey,
    ) -> PinBox<dyn Future<Output = DatabaseResult<Option<User>>> + 'a + Send>;

    /// Save a user to database storage.
    fn save_user<'a>(
        &'a self,
        user: &'a User,
    ) -> PinBox<dyn Future<Output = DatabaseResult<User>> + 'a + Send>;

    /// Update a user in database storage.
    fn update_user<'a>(
        &'a self,
        user: &'a UpdateUser,
    ) -> PinBox<dyn Future<Output = DatabaseResult<()>> + 'a + Send>;

    /// Remove a user from database storage.
    fn remove_user<'a>(
        &'a self,
        user: &'a UserKey,
    ) -> PinBox<dyn Future<Output = DatabaseResult<()>> + 'a + Send>;

    /// Search for users with search criteria in `UserSearch` from
    /// database storage.
    fn search_users<'a>(
        &'a self,
        user: &'a UserSearch,
    ) -> PinBox<dyn Future<Output = DatabaseResult<Vec<User>>> + 'a + Send>;

    /// Count the number of users grouping by gender.
    fn count_genders(&self) -> PinBox<dyn Future<Output = DatabaseResult<Vec<Value>>> + '_ + Send>;

    /// Download all user records
    fn download(&self) -> PinBox<dyn Future<Output = PinnedUserStream> + '_ + Send>;
}

pub type PinBox<T> = Pin<Box<T>>;

pub type PinnedUserStream = Pin<Box<dyn Stream<Item = DatabaseResult<User>> + 'static + Send>>;

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
