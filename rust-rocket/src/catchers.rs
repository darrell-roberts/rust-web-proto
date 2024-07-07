use crate::{guards::UserErrorMessage, types::USER_MS_TARGET};
use rocket::{
    serde::json::{json, Value},
    Request,
};
use tracing::{event, Level};
use user_persist::ValidationErrors;

#[catch(403)]
pub fn not_authorized() -> Value {
    json!([{"label": "unauthorized", "message": "Not authorized to make request"}])
}

#[catch(404)]
pub fn not_found() -> Value {
    json!([])
}

#[catch(422)]
pub fn unprocessable_entry(req: &Request) -> Value {
    event!(
      target: USER_MS_TARGET,
      Level::WARN,
      "Returning error responder for {}",
      req.uri()
    );
    json! [{"label": "failed.request", "message": "failed to service request"}]
}

#[catch(400)]
pub fn bad_request(req: &Request) -> Value {
    let validation_errors = req.local_cache::<Option<ValidationErrors>, _>(|| None);
    let message = match validation_errors {
        Some(_) => "validation failed",
        None => "invalid or malformed request",
    };

    event!(
      target: USER_MS_TARGET,
      Level::WARN,
      "Invalid request for {}",
      req.uri()
    );
    json! [{"label": "bad.request", "message": message, "validation": validation_errors}]
}

#[catch(500)]
pub fn internal_server_error(req: &Request) -> Value {
    let error_message =
        req.local_cache(|| Some(UserErrorMessage("Internal server error".to_owned())));

    event!(
      target: USER_MS_TARGET,
      Level::ERROR,
      "Internal server error for {} {}",
      req.method(),
      req.uri()
    );

    json! [{"label": "internal.error", "message": error_message}]
}
