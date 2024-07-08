use crate::{
    arguments::AppConfig,
    handlers::user_handlers,
    // middleware::hashing::HashingMiddleware,
    types::jwt::{JWTClaims, Role},
};
use axum::{
    extract::Extension,
    http::header::HeaderName,
    routing::{delete, get, post, put},
    Router,
};
use middleware::request_trace::RequestLogger;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    classify::StatusInRangeAsFailures, compression::CompressionLayer,
    propagate_header::PropagateHeaderLayer, request_id::SetRequestIdLayer, trace::TraceLayer,
};
use user_persist::persistence::UserPersistence;

pub mod arguments;
mod extractors;
mod handlers;
mod middleware;
pub mod security;
pub mod types;

/// Tracing target for user-ms.
pub const USER_MS_TARGET: &str = "user-ms";
/// Tracing target for framework-ms.
pub const FRAMEWORK_TARGET: &str = "framework-ms";
/// Header name for correlation request identifier.
pub const REQ_ID_HEADER: &str = "x-request-id";

/// User endpoint routes with handler mappings.
fn user_routes() -> Router {
    Router::new()
        .route(
            "/user/:id",
            get(user_handlers::get_user), //.layer(HashingMiddleware::hash_user_layer()),
        )
        .route(
            "/user",
            post(user_handlers::save_user), // .layer(HashingMiddleware::hash_user_layer()),
        )
        // TODO: hashing middleware to validate hash on update.
        .route("/user", put(user_handlers::update_user))
        .route(
            "/user/search",
            post(user_handlers::search_users), // .layer(HashingMiddleware::hash_users_layer()),
        )
        .route("/user/counts", get(user_handlers::count_users))
        .route("/user/download", get(user_handlers::download_users))
        .route("/user/:id", delete(user_handlers::delete_user))
}

/// Builds the routes and the layered middleware.
pub fn build_app(persist: Arc<dyn UserPersistence>, app_config: AppConfig) -> Router {
    let tower_middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static(REQ_ID_HEADER),
            middleware::MakeRequestUuid,
        ))
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            REQ_ID_HEADER,
        )))
        .layer(
            TraceLayer::new(
                StatusInRangeAsFailures::new_for_client_and_server_errors().into_make_classifier(),
            )
            .make_span_with(RequestLogger)
            .on_request(RequestLogger)
            .on_failure(RequestLogger)
            .on_response(RequestLogger),
        )
        .layer(Extension(persist))
        .layer(Extension(Arc::new(app_config)))
        .layer(CompressionLayer::new());

    Router::new()
        .nest("/api/v1", user_routes())
        .layer(tower_middleware)
}
