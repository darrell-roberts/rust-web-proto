use crate::{
    catchers, fairings, routes,
    types::{JWTClaims, Role},
    TEST_JWT_SECRET,
};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use rocket::{
    http::{ContentType, Header, Status},
    local::blocking::Client,
    Build, Rocket,
};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Once},
};
use thiserror::Error;
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_database::{
    database::{DatabaseError, DatabaseResult, UserDatabaseDynSafe},
    types::{Email, Gender, UpdateUser, User, UserKey, UserSearch},
};

const USER_PATH: &str = "/api/v1/user";

fn get_rocket() -> Rocket<Build> {
    let mongo_pesist: Arc<dyn UserDatabaseDynSafe> = Arc::new(TestDatabase);
    rocket::build()
        .manage(mongo_pesist)
        .attach(fairings::RequestIdFairing)
        .attach(fairings::LoggerFairing)
        .attach(fairings::RequestTimer)
        .mount(
            USER_PATH,
            routes![
                routes::count_genders,
                routes::get_user,
                routes::save_user,
                routes::find_users,
                routes::update_user,
                // routes::download
            ],
        )
        .register(
            USER_PATH,
            catchers![
                catchers::not_found,
                catchers::bad_request,
                catchers::unprocessable_entry,
                catchers::internal_server_error
            ],
        )
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
impl UserDatabaseDynSafe for TestDatabase {
    fn get_user<'a>(
        &'a self,
        id: &'a UserKey,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<Option<User>>> + 'a + Send>> {
        Box::pin(async {
            if id.0 == "61c0d1954c6b974ca7000000" {
                Ok(Some(test_user()))
            } else {
                Ok(None)
            }
        })
    }

    fn save_user<'a>(
        &'a self,
        user: &'a User,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<User>> + 'a + Send>> {
        Box::pin(async { Ok(user.clone()) })
    }

    fn update_user<'a>(
        &'a self,
        _user: &'a UpdateUser,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<()>> + 'a + Send>> {
        Box::pin(async { Ok(()) })
    }

    fn remove_user<'a>(
        &'a self,
        _user: &'a UserKey,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<()>> + 'a + Send>> {
        todo!()
    }

    fn search_users<'a>(
        &'a self,
        _user_search: &'a UserSearch,
    ) -> Pin<Box<dyn Future<Output = DatabaseResult<Vec<User>>> + 'a + Send>> {
        Box::pin(async { Ok(vec![test_user()]) })
    }

    fn count_genders(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, DatabaseError>> + '_ + Send>> {
        Box::pin(async {
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
        })
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
