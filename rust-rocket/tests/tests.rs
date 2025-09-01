//! Route integration tests.
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use rocket::{
    http::{ContentType, Header, Status},
    local::blocking::Client,
    Build, Rocket,
};
use rust_rocket::{
    types::{JWTClaims, Role},
    TEST_JWT_SECRET,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::sync::{Arc, Once};
use thiserror::Error;
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_database::{
    database::{DatabaseResult, UserDatabase},
    types::{Email, Gender, UpdateUser, User, UserKey, UserSearch},
};

fn get_rocket() -> Rocket<Build> {
    rust_rocket::rocket(Arc::new(TestDatabase))
}

const TEST_TARGET: &str = "test";

static INIT: Once = Once::new();

#[derive(Debug, Error)]
enum TestError {
    #[error("Test failed")]
    RocketError {
        #[from]
        source: Box<rocket::error::Error>,
    },
    #[error("Serialization failed")]
    SerializeError {
        #[from]
        source: serde_json::Error,
    },
}

type TestResult<T> = Result<T, TestError>;

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
    async fn get_user(&self, id: &UserKey) -> DatabaseResult<Option<User>> {
        Ok((id.0 == "61c0d1954c6b974ca7000000").then(test_user))
    }

    async fn save_user(&self, user: &User) -> DatabaseResult<User> {
        Ok(user.clone())
    }

    async fn update_user(&self, _user: &UpdateUser) -> DatabaseResult<()> {
        Ok(())
    }

    async fn remove_user(&self, _user: &UserKey) -> DatabaseResult<()> {
        todo!()
    }

    async fn search_users(&self, _user_search: &UserSearch) -> DatabaseResult<Vec<User>> {
        Ok(vec![test_user()])
    }

    async fn count_genders(&self) -> DatabaseResult<Vec<Value>> {
        Ok(vec![
            json! (   {
                "_id": "Male",
                "count": 6
            }),
            json!({
                "_id": "Female",
                "count": 12
            }),
        ])
    }

    async fn download(&self) -> impl futures::Stream<Item = DatabaseResult<User>> + 'static + Send {
        futures::stream::iter([
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

// Setup tracing first.
fn init_log() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_target(true)
            .pretty()
            .init();
    });
}

type HmacSha256 = Hmac<Sha256>;

fn test_jwt(role: Role) -> String {
    let key = HmacSha256::new_from_slice(TEST_JWT_SECRET).unwrap();
    let expiration = Utc::now() + Duration::minutes(5);
    let claims = JWTClaims {
        sub: "somebody".to_owned(),
        role,
        exp: expiration.timestamp(),
    };
    format!("Bearer {}", claims.sign_with_key(&key).unwrap())
}

fn test_jwt_expired(role: Role) -> String {
    let key = HmacSha256::new_from_slice(TEST_JWT_SECRET).unwrap();
    let expiration = Utc::now() - Duration::minutes(5);
    let claims = JWTClaims {
        sub: "somebody".to_owned(),
        role,
        exp: expiration.timestamp(),
    };
    format!("Bearer {}", claims.sign_with_key(&key).unwrap())
}

// Call get user with Admin role and valid user.
#[test]
fn get_user() -> TestResult<()> {
    init_log();
    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let response = client
        .get("/api/v1/user/61c0d1954c6b974ca7000000")
        .header(Header::new("Authorization", test_jwt(Role::Admin)))
        .dispatch();

    let status = response.status();
    let body = response.into_string().unwrap_or_default();
    event!(target: TEST_TARGET, Level::DEBUG, "response: {body}");
    assert_eq!(status, Status::Ok);
    Ok(())
}

// Call get user with User role and valid user.
#[test]
fn get_user_invalid_access() -> TestResult<()> {
    init_log();

    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let response = client
        .get("/api/v1/user/61c0d1954c6b974ca7000000")
        .header(Header::new("Authorization", test_jwt(Role::User)))
        .dispatch();

    let status = response.status();
    let body = response.into_string().unwrap_or_default();
    event!(target: TEST_TARGET, Level::DEBUG, "response: {body}");
    assert_eq!(status, Status::Forbidden);
    Ok(())
}

