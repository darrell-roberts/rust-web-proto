use flate2::read::GzDecoder;
use futures::stream;
use rust_warp::filters::user;
use serde_json::{from_str, json, Value};
use std::{
    convert::Infallible,
    fmt::Debug,
    io::Read,
    sync::{Arc, Once},
};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use user_database::{
    database::{DatabaseError, DatabaseResult, UserDatabase},
    types::{Email, Gender, UpdateUser, User, UserKey, UserSearch},
};
use warp::{hyper::body::Bytes, Filter, Reply};

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
pub struct TestDatabase;

fn test_user() -> User {
    User {
        id: None,
        name: String::from("Test User"),
        email: Email(String::from("test@test.com")),
        age: 100,
        gender: Gender::Male,
    }
}

// A mock database for testing.
impl UserDatabase for TestDatabase {
    async fn get_user(&self, id: &UserKey) -> Result<Option<User>, DatabaseError> {
        if id.0 == "61c0d1954c6b974ca7000000" {
            Ok(Some(test_user()))
        } else {
            Ok(None)
        }
    }

    async fn save_user(&self, user: &User) -> Result<User, DatabaseError> {
        Ok(user.clone())
    }

    async fn update_user(&self, _user: &UpdateUser) -> Result<(), DatabaseError> {
        Ok(())
    }

    async fn remove_user(&self, _user: &UserKey) -> DatabaseResult<()> {
        todo!()
    }

    async fn search_users(&self, _user_search: &UserSearch) -> Result<Vec<User>, DatabaseError> {
        Ok(vec![test_user()])
    }

    async fn count_genders(&self) -> Result<Vec<Value>, DatabaseError> {
        Err(DatabaseError::TestError)
    }

    async fn download(&self) -> impl futures::Stream<Item = DatabaseResult<User>> + 'static {
        stream::iter([
            Ok(User {
                id: Some(UserKey("key1".into())),
                name: "Test User 1".into(),
                age: 100,
                email: Email("test1@test.com".into()),
                gender: Gender::Male,
            }),
            Ok(User {
                id: Some(UserKey("key2".into())),
                name: "Test User 2".into(),
                age: 100,
                email: Email("test2@test.com".into()),
                gender: Gender::Male,
            }),
            Ok(User {
                id: Some(UserKey("key3".into())),
                name: "Test User 3".into(),
                age: 100,
                email: Email("test3@test.com".into()),
                gender: Gender::Male,
            }),
        ])
    }
}

fn test_user_filter() -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    init_log();
    user(Arc::new(TestDatabase))
}

fn decompress_body(b: Bytes) -> String {
    let mut decoder = GzDecoder::new(b.as_ref());
    let mut s = String::new();
    decoder.read_to_string(&mut s).unwrap();
    s
}

/// Test get user route.
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
    debug!("body: {:?}", body);
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

/// Bad bson. Filter won't route to handler.
#[tokio::test]
async fn test_get_user_404() {
    let filter = test_user_filter();
    let res = warp::test::request()
        .path("/api/v1/user/abc")
        .reply(&filter)
        .await
        .map(decompress_body);

    debug!("Body: {:?}", res.body());
    assert_eq!(res.status(), 404);
}

/// Good bson. Does not find result.
#[tokio::test]
async fn test_get_user_no_user() {
    let filter = test_user_filter();
    let res = warp::test::request()
        .path("/api/v1/user/61c0e3c94c6b977028000000")
        .reply(&filter)
        .await
        .map(decompress_body);

    debug!("Body: {:?}", res.body());
    assert_eq!(res.status(), 404);
}
