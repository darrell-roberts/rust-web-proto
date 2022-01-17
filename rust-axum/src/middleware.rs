use http::Request;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct MakeRequestUuid;


impl MakeRequestId for MakeRequestUuid {
  fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
    let request_id = Uuid::new_v4().to_string().parse().unwrap();
    Some(RequestId::new(request_id))
  }
}
