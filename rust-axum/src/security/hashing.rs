//! Provides hashing capabilities for API validation.
use axum::response::{IntoResponse, Json, Response};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::fmt::Formatter;
use tracing::debug;
use user_persist::types::{UpdateUser, User};
use user_persist::{Validate, ValidationErrors};

/// A type that can be converted into a hash.
pub trait Hashable {
    type Hashed: Serialize + IntoResponse;
    fn hash(&self, hash_prefix: &str) -> Self::Hashed;
}

/// A hashed type that validates its hash.
pub trait HashValidating {
    fn is_valid(&self, hash_prefix: &str) -> bool;
}

/// Create a sha 256 hash of the provided string
/// and return the hash as a base64 encoded string.
fn hash_value(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    base64::engine::general_purpose::URL_SAFE.encode(hasher.finalize())
}

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
        debug!(target: super::HASHING_TARGET, "computed hash: {new_hash}");
        new_hash == self.hid
    }
}

impl Hashable for User {
    type Hashed = HashedUser;

    fn hash(&self, hash_prefix: &str) -> Self::Hashed {
        HashedUser {
            user: self.clone(),
            hid: hash_value(&format!("{hash_prefix}{}{}", self.name, self.email.0)),
        }
    }
}

impl<T> Hashable for Vec<T>
where
    T: Hashable,
    Vec<<T as Hashable>::Hashed>: IntoResponse,
{
    type Hashed = Vec<T::Hashed>;
    fn hash(&self, hash_prefix: &str) -> Self::Hashed {
        self.iter().map(|t| t.hash(hash_prefix)).collect::<Vec<_>>()
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
    use super::Hashable;
    use user_persist::types::{Email, Gender, User};
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

        print!("hashed user: {}", serde_json::to_string(&hashed).unwrap());
        assert_eq!(
            hashed.hid,
            "0HBmtxUP3a38op1YHscpgdAPjyRDkHq89bzPnk8ibDo=".to_owned()
        );
    }
}
