use crate::common::{
    add_jwt, app, body_as, body_as_str, dump_result, test_persist::test_user, MIME_JSON,
    TEST_TARGET,
};
use axum::{
    body::Body,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Method, Request, StatusCode,
    },
};
use rust_axum::{security::hashing::HashedUser, types::jwt::Role};
use serde_json::{from_str, json, to_string, Value};
use tower::ServiceExt;
use tracing::debug;
use user_persist::types::{Email, UpdateUser, User, UserKey, UserSearch};

mod common;

#[tokio::test]
async fn get_user() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user/61c0d1954c6b974ca7000000")
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let user = body_as::<HashedUser>(response).await;
    assert_eq!(&user.hid, "LCZLrq1TUum5LmbwzIoopIolNqLGv8iewjdsu7_49G8=")
}

#[tokio::test]
async fn get_user_invalid_role() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user/61c0d1954c6b974ca7000000")
                .header(AUTHORIZATION, add_jwt(Role::User))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    dump_result(response).await;
}

#[tokio::test]
async fn get_user_not_found() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user/71c0d1954c6b974ca7000000")
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn save_user() {
    let json_user = serde_json::to_string(&test_user(None)).unwrap();
    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user")
                .method(Method::POST)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(Role::User))
                .body(Body::from(json_user))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let saved_user = body_as::<User>(response).await;
    debug!("response body: {saved_user:?}");
    assert!(saved_user.id.is_some());
}

#[tokio::test]
async fn save_user_validation_rejection() {
    let json_user = r#"{
    "name": "Test User",
    "age": 1,
    "email": "bad_value",
    "gender": "Male"
  }"#;

    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user")
                .method(Method::POST)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(Role::User))
                .body(Body::from(json_user))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = body_as_str(response).await;

    let validation_errors = from_str::<Value>(&body).unwrap();

    debug!(target: TEST_TARGET, "json errors {body}");

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

    debug!(target: TEST_TARGET, "update user: {update_user_json}");

    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user")
                .method(Method::PUT)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::from(update_user_json))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = body_as_str(response).await;
    debug!(target: TEST_TARGET, "response body: {body}");
    assert_eq!(status, StatusCode::OK);
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

    debug!(target: TEST_TARGET, "update user: {update_user_json}");

    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user")
                .method(Method::PUT)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::from(update_user_json))
                .unwrap(),
        )
        .await
        .unwrap();
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

    let search_json = to_string(&search).unwrap();

    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user/search")
                .method(Method::POST)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::from(search_json))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    dump_result(response).await;
}

#[tokio::test]
async fn count_users() {
    let response = app(None)
        .oneshot(
            Request::builder()
                .uri("/api/v1/user/counts")
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    dump_result(response).await;
}
