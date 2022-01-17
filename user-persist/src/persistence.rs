use crate::types::{UpdateUser, User, UserKey, UserSearch};
use async_trait::async_trait;
use std::fmt;
use thiserror::Error;
use serde_json::Value;

pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Abstract our persistence API so it can be swapped out
/// for any backend.
#[async_trait]
pub trait UserPersistence: Send + Sync + fmt::Debug {
  /// Lookup a user from persistent storage.
  async fn get_user(&self, id: &UserKey) -> PersistenceResult<Option<User>>;
  /// Save a user to persistent storage.
  async fn save_user(&self, user: &User) -> PersistenceResult<User>;
  /// Update a user in persistent storage.
  async fn update_user(&self, user: &UpdateUser) -> PersistenceResult<()>;
  /// Search for users with search critera in `UserSearch` from
  /// peristent storage.
  async fn search_users(
    &self,
    user: &UserSearch,
  ) -> PersistenceResult<Vec<User>>;
  /// Count the number of users grouping by gender.
  async fn count_genders(&self) -> Result<Vec<Value>, PersistenceError>;
  // async fn download<S: Stream<Item = MongoUser>>(
  //   &self,
  // ) -> PersistenceResult<S>;
  // async fn download(
  //   &self,
  // ) -> PersistenceResult<Pin<Box<dyn Stream<Item = MongoUser>>>>;
}

/// Enumeration of persistence errors.
#[derive(Error, Debug)]
pub enum PersistenceError {
  #[error("Mongodb error")]
  MongoError(#[from] mongodb::error::Error),
  #[error("Persistence Test Failure")]
  TestError,
  #[error("Bson error")]
  BsonError(#[from] mongodb::bson::oid::Error)
}
