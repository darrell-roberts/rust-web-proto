//! User database types.
use mongodb::bson::oid::ObjectId;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    ops::Deref,
    sync::LazyLock,
};
use tracing::debug;
use validator::{Validate, ValidationError};

/// User Gender
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

impl Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Gender::Male => "Male",
                Gender::Female => "Female",
            }
        )
    }
}

/// Email newtype.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Email(pub String);

impl Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", mask_str(self))
    }
}

impl Deref for Email {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

static RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9+._-]+@[a-zA-Z-]+\.[a-z]+").unwrap());

impl Email {
    /// Validate email.
    fn is_valid(&self) -> bool {
        RE.is_match(self)
    }
}

/// Email validator.
fn validate_email(email: &Email) -> Result<(), ValidationError> {
    debug!("validating email {email}");
    if email.is_valid() {
        Ok(())
    } else {
        Err(ValidationError::new("invalid email"))
    }
}

/// User primary key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UserKey(pub String);

impl Deref for UserKey {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for UserKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ObjectId> for UserKey {
    fn from(oid: ObjectId) -> Self {
        Self(oid.to_string())
    }
}

/// Key error.
#[derive(Debug)]
pub struct InvalidKeyError;

impl std::str::FromStr for UserKey {
    type Err = InvalidKeyError;
    fn from_str(s: &str) -> Result<UserKey, InvalidKeyError> {
        if s.is_empty() {
            Err(InvalidKeyError)
        } else {
            Ok(UserKey(s.to_string()))
        }
    }
}

/// User type.
#[derive(Clone, Debug, Deserialize, Serialize, Validate, PartialEq, Eq)]
pub struct User {
    /// User id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<UserKey>,
    /// User name.
    pub name: String,
    /// User age.
    #[validate(range(min = 100))]
    pub age: u32,
    /// User email.
    #[validate(custom(function = "validate_email"))]
    pub email: Email,
    /// User gender.
    pub gender: Gender,
}

/// Mask a string value showing only the first and last character and
/// masking the rest.
fn mask_str(str: &str) -> String {
    let head = str.chars().next().unwrap_or_default();
    let last = str.chars().last().unwrap_or_default();
    let mask_chars_len = if str.len() > 3 { str.len() - 2 } else { 1 };
    let mask_chars = "*".repeat(mask_chars_len);

    format!("{head}{mask_chars}{last}")
}

impl Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", mask_str(&self.name), mask_str(&self.email))
    }
}

/// Request type to update a user record.
#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct UpdateUser {
    /// User id.
    pub id: UserKey,
    /// User name.
    pub name: String,
    /// User email.
    #[validate(custom(function = "validate_email"))]
    pub email: Email,
    /// User age.
    #[validate(range(min = 100))]
    pub age: u32,
    /// User hash.
    pub hid: String,
}

impl Display for UpdateUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.id, mask_str(&self.name), self.age)
    }
}

/// Request type for user search.
#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct UserSearch {
    #[validate(custom(function = "validate_email"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Display for UserSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"email = "{}", gender = "{}", name = "{}""#,
            self.email.as_ref().map(|s| mask_str(s)).unwrap_or_default(),
            self.gender
                .as_ref()
                .map(|g| format!("{g}"))
                .unwrap_or_default(),
            self.name.as_ref().map(|s| mask_str(s)).unwrap_or_default()
        )
    }
}

#[cfg(test)]
mod test {
    use super::{Email, User};
    use crate::types::Gender;

    #[test]
    fn test_deserialize_user() {
        let json_user = r#"{
      "name": "Scenario User",
      "email": "scenario@test.com",
      "age": 20,
      "gender": "Female"
    }"#;

        let user = serde_json::from_str::<User>(json_user).unwrap();
        assert_eq!(
            user,
            User {
                id: None,
                name: "Scenario User".into(),
                email: Email("scenario@test.com".into()),
                age: 20,
                gender: Gender::Female
            }
        );
    }
}
