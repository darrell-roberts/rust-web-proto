#[macro_use]
extern crate rocket;

mod catchers;
mod fairings;
mod guards;
mod routes;
#[cfg(test)]
mod tests;
mod types;

use crate::types::{JWTClaims, Role};
use chrono::{Duration, Utc};
use clap::Parser;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;
use std::{fmt, process, sync::Arc};
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;
use user_database::{database::UserDatabaseDynSafe, mongo_database::MongoDatabase, MongoArgs};

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

type HmacSha256 = Hmac<Sha256>;

fn test_jwt(role: Role) -> String {
    let key = HmacSha256::new_from_slice(TEST_JWT_SECRET).unwrap();
    let expiration = Utc::now() + Duration::minutes(15);
    let claims = JWTClaims {
        sub: "somebody".to_owned(),
        role,
        exp: expiration.timestamp(),
    };
    format!("Bearer {}", claims.sign_with_key(&key).unwrap())
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

    let program_opts = ProgramArgs::parse();

    event!(
      target: types::USER_MS_TARGET,
      Level::INFO,
      "mongo_args: {program_opts}"
    );

    event!(
      target: types::USER_MS_TARGET,
      Level::DEBUG,
      "admin {}",
      test_jwt(Role::Admin)
    );

    match MongoDatabase::new(program_opts.mongo_opts).await {
        Ok(db) => {
            let mongo_database: Arc<dyn UserDatabaseDynSafe> = Arc::new(db);

            let _ = rocket::build()
                .attach(fairings::RequestIdFairing)
                .attach(fairings::LoggerFairing)
                .attach(fairings::RequestTimer)
                .manage(mongo_database)
                .mount(
                    "/api/v1/user",
                    routes![
                        routes::count_genders,
                        routes::get_user,
                        routes::save_user,
                        routes::find_users,
                        routes::update_user,
                        // routes::download
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
                .unwrap();
        }
        Err(e) => {
            error!("Failed to connect to database: {e}");
            process::exit(1);
        }
    };
}
