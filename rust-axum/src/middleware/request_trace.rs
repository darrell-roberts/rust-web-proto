use crate::USER_MS_TARGET;
use http::{header::HOST, Request, Response};
use std::{fmt::Display, time::Duration};
use tower_http::{
    request_id::RequestId,
    trace::{MakeSpan, OnFailure, OnRequest, OnResponse},
};
use tracing::{field, Span};

#[derive(Clone, Debug)]
pub struct RequestLogger;

/// Each request span will have a requestId, uri and method.
impl<B> MakeSpan<B> for RequestLogger {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let req_id = request
            .extensions()
            .get::<RequestId>()
            .map(|r| r.header_value().to_str().unwrap_or_default())
            .unwrap_or_default();

        let host = request
            .headers()
            .get(HOST)
            .map(|v| v.to_str().unwrap_or_default())
            .unwrap_or_else(|| "Unknown host");

        tracing::info_span!(
          USER_MS_TARGET,
          "requestId" = req_id,
          "uri" = request.uri().path(),
          "method" = request.method().as_str(),
          "statusCode" = field::Empty,
          "failureClass" = field::Empty,
          %host
        )
    }
}

impl<C: Display> OnFailure<C> for RequestLogger {
    fn on_failure(&mut self, failure_classification: C, latency: Duration, span: &Span) {
        span.record("failureClass", &field::display(&failure_classification));
        tracing::error!(
            "request failed with {failure_classification} in {} ms",
            latency.as_millis()
        );
    }
}

impl<B> OnRequest<B> for RequestLogger {
    fn on_request(&mut self, request: &Request<B>, _span: &Span) {
        tracing::info!(
            "request started {} {}",
            request.method(),
            request.uri().path()
        )
    }
}

impl<B> OnResponse<B> for RequestLogger {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        span.record("statusCode", &field::display(response.status().as_str()));
        tracing::info!(
            "response completed with status {} in {} minutes {} seconds {} ms",
            response.status(),
            latency.as_secs() * 60,
            latency.as_secs(),
            latency.as_millis()
        );
    }
}
