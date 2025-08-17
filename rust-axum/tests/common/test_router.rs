//! Test Router
use crate::common::test_database::TestDatabase;
use axum::Router;
use rust_axum::{arguments::AppConfig, build_app};
use std::sync::{Arc, Once};
use tracing_subscriber::EnvFilter;

/// Build a test router.
pub struct TestRouterBuilder {
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
    #[must_use]
    pub fn with_database(mut self, database: impl Into<Option<Arc<TestDatabase>>>) -> Self {
        self.database = database.into();
        self
    }

    /// Build router
    pub fn build(self) -> Router {
        app(self.database)
    }
}

/// Build test Router.
fn app(database: Option<Arc<TestDatabase>>) -> Router {
    init_log();
    let database = match database {
        Some(p) => p,
        None => Arc::new(TestDatabase::new()),
    };
    build_app(database, AppConfig::new(SECRET))
}
