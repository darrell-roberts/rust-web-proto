//! This module provides data access to a a mongodb user collection.
//!
//! This implements a dyn safe trait. This is needed for actix-web and rocket that don't
//! support generics in the route functions.
use crate::{
    database::{
        DatabaseError, DatabaseResult, PinBoxFuture, PinBoxStream, UserDatabase,
        UserDatabaseDynSafe,
    },
    types::{UpdateUser, User, UserKey, UserSearch},
};
use futures::{FutureExt, StreamExt};
use serde_json::Value;
use tracing::instrument;

// For all types that implement the non dyn safe we proxy and wrap in a dyn safe implementation.
impl<T: UserDatabase> UserDatabaseDynSafe for T {
    fn get_user<'a>(&'a self, id: &'a UserKey) -> PinBoxFuture<'a, DatabaseResult<Option<User>>> {
        Box::pin(T::get_user(self, id))
    }

    fn save_user<'a>(&'a self, user: &'a User) -> PinBoxFuture<'a, DatabaseResult<User>> {
        Box::pin(T::save_user(self, user))
    }

    fn update_user<'a>(&'a self, user: &'a UpdateUser) -> PinBoxFuture<'a, DatabaseResult<()>> {
        Box::pin(T::update_user(self, user))
    }

    fn remove_user<'a>(&'a self, key: &'a UserKey) -> PinBoxFuture<'a, DatabaseResult<()>> {
        Box::pin(T::remove_user(self, key))
    }

    #[instrument(skip_all, level = "debug", target = "database", name = "search-span")]
    fn search_users<'a>(
        &'a self,
        user_search: &'a UserSearch,
    ) -> PinBoxFuture<'a, DatabaseResult<Vec<User>>> {
        Box::pin(T::search_users(self, user_search))
    }

    fn count_genders(&self) -> PinBoxFuture<'_, Result<Vec<Value>, DatabaseError>> {
        Box::pin(T::count_genders(self))
    }

    fn download(&self) -> PinBoxFuture<'_, PinBoxStream<DatabaseResult<User>>> {
        Box::pin(T::download(self).map(StreamExt::boxed))
    }
}
