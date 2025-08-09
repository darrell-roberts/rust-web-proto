//! Hashing middleware.
use crate::{security::hashing::Hashable, AppConfig};
use axum::{body::Body, http::Request, response::Response};
use futures::future::BoxFuture;
use hyper::body::Bytes;
use std::{
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;
use tower_http::ServiceExt;
use tower_layer::{layer_fn, LayerFn};
use tracing::{debug, error};
use user_persist::types::User;

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
    match serde_json::from_slice(&bytes)
        .map(|v: Vec<User>| v.iter().map(|u| u.hash(hash_prefix)).collect::<Vec<_>>())
    {
        Ok(hashed) => Bytes::from(serde_json::to_vec(&hashed).unwrap()),
        Err(e) => {
            error!("Failed to hash response {e}");
            bytes
        }
    }
}

#[derive(Clone)]
pub struct HashingMiddleware<S, F> {
    pub inner: S,
    pub hash_fn: F,
}

type HashingFunc = fn(&str, Bytes) -> Bytes;

impl<S> HashingMiddleware<S, HashingFunc> {
    pub fn hash_users_layer() -> LayerFn<fn(S) -> HashingMiddleware<S, HashingFunc>> {
        layer_fn(|inner| HashingMiddleware {
            inner,
            hash_fn: hash_users,
        })
    }

    pub fn hash_user_layer() -> LayerFn<fn(S) -> HashingMiddleware<S, HashingFunc>> {
        layer_fn(|inner| HashingMiddleware {
            inner,
            hash_fn: hash_user,
        })
    }

    // pub fn hash_with_fn<F>(f: F) -> LayerFn<fn(S) -> HashingMiddleware<S, F>>
    // where
    //   F: FnMut(Bytes, &str) -> Bytes + Clone + 'static + Send,
    // {
    //   layer_fn(|inner| HashingMiddleware { inner, hash_fn: f })
    // }
}

impl<S, F> Service<Request<Body>> for HashingMiddleware<S, F>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    F: FnMut(&str, Bytes) -> Bytes + Clone + 'static + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let hash_prefix = req
            .extensions()
            .get::<Arc<AppConfig>>()
            .map(|config| config.hash_prefix())
            .unwrap_or_else(|| "default_prefix")
            .to_owned();

        debug!("hash_prefix: {hash_prefix}");

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let mut hash_f = self.hash_fn.clone();

        Box::pin(async move {
            let res = inner.call(req).await?;

            if res.status().is_success() {
                debug!("Hashing response");

                let response = res
                    .map(move |body| body.map_response_body(|bytes| hash_f(&hash_prefix, bytes)));

                Ok(response)
            } else {
                Ok(res)
            }
        })
    }
}
