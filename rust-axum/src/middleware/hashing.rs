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
use tower::{Layer, Service};
use tracing::error;

/// Middleware for adding hashes to successful responses.
#[derive(Clone, Copy)]
pub struct HashingService<S, R> {
    pub inner: S,
    _phantom: PhantomData<R>,
}

/// Hashing middleware layer.
#[derive(Clone, Copy)]
pub struct HashingLayer<R> {
    _phantom: PhantomData<R>,
}

impl<S, R> Layer<S> for HashingLayer<R> {
    type Service = HashingService<S, R>;

    fn layer(&self, inner: S) -> Self::Service {
        HashingService {
            inner,
            _phantom: PhantomData,
        }
    }
}

/// Create a hashing middleware that hashes `R` in the response body.
pub fn hashing_layer<R>() -> HashingLayer<R>
where
    R: IntoTypeWithHash + Send + 'static,
{
    HashingLayer {
        _phantom: PhantomData,
    }
}

/// Apply hashing transformation on the body response type.
async fn transform_body<T>(hash_prefix: &str, response: Response) -> Response
where
    for<'a> T: IntoTypeWithHash + Deserialize<'a> + 'static,
{
    match to_bytes(response.into_body(), usize::MAX).await {
        Ok(bytes) => match serde_json::from_slice(&bytes).map(|b: T| b.hash(hash_prefix)) {
            Ok(hashed) => {
                Body::from(Bytes::from(serde_json::to_vec(&hashed).unwrap())).into_response()
            }
            Err(e) => {
                error!("Failed to hash response {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to deserialize body for hashing",
                )
                    .into_response()
            }
        },
        Err(err) => {
            error!("Failed to hash body {err}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Hashing failed").into_response()
        }
    }
}

impl<S, R> Service<Request<Body>> for HashingService<S, R>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    for<'a> R: IntoTypeWithHash + Deserialize<'a> + Send + 'static,
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
