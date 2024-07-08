use crate::FRAMEWORK_TARGET;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::outcome::Outcome::Success;
use rocket::request::{FromRequest, Outcome};
use rocket::{Data, Request, Response};
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::time::SystemTime;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct RequestId(pub Option<Uuid>);

#[derive(Copy, Clone, Debug)]
struct TimerStart(Option<SystemTime>);

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.unwrap_or_default())
    }
}

pub struct RequestIdFairing;
pub struct LoggerFairing;
pub struct RequestTimer;

#[rocket::async_trait]
impl Fairing for RequestTimer {
    fn info(&self) -> Info {
        Info {
            name: "Request timer",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        req.local_cache(|| TimerStart(Some(SystemTime::now())));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, _res: &mut Response<'r>) {
        let req_id = req.local_cache(|| RequestId(None));
        let TimerStart(start_time) = req.local_cache(|| TimerStart(None));
        if let Some(Ok(duration)) = start_time.map(|st| st.elapsed()) {
            let ms = duration.as_secs() * 1000 + duration.subsec_millis() as u64;
            event!(
              target: FRAMEWORK_TARGET,
              Level::INFO,
              %req_id,
              "{} {} completed in {} ms",
              req.method(),
              req.uri(),
              ms
            );
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestId {
    type Error = Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let r_id = *req.local_cache(|| RequestId(None));
        Success(r_id)
    }
}

/// Fairing that attemps to get a X-Request-Id header and store it in
/// request local cache otherwise creates a new UUID to store in request
/// local cache. UUID is also sent back as a X-Request-Id response header.
#[rocket::async_trait]
impl Fairing for RequestIdFairing {
    fn info(&self) -> Info {
        Info {
            name: "Request Id",
            kind: Kind::Request | Kind::Response,
        }
    }

    #[instrument(
        skip_all,
        level = "debug",
        target = "ms-framework",
        name = "request-span"
    )]
    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        match req.headers().get_one("X-Request-Id") {
            Some(h) => {
                let req_id = Uuid::parse_str(h).unwrap_or_else(|_| Uuid::new_v4());
                req.local_cache(|| RequestId(Some(req_id)));
            }
            None => {
                req.local_cache(|| RequestId(Some(Uuid::new_v4())));
            }
        }
    }

    /// Take the requestId from the request local cache and add it to the response
    /// header.
    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let RequestId(req_id) = req.local_cache(|| RequestId(None));
        res.set_header(Header::new("X-Request-Id", req_id.unwrap().to_string()));
    }
}

/// Fairing that logs on start request /end response.
#[rocket::async_trait]
impl Fairing for LoggerFairing {
    fn info(&self) -> Info {
        Info {
            name: "Req/Res Logger",
            kind: Kind::Request | Kind::Response,
        }
    }

    // Log incoming requests.
    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        let req_id = req.local_cache(|| RequestId(None));
        event!(
          target: FRAMEWORK_TARGET,
          Level::INFO,
          %req_id,
          "request start: {} {}",
          req.method(),
          req.uri()
        )
    }

    // Log outgoing requests.
    async fn on_response<'r>(&self, req: &'r Request<'_>, _res: &mut Response<'r>) {
        let req_id = req.local_cache(|| RequestId(None));
        event!(target: FRAMEWORK_TARGET, Level::INFO, %req_id,
      "request end: {} {}", req.method(), req.uri())
    }
}
