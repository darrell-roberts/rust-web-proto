//! Creates a User REST API backend.
use crate::{
    arguments::AppConfig,
    handlers::user_handlers,
    types::jwt::{JWTClaims, Role},
};
use axum::{
    extract::Extension,
    http::header::HeaderName,
    routing::{delete, get, post, put},
    Router,
};
use middleware::{hashing::HashingMiddleware, request_trace::RequestLogger};
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

/// Header name for correlation request identifier.
pub const REQ_ID_HEADER: &str = "x-request-id";

/// User endpoint routes with handler mappings.
fn user_routes<P>() -> Router
where
    P: UserPersistence + 'static,
{
    Router::new()
        .route(
            "/user/{id}",
            get(user_handlers::get_user::<P>).layer(HashingMiddleware::hash_user_layer()),
        )
        .route(
            "/user",
            post(user_handlers::save_user::<P>).layer(HashingMiddleware::hash_user_layer()),
        )
        .route("/user", put(user_handlers::update_user::<P>))
        .route(
            "/user/search",
            post(user_handlers::search_users::<P>).layer(HashingMiddleware::hash_users_layer()),
        )
        .route("/user/counts", get(user_handlers::count_users::<P>))
        .route("/user/download", get(user_handlers::download_users))
        .route("/user/{id}", delete(user_handlers::delete_user::<P>))
}

/// Builds the routes and the layered middleware.
pub fn build_app<P>(persist: Arc<P>, app_config: AppConfig) -> Router
where
    P: UserPersistence + 'static,
{
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
        .nest("/api/v1", user_routes::<P>())
        .layer(tower_middleware)
}
