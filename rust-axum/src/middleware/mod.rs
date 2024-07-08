/*!
API server middleware.
*/

use http::Request;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

// pub mod hashing;
pub mod request_trace;

#[derive(Clone, Copy)]
pub struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        Uuid::new_v4().to_string().parse().map(RequestId::new).ok()
    }
}
