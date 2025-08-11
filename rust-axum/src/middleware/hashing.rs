//! Hashing middleware.
use crate::{security::hashing::IntoTypeWithHash, AppConfig};
use axum::{
    body::{to_bytes, Body},
    http::Request,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use http::StatusCode;
use hyper::body::Bytes;
use serde::Deserialize;
use std::{
    marker::PhantomData,
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;
use tower_layer::{layer_fn, LayerFn};
use tracing::error;

/// Middleware for adding hashes to successful responses.
#[derive(Clone, Copy)]
pub struct HashingMiddleware<S, R> {
    pub inner: S,
    _phantom: PhantomData<R>,
}

/// Create a hashing middleware with a provided hashing transformation function.
pub fn hashing_middleware<R, S>() -> LayerFn<impl Fn(S) -> HashingMiddleware<S, R> + Clone + 'static>
where
    R: IntoTypeWithHash + Send + 'static,
{
    layer_fn(move |inner| HashingMiddleware {
        inner,
        _phantom: PhantomData,
    })
}

/// Apply hashing transformation on the body response type.
async fn transform_body<T>(hash_prefix: &str, response: Response) -> Response
where
    for<'a> T: IntoTypeWithHash + Deserialize<'a> + 'static,
{
    match to_bytes(response.into_body(), usize::MAX).await {
        Ok(bytes) => {
            let body = Body::from(
                match serde_json::from_slice(&bytes).map(|b: T| b.hash(hash_prefix)) {
                    Ok(hashed) => Bytes::from(serde_json::to_vec(&hashed).unwrap()),
                    Err(e) => {
                        error!("Failed to hash response {e}");
                        bytes
                    }
                },
            );
            body.into_response()
        }
        Err(err) => {
            error!("Failed to hash body {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed").into_response()
        }
    }
}

impl<S, R> Service<Request<Body>> for HashingMiddleware<S, R>
where
    S: Service<Request<Body>, Response = Response> + Send + Sync + 'static,
    S::Future: Send + 'static,
    for<'a> R: IntoTypeWithHash + Deserialize<'a> + Send + 'static + Sync,
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

        let fut = self.inner.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(if res.status().is_success() {
                // Apply hashing function.
                transform_body::<R>(config.hash_prefix(), res).await
            } else {
                // No hashing.
                res
            })
        })
    }
}
