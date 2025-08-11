//! Hashing middleware.
use crate::{security::hashing::IntoTypeWithHash, AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    response::{IntoResponse, Response},
};
use futures::{future::BoxFuture, TryFutureExt as _};
use http::StatusCode;
use hyper::body::Bytes;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;
use tower_layer::{layer_fn, LayerFn};
use tracing::error;
use user_database::types::User;

/// Deserialize the response and call its hash method.
pub fn hash_user(hash_prefix: &str, bytes: Bytes) -> Bytes {
    match serde_json::from_slice(&bytes).map(|b: User| b.hash(hash_prefix)) {
        Ok(hashed) => Bytes::from(serde_json::to_vec(&hashed).unwrap()),
        Err(e) => {
            error!("Failed to hash response {e}");
            bytes
        }
    }
}
/// Deserialize the response and call its hash method.
pub fn hash_users(hash_prefix: &str, bytes: Bytes) -> Bytes {
    match serde_json::from_slice(&bytes).map(|v: Vec<User>| {
        v.into_iter()
            .map(|u| u.hash(hash_prefix))
            .collect::<Vec<_>>()
    }) {
        Ok(hashed) => Bytes::from(serde_json::to_vec(&hashed).unwrap()),
        Err(e) => {
            error!("Failed to hash response {e}");
            bytes
        }
    }
}

/// Middleware for adding hashes to successful responses.
#[derive(Clone, Copy)]
pub struct HashingMiddleware<S, F> {
    pub inner: S,
    pub hash_fn: F,
}

impl<S, F> HashingMiddleware<S, F> {
    /// Create a hashing middleware with a provided hashing transformation function.
    pub fn new(hash_fn: F) -> LayerFn<impl Fn(S) -> HashingMiddleware<S, F> + Clone + 'static>
    where
        F: FnOnce(&str, Bytes) -> Bytes + Clone + Copy + 'static + Send,
    {
        layer_fn(move |inner| HashingMiddleware { inner, hash_fn })
    }
}

impl<S, F> Service<Request<Body>> for HashingMiddleware<S, F>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    F: FnOnce(&str, Bytes) -> Bytes + Clone + Copy + 'static + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let config = req
            .extensions()
            .get::<Arc<AppConfig>>()
            .expect("Did you forget to add Arc<AppConfig> to state?")
            .clone();

        let hash_fn = self.hash_fn;

        Box::pin(self.inner.call(req).and_then(move |res| async move {
            Ok(if res.status().is_success() {
                // Apply hashing function.
                match to_bytes(res.into_body(), usize::MAX).await {
                    Ok(bytes) => Body::from(hash_fn(config.hash_prefix(), bytes)).into_response(),
                    Err(_err) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed").into_response()
                    }
                }
            } else {
                // No hashing.
                res
            })
        }))
    }
}