// Call get user with User role and valid user but with a jwt that has expired
#[test]
fn get_user_invalid_access_expired_claim() -> TestResult<()> {
    init_log();

    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let response = client
        .get("/api/v1/user/61c0d1954c6b974ca7000000")
        .header(Header::new("Authorization", test_jwt_expired(Role::User)))
        .dispatch();

    let status = response.status();
    let body = response.into_string().unwrap_or_default();
    event!(target: TEST_TARGET, Level::DEBUG, "response: {body}");
    assert_eq!(status, Status::Forbidden);
    Ok(())
}

#[test]
fn save_user() -> TestResult<()> {
    init_log();
    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let json_user = serde_json::to_string(&test_user())?;

    event!(target: TEST_TARGET, Level::DEBUG, "json_user: {json_user}");

    let response = client
        .post("/api/v1/user")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", test_jwt(Role::User)))
        .body(json_user)
        .dispatch();

    assert_eq!(response.status(), Status::Ok);
    Ok(())
}

#[test]
fn save_user_rejection() -> TestResult<()> {
    init_log();
    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let response = client
        .post("/api/v1/user")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", test_jwt(Role::User)))
        .body(
            r#"{
    "name": "Test User",
    "age": 5,
    "email": "bad-email-value",
    "gender": "Male"
  }"#,
        )
        .dispatch();

    let status = response.status();
    let body = response.into_string().unwrap_or_default();

    event!(target: TEST_TARGET, Level::DEBUG, "result {body}");

    let validation_errors = serde_json::from_str::<Value>(&body)?;

    event!(
      target: TEST_TARGET,
      Level::DEBUG,
      "json errors {validation_errors:?}"
    );

    let email_validation_code = validation_errors
        .get("validation")
        .and_then(|v| v.get("email"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("code"));

    let age_validation_code = validation_errors
        .get("validation")
        .and_then(|v| v.get("age"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("code"));

    assert_eq!(status, Status::BadRequest);
    assert_eq!(email_validation_code, Some(json!("invalid email")).as_ref());
    assert_eq!(age_validation_code, Some(json!("range")).as_ref());

    Ok(())
}

#[test]
fn search_users() -> TestResult<()> {
    init_log();
    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let users_search = UserSearch {
        email: Some(Email("test@somewhere.com".to_owned())),
        gender: None,
        name: None,
    };
    let response = client
        .post("/api/v1/user/search")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", test_jwt(Role::Admin)))
        .body(serde_json::to_string(&users_search)?)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    Ok(())
}

#[test]
fn count_genders() -> TestResult<()> {
    init_log();
    let client = Client::tracked(get_rocket()).map_err(Box::new)?;
    let response = client
        .get("/api/v1/user/counts")
        .header(Header::new("Authorization", test_jwt(Role::User)))
        .dispatch();

    let status = response.status();
    let body = response.into_string().unwrap_or_default();

    event!(target: TEST_TARGET, Level::DEBUG, "body: {body}");

    assert_eq!(status, Status::Ok);

    Ok(())
}

#[test]
fn test_download() -> TestResult<()> {
    init_log();
    let client = Client::tracked(get_rocket()).map_err(Box::new)?;

    let response = client
        .get("/api/v1/user/download")
        .header(Header::new("Authorization", test_jwt(Role::Admin)))
        .dispatch();

    let status = response.status();
    let body = response.into_string().unwrap_or_default();

    event!(target: TEST_TARGET, Level::DEBUG, "body: {body}");

    assert_eq!(status, Status::Ok);
    assert_eq!(
        body,
        r#"[{"id":"key1","name":"Test User 1","age":100,"email":"test1@test.com","gender":"Male"},{"id":"key2","name":"Test User 2","age":100,"email":"test2@test.com","gender":"Male"},{"id":"key3","name":"Test User 3","age":100,"email":"test3@test.com","gender":"Male"}]"#
    );

    Ok(())
}
