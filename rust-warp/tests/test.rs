use async_trait::async_trait;
use flate2::read::GzDecoder;
use rust_warp::filters::user;
use serde_json::{from_str, json, Value};
use std::{
  convert::Infallible,
  fmt::Debug,
  io::Read,
  sync::{Arc, Once},
};
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_persist::persistence::PersistenceResult;
use user_persist::{
  persistence::{PersistenceError, UserPersistence},
  types::{Email, Gender, UpdateUser, User, UserKey, UserSearch},
};
use warp::{hyper::body::Bytes, Filter, Reply};

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

  async fn remove_user(&self, user: &UserKey) -> PersistenceResult<()> {
    todo!()
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

fn test_user_filter(
) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
  init_log();
  let test_db = Arc::new(TestPersistence);
  user(test_db)
}

fn decompress_body(b: Bytes) -> String {
  let mut decoder = GzDecoder::new(b.as_ref());
  let mut s = String::new();
  decoder.read_to_string(&mut s).unwrap();
  s
}

#[tokio::test]
async fn test_get_user() {
  let filter = test_user_filter();
  let res = warp::test::request()
    .path("/api/v1/user/61c0d1954c6b974ca7000000")
    .reply(&filter)
    .await
    .map(decompress_body)
    .map(|b| from_str::<Value>(&b).unwrap());

  let body = res.body();
  event!(target: TEST_TARGET, Level::DEBUG, "body: {:?}", body);
  assert_eq!(res.status(), 200, "status is ok");
  assert_eq!(
    res.into_body(),
    json! ({
      "name": "Test User",
      "age":100,
      "email":"test@test.com",
      "gender":"Male"
    })
  )
}

// Bad bson. Filter won't route to handler.
#[tokio::test]
async fn test_get_user_404() {
  let filter = test_user_filter();
  let res = warp::test::request()
    .path("/api/v1/user/abc")
    .reply(&filter)
    .await
    .map(decompress_body);

  event!(target: TEST_TARGET, Level::DEBUG, "Body: {:?}", res.body());

  assert_eq!(res.status(), 404);
}

// Good bson. Does not find result.
#[tokio::test]
async fn test_get_user_no_user() {
  let filter = test_user_filter();
  let res = warp::test::request()
    .path("/api/v1/user/61c0e3c94c6b977028000000")
    .reply(&filter)
    .await
    .map(decompress_body);

  event!(target: TEST_TARGET, Level::DEBUG, "Body: {:?}", res.body());

  assert_eq!(res.status(), 404);
}
