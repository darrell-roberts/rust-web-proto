use crate::PERSISTENCE_TARGET;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use validator::{Validate, ValidationError};

// Similar to a Sum type.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Gender {
  Male,
  Female,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Email(pub String);

fn is_valid_email(email: &Email) -> bool {
  lazy_static! {
    static ref RE: Regex =
      Regex::new(r"[a-zA-Z0-9+._-]+@[a-zA-Z-]+\.[a-z]+").unwrap();
  }
  RE.is_match(&email.0)
}

#[instrument(target = "persistence")]
fn validate_email(email: &Email) -> Result<(), ValidationError> {
  event!(
    target: PERSISTENCE_TARGET,
    Level::DEBUG,
    "validating email {}",
    email.0
  );
  if is_valid_email(email) {
    Ok(())
  } else {
    Err(ValidationError::new("invalid email"))
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserKey(pub String);

pub struct ParseUserKeyError;

impl std::str::FromStr for UserKey {
  type Err = ParseUserKeyError;
  fn from_str(s: &str) -> Result<UserKey, ParseUserKeyError> {
    if s.is_empty() {
      Err(ParseUserKeyError)
    } else {
      Ok(UserKey(s.to_string()))
    }
  }
}

// Similar to a Product type with record syntax.
#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct User {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  pub name: String,
  #[validate(range(min = 100))]
  pub age: u32,
  #[validate(custom = "validate_email")]
  pub email: Email,
  pub gender: Gender,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct UpdateUser {
  pub id: UserKey,
  pub name: String,
  #[validate(range(min = 100))]
  pub age: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct UserSearch {
  #[validate(custom = "validate_email")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub email: Option<Email>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub gender: Option<Gender>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
}
