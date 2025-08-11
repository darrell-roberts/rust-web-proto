//! Creates a User REST API backend.
use crate::{
    arguments::AppConfig, handlers::user_handlers, middleware::hashing::hashing_middleware,
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
use user_database::{database::UserDatabase, types::User};

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
    P: UserDatabase + 'static,
{
    Router::new()
        .route(
            "/user/{id}",
            get(user_handlers::get_user::<P>).layer(hashing_middleware::<User, _>()),
        )
        .route(
            "/user",
            post(user_handlers::save_user::<P>).layer(hashing_middleware::<User, _>()),
        )
        .route("/user", put(user_handlers::update_user::<P>))
        .route(
            "/user/search",
            post(user_handlers::search_users::<P>).layer(hashing_middleware::<Vec<User>, _>()),
        )
        .route("/user/counts", get(user_handlers::count_users::<P>))
        .route("/user/download", get(user_handlers::download_users))
        .route("/user/{id}", delete(user_handlers::delete_user::<P>))
}

/// Builds the routes and the layered middleware.
pub fn build_app<P>(database: Arc<P>, app_config: AppConfig) -> Router
where
    P: UserDatabase + 'static,
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
        .layer(Extension(database))
        .layer(Extension(Arc::new(app_config)))
        .layer(CompressionLayer::new());

    Router::new()
        .nest("/api/v1", user_routes::<P>())
        .layer(tower_middleware)
}
