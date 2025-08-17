//! Integration tests for routes.
use crate::common::{
    body_as, body_as_str, dump_result, test_database::test_user, test_router::TestRouterBuilder,
};
use axum::http::StatusCode;
use cool_asserts::assert_matches;
use rust_axum::{security::hashing::HashedUser, types::jwt::Role};
use serde_json::{from_str, json, to_string, to_vec, Value};
use tracing::debug;
use user_database::types::{Email, Gender, UpdateUser, User, UserKey, UserSearch};

mod common;

#[tokio::test]
async fn get_user() {
    let response = TestRouterBuilder::new()
        .get("/api/v1/user/61c0d1954c6b974ca7000000", Role::Admin)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let user = body_as::<HashedUser>(response).await;
    assert_eq!(&user.hid, "LCZLrq1TUum5LmbwzIoopIolNqLGv8iewjdsu7_49G8=")
}

#[tokio::test]
async fn get_user_invalid_role() {
    let response = TestRouterBuilder::new()
        .get("/api/v1/user/61c0d1954c6b974ca7000000", Role::User)
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    dump_result(response).await;
}

#[tokio::test]
async fn get_user_not_found() {
    let response = TestRouterBuilder::new()
        .get("/api/v1/user/71c0d1954c6b974ca7000000", Role::Admin)
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn save_user() {
    let json_user = to_vec(&test_user(None)).unwrap();
    let response = TestRouterBuilder::new()
        .post("/api/v1/user", Role::User, json_user)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let saved_user = body_as::<User>(response).await;
    debug!("response body: {saved_user:?}");

    assert!(saved_user.id.is_some());
    assert_eq!(saved_user.name, "Test User");
    assert_eq!(saved_user.age, 100);
    assert_eq!(saved_user.email, Email("test@test.com".to_string()));
    assert_eq!(saved_user.gender, Gender::Male);
}

#[tokio::test]
async fn save_user_validation_rejection() {
    let json_user = r#"{
        "name": "Test User",
        "age": 1,
        "email": "bad_value",
        "gender": "Male"
   }"#;

    let response = TestRouterBuilder::new()
        .post("/api/v1/user", Role::User, json_user)
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = body_as_str(response).await;

    let validation_errors = from_str::<Value>(&body).unwrap();

    debug!("json errors {body}");

    let email_validation_code = validation_errors
        .get("validation_errors")
        .and_then(|v| v.get("email"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("code"));

    let age_validation_code = validation_errors
        .get("validation_errors")
        .and_then(|v| v.get("age"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("code"));

    assert_eq!(email_validation_code, Some(json!("invalid email")).as_ref());
    assert_eq!(age_validation_code, Some(json!("range")).as_ref());
}

#[tokio::test]
async fn update_user() {
    let update_user = UpdateUser {
        id: UserKey("fakekey".into()),
        name: "New Name".into(),
        email: Email("test@test.com".into()),
        age: 100,
        hid: "xBS6Bfv589WArC5A3psqFZRv_sPe8thJqRHBaipYsho=".into(),
    };

    let update_user_json = to_string(&update_user).unwrap();
    debug!("update user: {update_user_json}");

    let response = TestRouterBuilder::new()
        .put("/api/v1/user", Role::Admin, update_user_json)
        .await;
    let status = response.status();
    let body = body_as_str(response).await;
    debug!("response body: {body}");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "");
}

#[tokio::test]
async fn update_user_bad_hash() {
    let update_user = UpdateUser {
        id: UserKey("fakekey".into()),
        name: "New Name".into(),
        email: Email("test@test.com".into()),
        age: 100,
        hid: "invalid_hash".into(),
    };

    let update_user_json = to_string(&update_user).unwrap();
    debug!("update user: {update_user_json}");

    let response = TestRouterBuilder::new()
        .put("/api/v1/user", Role::Admin, update_user_json)
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        body_as::<Value>(response).await,
        json!({
          "label": "json_parse.failed",
          "message": "Invalid Hash"
        })
    );
}

#[tokio::test]
async fn search_users() {
    let search = UserSearch {
        email: Some(Email("test@test.com".to_owned())),
        name: None,
        gender: None,
    };

    let search_json = to_vec(&search).unwrap();

    let response = TestRouterBuilder::new()
        .post("/api/v1/user/search", Role::Admin, search_json)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let users = body_as::<Vec<HashedUser>>(response).await;

    assert_matches!(users, [
        HashedUser { user: User { id, name, age, email, gender: Gender::Male }, hid } => {
            assert_eq!(id.as_deref().map(AsRef::as_ref), Some("61c0d1954c6b974ca7000000"));
            assert_eq!(name, "Test User");
            assert_eq!(age, 100);
            assert_eq!(email.as_ref() as &str, "test@test.com");
            assert_eq!(hid, "LCZLrq1TUum5LmbwzIoopIolNqLGv8iewjdsu7_49G8=");
        }
    ]);
}

#[tokio::test]
async fn count_users() {
    let response = TestRouterBuilder::new()
        .get("/api/v1/user/counts", Role::Admin)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body_as::<Value>(response).await,
        json!([{"_id":"Male","count":6},{"_id":"Female","count":12}])
    );
}

#[tokio::test]
async fn download_users() {
    let response = TestRouterBuilder::new()
        .get("/api/v1/user/download", Role::Admin)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body_as::<Vec<User>>(response).await,
        [
            User {
                id: Some(UserKey("key1".into())),
                name: "Test User 1".into(),
                age: 100,
                email: Email("test1@test.com".into()),
                gender: Gender::Male,
            },
            User {
                id: Some(UserKey("key2".into())),
                name: "Test User 2".into(),
                age: 100,
                email: Email("test2@test.com".into()),
                gender: Gender::Male,
            },
            User {
                id: Some(UserKey("key3".into())),
                name: "Test User 3".into(),
                age: 100,
                email: Email("test3@test.com".into()),
                gender: Gender::Male,
            },
        ]
    )
}
