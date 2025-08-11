//! Provides hashing capabilities for API validation.
use axum::response::{IntoResponse, Json, Response};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};
use tracing::debug;
use user_database::{
    types::{UpdateUser, User},
    Validate, ValidationErrors,
};

/// A type that can be converted into a type with a hash.
pub trait IntoTypeWithHash {
    /// The hashed type this converts into.
    type Hashed: Serialize;
    /// Create a hash from self and consume into a new hashed type.
    fn hash(self, hash_prefix: &str) -> Self::Hashed;
}

/// A hashed type that validates its hash.
pub trait HashValidating {
    /// Checks if the payload has been tampered with.
    fn is_valid(&self, hash_prefix: &str) -> bool;
}

/// Create a sha 256 hash of the provided string
/// and return the hash as a base64 encoded string.
fn hash_value(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    URL_SAFE.encode(hasher.finalize())
}

/// A User type that now has a hash.
#[derive(Debug, Serialize, Deserialize)]
pub struct HashedUser {
    #[serde(flatten)]
    pub user: User,
    pub hid: String,
}

impl Display for HashedUser {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "hid: {}, {}", self.hid, self.user)
    }
}

impl IntoResponse for HashedUser {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl HashValidating for HashedUser {
    fn is_valid(&self, hash_prefix: &str) -> bool {
        let new_hash = hash_value(&format!(
            "{hash_prefix}{}{}",
            self.user.name, self.user.email
        ));
        new_hash == self.hid
    }
}

impl Validate for HashedUser {
    fn validate(&self) -> Result<(), ValidationErrors> {
        self.user.validate()
    }
}

impl HashValidating for UpdateUser {
    fn is_valid(&self, hash_prefix: &str) -> bool {
        let new_hash = hash_value(&format!("{hash_prefix}{}{}", self.name, self.email.0));
        debug!("computed hash: {new_hash}");
        new_hash == self.hid
    }
}

impl IntoTypeWithHash for User {
    type Hashed = HashedUser;

    fn hash(self, hash_prefix: &str) -> Self::Hashed {
        HashedUser {
            hid: hash_value(&format!("{hash_prefix}{}{}", self.name, self.email.0)),
            user: self,
        }
    }
}

impl<T> IntoTypeWithHash for Vec<T>
where
    T: IntoTypeWithHash,
{
    type Hashed = Vec<T::Hashed>;

    fn hash(self, hash_prefix: &str) -> Self::Hashed {
        self.into_iter()
            .map(|t| t.hash(hash_prefix))
            .collect::<Vec<_>>()
    }
}

/*
// Alternative to middleware
pub struct HashingResponse<T: Hashable> {
    payload: T,
    config: Arc<AppConfig>,
}

impl<T: Hashable> HashingResponse<T> {
    pub fn new(config: Arc<AppConfig>, payload: T) -> Self {
        Self { payload, config }
    }
}

impl<T: Hashable> IntoResponse for HashingResponse<T> {
    fn into_response(self) -> Response {
        let hashed = self.payload.hash(self.config.hash_prefix());
        hashed.into_response()
    }
}

/// Newtype to implement IntoResponse trait for Vec<T: Hashable>.
pub struct HashableVector<T: Hashable> {
    payload: Vec<T>,
    config: Arc<AppConfig>,
}

impl<T: Hashable> HashableVector<T> {
    pub fn new(config: Arc<AppConfig>, payload: Vec<T>) -> Self {
        Self { config, payload }
    }
}

impl<T: Hashable> IntoResponse for HashableVector<T> {
    fn into_response(self) -> Response {
        let hashed = self
            .payload
            .iter()
            .map(|d| d.hash(self.config.hash_prefix()))
            .collect::<Vec<_>>();
        (StatusCode::OK, Json(hashed)).into_response()
    }
}
*/

#[cfg(test)]
mod test {
    use super::IntoTypeWithHash;
    use user_database::types::{Email, Gender, User};

    #[test]
    fn test_hash_user() {
        let user = User {
            id: None,
            name: "Test User".to_owned(),
            age: 100,
            email: Email("test@user.com".to_owned()),
            gender: Gender::Male,
        };

        let hashed = user.hash("some_prefix");
        assert_eq!(hashed.hid, "0HBmtxUP3a38op1YHscpgdAPjyRDkHq89bzPnk8ibDo=");
    }
}
