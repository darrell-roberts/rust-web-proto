//! Test an end to end scenario.
use crate::common::test_router::TestRouterBuilder;
use axum::http::StatusCode;
use common::{body_as, test_database::TestDatabase};
use rust_axum::{security::hashing::HashedUser, types::jwt::Role};
use std::sync::Arc;
use tracing::debug;
use user_database::types::{UpdateUser, User};

mod common;

/// Runs a test scenario. A user is saved/updated/fetched/deleted/fetched.
#[tokio::test]
async fn test_scenario() {
    let database = Arc::new(TestDatabase::default());

    let user = create_user(database.clone()).await;
    update_user(database.clone(), &user).await;
    get_user(database.clone(), &user).await;
    delete_user(database.clone(), &user).await;

    let response = TestRouterBuilder::new()
        .with_database(database)
        .get(
            format!(
                "/api/v1/user/{}",
                user.user.id.clone().expect("Missing user id")
            ),
            Role::Admin,
        )
        .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn create_user(database: Arc<TestDatabase>) -> HashedUser {
    let json_user = r#"{
    "name": "Scenario User",
    "email": "scenario@test.com",
    "age": 120,
    "gender": "Female"
  }"#;

    let save_response = TestRouterBuilder::new()
        .with_database(database)
        .post("/api/v1/user", Role::User, json_user)
        .await;

    assert_eq!(save_response.status(), StatusCode::OK);
    let saved_user = body_as::<HashedUser>(save_response).await;
    debug!("saved_user: {saved_user:?}");
    assert!(saved_user.user.id.is_some());
    saved_user
}

async fn update_user(database: Arc<TestDatabase>, user: &HashedUser) {
    let update_user = UpdateUser {
        id: user.user.id.clone().expect("No user id"),
        name: user.user.name.clone(),
        hid: user.hid.clone(),
        age: 150,
        email: user.user.email.clone(),
    };

    let update_response = TestRouterBuilder::new()
        .with_database(database)
        .put(
            "/api/v1/user",
            Role::Admin,
            serde_json::to_string(&update_user).expect("Update user serialization failed"),
        )
        .await;

    assert_eq!(update_response.status(), StatusCode::OK);
}

async fn get_user(database: Arc<TestDatabase>, user: &HashedUser) {
    let response = TestRouterBuilder::new()
        .with_database(database)
        .get(
            format!(
                "/api/v1/user/{}",
                user.user.id.clone().expect("Missing user id")
            ),
            Role::Admin,
        )
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let fetched_user = body_as::<User>(response).await;
    assert_eq!(fetched_user.age, 150);
}

async fn delete_user(database: Arc<TestDatabase>, user: &HashedUser) {
    let response = TestRouterBuilder::new()
        .with_database(database)
        .delete(
            format!(
                "/api/v1/user/{}",
                user.user.id.clone().expect("Missing user id")
            ),
            Role::Admin,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}
