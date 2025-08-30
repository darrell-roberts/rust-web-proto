/*!
This module provides data access to a a mongodb user collection.

This implements a dyn safe trait. This is needed for actix-web and rocket that don't
support generics in the route functions.
*/
use crate::{
    database::{DatabaseError, DatabaseResult, UserDatabase, UserDatabaseDynSafe},
    types::{UpdateUser, User, UserKey, UserSearch},
};
use serde_json::Value;
use std::{future::Future, pin::Pin};
use tracing::instrument;

// For all types that implement the non dyn safe we proxy and wrap in a dyn safe implementation.
impl<T: UserDatabase> UserDatabaseDynSafe for T {
    fn get_user<'a>(
        &'a self,
        id: &'a UserKey,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<Option<User>>> + 'a + Send>> {
        Box::pin(UserDatabase::get_user(self, id))
    }

    fn save_user<'a>(
        &'a self,
        user: &'a User,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<User>> + 'a + Send>> {
        Box::pin(UserDatabase::save_user(self, user))
    }

    fn update_user<'a>(
        &'a self,
        user: &'a UpdateUser,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<()>> + 'a + Send>> {
        Box::pin(UserDatabase::update_user(self, user))
    }

    fn remove_user<'a>(
        &'a self,
        key: &'a UserKey,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<()>> + 'a + Send>> {
        Box::pin(UserDatabase::remove_user(self, key))
    }

    #[instrument(skip_all, level = "debug", target = "database", name = "search-span")]
    fn search_users<'a>(
        &'a self,
        user_search: &'a UserSearch,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<Vec<User>>> + 'a + Send>> {
        Box::pin(UserDatabase::search_users(self, user_search))
    }

    fn count_genders(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, DatabaseError>> + '_ + Send>> {
        Box::pin(UserDatabase::count_genders(self))
    }
}
