use actix_http::header::TryIntoHeaderPair;
use actix_service::Service;
use actix_web::{
    body::{self, MessageBody},
    dev,
    rt::pin,
    test, web, App,
};
use rust_actix_web::{
    handlers,
    middleware::{create_test_jwt, JwtAuth},
    types::Role,
};
use serde_json::{json, Value};
use std::{
    future,
    sync::{Arc, Once},
};
use tracing::info;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::EnvFilter;
use user_database::database::{DatabaseResult, UserDatabase, UserDatabaseDynSafe};
use user_database::types::{Email, Gender, UpdateUser, User, UserKey, UserSearch};

static INIT: Once = Once::new();

// Setup tracing first.
fn init_log() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_target(true)
            .pretty()
            // .json()
            // .flatten_event(true)
            .init();
    });
}

fn test_user() -> User {
    User {
        id: None,
        name: String::from("Test User"),
        email: Email(String::from("test@test.com")),
        age: 100,
        gender: Gender::Male,
    }
}

#[derive(Debug, Clone)]
pub struct TestDatabase;

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

async fn get_service() -> impl Service<
    actix_http::Request,
    Response = dev::ServiceResponse<impl MessageBody>,
    Error = actix_web::Error,
> {
    let database: web::Data<Arc<dyn UserDatabaseDynSafe>> = web::Data::new(Arc::new(TestDatabase));
    test::init_service(
        App::new()
            .app_data(database)
            .wrap(JwtAuth::default())
            .wrap(TracingLogger::default())
            .service(
                web::scope("/api/v1/user")
                    .service(handlers::count_users)
                    .service(handlers::search_users)
                    .service(handlers::download_users)
                    .service(handlers::get_user)
                    .service(handlers::save_user)
                    .service(handlers::update_user),
            ),
    )
    .await
}

fn jwt_header(role: Role) -> impl TryIntoHeaderPair {
    (
        "Authorization",
        format!("Bearer {}", create_test_jwt(role).unwrap()),
    )
}

#[actix_web::test]
async fn get_user() {
    init_log();
    let service = get_service().await;
    let uri = "/api/v1/user/61c0d1954c6b974ca7000000";
    let req = test::TestRequest::with_uri(uri)
        .insert_header(jwt_header(Role::Admin))
        .to_request();

    let res = service.call(req).await.unwrap();

    assert!(res.status().is_success());
    dump_body(res.into_body(), uri).await;
}

#[actix_web::test]
async fn count_users() {
    init_log();
    let service = get_service().await;
    let uri = "/api/v1/user/counts";
    let req = test::TestRequest::with_uri(uri)
        .insert_header(jwt_header(Role::Admin))
        .to_request();

    let res = service.call(req).await.unwrap();
    assert!(res.status().is_success());

    dump_body(res.into_body(), uri).await;
}

#[actix_web::test]
async fn save_user() {
    init_log();
    let service = get_service().await;
    let uri = "/api/v1/user";
    let req = test::TestRequest::post()
        .uri(uri)
        .insert_header(jwt_header(Role::User))
        .set_json(test_user())
        .to_request();

    let res = service.call(req).await.unwrap();

    assert!(res.status().is_success());
    dump_body(res.into_body(), uri).await;
}

#[actix_web::test]
async fn search_users() {
    init_log();
    let service = get_service().await;
    let uri = "/api/v1/user/search";
    let req = test::TestRequest::post()
        .uri(uri)
        .insert_header(jwt_header(Role::Admin))
        .set_json(UserSearch {
            email: Some(Email("some@where.com".to_owned())),
            name: None,
            gender: None,
        })
        .to_request();

    let res = service.call(req).await.unwrap();
    assert!(res.status().is_success());
    dump_body(res.into_body(), uri).await;
}

#[actix_web::test]
async fn update_user() {
    init_log();
    let service = get_service().await;
    let uri = "/api/v1/user";
    let req = test::TestRequest::put()
        .uri(uri)
        .insert_header(jwt_header(Role::Admin))
        .set_json(UpdateUser {
            id: UserKey("some_key".to_owned()),
            name: "New name".to_owned(),
            age: 100,
            email: Email("test@test.com".into()),
            hid: "xBS6Bfv589WArC5A3psqFZRv/sPe8thJqRHBaipYsho=".into(),
        })
        .to_request();

    let res = service.call(req).await.unwrap();

    assert!(res.status().is_success());
    dump_body(res.into_body(), uri).await;
}

#[actix_web::test]
async fn test_download() {
    init_log();

    let uri = "/api/v1/user/download";

    let service = get_service().await;
    let req = test::TestRequest::with_uri(uri)
        .insert_header(jwt_header(Role::Admin))
        .to_request();

    let res = service.call(req).await.unwrap();
    assert!(res.status().is_success());
    let body = res.into_body();

    pin!(body);
    for i in 0..5 {
        let bytes = future::poll_fn(|cx| body.as_mut().poll_next(cx))
            .await
            .unwrap();
        match bytes {
            Ok(bytes) => {
                info!("{i} {uri} body: {}", String::from_utf8_lossy(&bytes));
            }
            Err(_) => panic!("No chunk"),
        }
    }
}

async fn dump_body(body: impl MessageBody, uri: &str) {
    pin!(body);

    let bytes = body::to_bytes(body).await;

    match bytes {
        Ok(bytes) => {
            info!("{uri} body: {}", String::from_utf8_lossy(&bytes));
        }
        Err(_) => panic!("Test failed"),
    }
}
