use super::filters::{get_user, user};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{Arc, Once};
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_persist::persistence::{UserPersistence, PersistenceError};
use user_persist::types::{
  Email, Gender, UpdateUser, User, UserKey, UserSearch,
};

const TEST_TARGET: &str = "test";

static INIT: Once = Once::new();

fn init_log() {
  INIT.call_once(|| {
    tracing_subscriber::fmt()
      .with_env_filter(EnvFilter::from_default_env())
      .with_target(true)
      .pretty()
      .init();
  });
}

#[derive(Debug, Clone)]
pub struct TestPersistence;

fn test_user() -> User {
  User {
    id: None,
    name: String::from("Test User"),
    email: Email(String::from("test@test.com")),
    age: 100,
    gender: Gender::Male,
  }
}

// A mock persistence for testing.
#[async_trait]
impl UserPersistence for TestPersistence {
  async fn get_user(
    &self,
    id: &UserKey,
  ) -> Result<Option<User>, PersistenceError> {
    if id.0 == "61c0d1954c6b974ca7000000" {
      Ok(Some(test_user()))
    } else {
      Ok(None)
    }
  }

  async fn save_user(&self, user: &User) -> Result<User, PersistenceError> {
    Ok(user.clone())
  }

  async fn update_user(
    &self,
    _user: &UpdateUser,
  ) -> Result<(), PersistenceError> {
    Ok(())
  }

  async fn search_users(
    &self,
    _user_search: &UserSearch,
  ) -> Result<Vec<User>, PersistenceError> {
    Ok(vec![test_user()])
  }

  async fn count_genders(&self) -> Result<Vec<Value>, PersistenceError> {
    Err(PersistenceError::TestError)
  }
}

#[tokio::test]
async fn test_get_user() {
  init_log();
  let test_db = Arc::new(TestPersistence);
  let filter = user(test_db);
  let res = warp::test::request()
    .path("/61c0d1954c6b974ca7000000")
    .json(&true)
    .reply(&filter)
    .await;

  event!(target: TEST_TARGET, Level::DEBUG, "body: {:?}", res.body());
  assert_eq!(res.status(), 200);
  assert_eq!(
    res.body(),
    r#"{ "id":null,
         "name":"Test User",
          "age":100,
          "email":"test@test.com",
          "gender":"Male"
       }"#
  )
}

// Bad bson. Filter won't route to handler.
#[tokio::test]
async fn test_get_user_404() {
  init_log();
  let test_db = Arc::new(TestPersistence);
  let filter = get_user(test_db);
  let res = warp::test::request()
    .path("/61c0d1954c6b974ca7000000a")
    .json(&true)
    .reply(&filter)
    .await;

  assert_eq!(res.status(), 404);
}

// Good bson. Does not find result.
#[tokio::test]
async fn test_get_user_no_user() {
  init_log();
  let test_db = Arc::new(TestPersistence);
  let filter = get_user(test_db);
  let res = warp::test::request()
    .path("/61c0e3c94c6b977028000000")
    .json(&true)
    .reply(&filter)
    .await;

  event!(target: TEST_TARGET, Level::DEBUG, "Bytes: {:?}", res.body());

  assert_eq!(res.status(), 200);
  assert_eq!(res.body(), "null");
}
