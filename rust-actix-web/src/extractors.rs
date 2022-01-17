// use actix_web::{
//   dev::Payload, web::HttpRequest, web::Json, Error, FromRequest,
// };

// use core::future::Future;
// use core::result::Result;
// use serde::Deserialize;
// use validator::Validate;

// pub struct ValidatingJson<T>(pub Json<T>);

// impl<T> FromRequest for ValidatingJson<T> {
//   type Error = Error;
//   type Future = Future<Output = Result<Self, Self::Error>>;

//   fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {

//   }
// }
