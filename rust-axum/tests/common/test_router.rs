//! Test Router
use crate::common::{add_jwt, test_database::TestDatabase, MIME_JSON};
use axum::{body::Body, Router};
use http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method, Request, Uri,
};
use rust_axum::{arguments::AppConfig, build_app, types::jwt::Role};
use std::{
    future::Future,
    sync::{Arc, Once},
};
use tower::ServiceExt;
use tracing_subscriber::EnvFilter;

pub struct TestApp {
    router: Router,
    request: Request<Body>,
}

impl TestApp {
    /// Run the test request.
    async fn run(self) -> http::Response<Body> {
        self.router.oneshot(self.request).await.unwrap()
    }
}

/// Build a test router.
pub(crate) struct TestRouterBuilder {
    database: Option<Arc<TestDatabase>>,
}

/// Test secret
pub(crate) static SECRET: &[u8] = "TEST_SECRET".as_bytes();

/// Global log initialization.
static INIT: Once = Once::new();

// Setup tracing first.
fn init_log() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_target(true)
            .init();
    });
}

impl TestRouterBuilder {
    /// New test router builder.
    #[must_use]
    pub fn new() -> Self {
        Self { database: None }
    }

    /// Add an existing database.
    #[allow(dead_code)]
    #[must_use]
    pub fn with_database(mut self, database: impl Into<Option<Arc<TestDatabase>>>) -> Self {
        self.database = database.into();
        self
    }

    /// Run a get request.
    #[allow(dead_code)]
    pub fn get<U>(self, uri: U, role: Role) -> impl Future<Output = http::Response<Body>>
    where
        U: TryInto<Uri>,
        <U as TryInto<Uri>>::Error: Into<http::Error>,
    {
        TestApp {
            router: app(self.database),
            request: Request::builder()
                .uri(uri)
                .header(AUTHORIZATION, add_jwt(role))
                .body(Body::empty())
                .unwrap(),
        }
        .run()
    }

    /// Run a post request.
    #[allow(dead_code)]
    pub fn post<U>(
        self,
        uri: U,
        role: Role,
        body: impl Into<Body>,
    ) -> impl Future<Output = http::Response<Body>>
    where
        U: TryInto<Uri>,
        <U as TryInto<Uri>>::Error: Into<http::Error>,
    {
        TestApp {
            router: app(self.database),
            request: Request::builder()
                .uri(uri)
                .method(Method::POST)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(role))
                .body(body.into())
                .unwrap(),
        }
        .run()
    }

    /// Run a put request
    #[allow(dead_code)]
    pub fn put<U>(
        self,
        uri: U,
        role: Role,
        body: impl Into<Body>,
    ) -> impl Future<Output = http::Response<Body>>
    where
        U: TryInto<Uri>,
        <U as TryInto<Uri>>::Error: Into<http::Error>,
    {
        TestApp {
            router: app(self.database),
            request: Request::builder()
                .uri(uri)
                .method(Method::PUT)
                .header(CONTENT_TYPE, MIME_JSON)
                .header(AUTHORIZATION, add_jwt(role))
                .body(body.into())
                .unwrap(),
        }
        .run()
    }

    /// Run a delete request.
    #[allow(dead_code)]
    pub fn delete<U>(self, uri: U, role: Role) -> impl Future<Output = http::Response<Body>>
    where
        U: TryInto<Uri>,
        <U as TryInto<Uri>>::Error: Into<http::Error>,
    {
        TestApp {
            router: app(self.database),
            request: Request::builder()
                .uri(uri)
                .method(Method::DELETE)
                .header(AUTHORIZATION, add_jwt(role))
                .body(Body::empty())
                .unwrap(),
        }
        .run()
    }
}

/// Build test Router.
fn app(database: Option<Arc<TestDatabase>>) -> Router {
    init_log();
    let database = match database {
        Some(p) => p,
        None => Arc::new(TestDatabase::default()),
    };
    build_app(database, AppConfig::new(SECRET))
}
