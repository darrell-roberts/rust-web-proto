use actix_http::header::TryIntoHeaderPair;
use actix_service::Service;
use actix_web::{body::MessageBody, dev, http, test, web, App};
use rust_actix_web::{
    handlers,
    middleware::{create_test_jwt, JwtAuth},
    types::Role,
};
use serde_json::{json, Value};
use std::sync::{Arc, Once};
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
        if id.0 == "61c0d1954c6b974ca7000000" {
            Ok(Some(test_user()))
        } else {
            Ok(None)
        }
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

    async fn download(
        &self,
    ) -> DatabaseResult<impl futures::Stream<Item = DatabaseResult<User>> + 'static + Send> {
        Ok(futures::stream::iter([]))
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
                    .service(handlers::get_user)
                    .service(handlers::search_users)
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
    let req = test::TestRequest::with_uri("/api/v1/user/61c0d1954c6b974ca7000000")
        .insert_header(jwt_header(Role::Admin))
        .to_request();

    let res = service.call(req).await.unwrap();

    assert_eq!(res.status(), http::StatusCode::OK);
}

#[actix_web::test]
async fn count_users() {
    init_log();
    let service = get_service().await;
    let req = test::TestRequest::with_uri("/api/v1/user/counts")
        .insert_header(jwt_header(Role::Admin))
        .to_request();

    let res = service.call(req).await.unwrap();

    assert_eq!(res.status(), http::StatusCode::OK);
}

#[actix_web::test]
async fn save_user() {
    init_log();
    let service = get_service().await;
    let req = test::TestRequest::post()
        .uri("/api/v1/user")
        .insert_header(jwt_header(Role::User))
        .set_json(test_user())
        .to_request();

    let res = service.call(req).await.unwrap();

    assert_eq!(res.status(), http::StatusCode::OK);
}

#[actix_web::test]
async fn search_users() {
    init_log();
    let service = get_service().await;
    let req = test::TestRequest::post()
        .uri("/api/v1/user/search")
        .insert_header(jwt_header(Role::Admin))
        .set_json(UserSearch {
            email: Some(Email("some@where.com".to_owned())),
            name: None,
            gender: None,
        })
        .to_request();

    let res = service.call(req).await.unwrap();

    assert_eq!(res.status(), http::StatusCode::OK);
}

#[actix_web::test]
async fn update_user() {
    init_log();
    let service = get_service().await;
    let req = test::TestRequest::put()
        .uri("/api/v1/user")
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

    assert_eq!(res.status(), http::StatusCode::OK);
}
