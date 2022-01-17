#[macro_use]
extern crate rocket;

mod catchers;
mod fairings;
mod guards;
mod routes;
#[cfg(test)]
mod tests;
mod types;

use clap::Parser;
use std::fmt;
use std::process;
use std::sync::Arc;
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_persist::init_mongo_client;
use user_persist::mongo_persistence::MongoPersistence;
use user_persist::persistence::UserPersistence;
use user_persist::MongoArgs;

// This would be sourced from some vault service.
const TEST_JWT_SECRET: &[u8] = b"TEST_SECRET";
const FRAMEWORK_TARGET: &str = "ms-framework";

#[derive(Parser, Debug, Clone)]
#[clap(about, version, author)]
struct ProgramArgs {
  #[clap(flatten)]
  mongo_opts: MongoArgs,
}

impl fmt::Display for ProgramArgs {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "mongo_opts {}", self.mongo_opts)
  }
}

#[rocket::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .with_target(true)
    .pretty()
    // .json()
    // .flatten_event(true)
    .init();

  let mongo_args = ProgramArgs::parse();

  event!(
    target: types::USER_MS_TARGET,
    Level::INFO,
    "mongo_args: {mongo_args}"
  );

  match init_mongo_client(mongo_args.mongo_opts) {
    Ok(db) => {
      let mongo_persist: Arc<dyn UserPersistence> =
        Arc::new(MongoPersistence::new(db.clone()));

      rocket::build()
        .attach(fairings::RequestIdFairing)
        .attach(fairings::LoggerFairing)
        .attach(fairings::RequestTimer)
        .manage(mongo_persist)
        .manage(MongoPersistence::new(db))
        .mount(
          "/api/v1/user",
          routes![
            routes::count_genders,
            routes::get_user,
            routes::save_user,
            routes::find_users,
            routes::update_user,
            routes::download
          ],
        )
        .register(
          "/api/v1/user",
          catchers![
            catchers::not_found,
            catchers::bad_request,
            catchers::unprocessable_entry,
            catchers::internal_server_error,
            catchers::not_authorized
          ],
        )
        .launch()
        .await
        .unwrap()
    }
    Err(e) => {
      error!("Failed to connect to database: {}", e);
      process::exit(1);
    }
  }
}
