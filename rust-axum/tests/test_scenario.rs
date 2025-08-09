//! Test an end to end scenario.
use axum::{
    body::Body,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Method, Request, StatusCode,
    },
};
use common::{add_jwt, app, body_as, test_persist::TestPersistence, MIME_JSON, TEST_TARGET};
use rust_axum::{security::hashing::HashedUser, types::jwt::Role};
use std::sync::Arc;
use tower::ServiceExt;
use tracing::debug;
use user_persist::types::{UpdateUser, User};

mod common;

/// Runs a test scenario. A user is saved/updated/fetched/deleted/fetched.
#[tokio::test]
async fn test_scenario() {
    let persist = Arc::new(TestPersistence::new());

    let user = create_user(persist.clone()).await;
    update_user(persist.clone(), &user).await;
    get_user(persist.clone(), &user).await;
    delete_user(persist.clone(), &user).await;

    let response = app(Some(persist))
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/user/{}",
                    user.user.id.clone().expect("Missing user id")
                ))
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn create_user(persist: Arc<TestPersistence>) -> HashedUser {
    let json_user = r#"{
    "name": "Scenario User",
    "email": "scenario@test.com",
    "age": 120,
    "gender": "Female"
  }"#;

    let save_response = app(Some(persist))
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

    assert_eq!(save_response.status(), StatusCode::OK);
    let saved_user = body_as::<HashedUser>(save_response).await;
    debug!(target: TEST_TARGET, "saved_user: {saved_user:?}");
    assert!(saved_user.user.id.is_some());
    saved_user
}

async fn update_user(persist: Arc<TestPersistence>, user: &HashedUser) {
    let update_user = UpdateUser {
        id: user.user.id.clone().expect("No user id"),
        name: user.user.name.clone(),
        hid: user.hid.clone(),
        age: 150,
        email: user.user.email.clone(),
    };

    let update_response = app(Some(persist))
        .oneshot(
            Request::builder()
                .uri("/api/v1/user")
                .method(Method::PUT)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::from(
                    serde_json::to_string(&update_user).expect("Update user serialization failed"),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
}

async fn get_user(persist: Arc<TestPersistence>, user: &HashedUser) {
    let response = app(Some(persist))
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/user/{}",
                    user.user.id.clone().expect("Missing user id")
                ))
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let fetched_user = body_as::<User>(response).await;
    assert_eq!(fetched_user.age, 150);
}

async fn delete_user(persist: Arc<TestPersistence>, user: &HashedUser) {
    let response = app(Some(persist))
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/user/{}",
                    user.user.id.clone().expect("Missing user id")
                ))
                .method("DELETE")
                .header(AUTHORIZATION, add_jwt(Role::Admin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
