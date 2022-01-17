// use crate::types::LocalUser;
// use actix_web::{Error, HttpRequest, HttpResponse, Responder};
// use futures::future::{ready, Ready};


// impl Responder for LocalUser {
//   type Error = Error;
//   type Future = Ready<Result<HttpResponse, Error>>;

//   fn respond_to(self, _req: &HttpRequest) -> Self::Future {
//     let body = serde_json::to_string(&self.0).unwrap();

//     ready(Ok(
//       HttpResponse::Ok()
//         .content_type("application/json")
//         .body(body),
//     ))
//   }
// }
