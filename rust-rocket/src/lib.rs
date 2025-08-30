//! Rocket user service.

use std::sync::Arc;

use crate::types::{JWTClaims, Role};
use chrono::{Duration, Utc};
use hmac::{digest::KeyInit as _, Hmac};
use jwt::SignWithKey as _;
use sha2::Sha256;
use user_database::database::UserDatabaseDynSafe;

#[macro_use]
extern crate rocket;

pub mod catchers;
pub mod fairings;
mod guards;
pub mod routes;
pub mod types;

const FRAMEWORK_TARGET: &str = "ms-framework";
// This would be sourced from some vault service.
pub const TEST_JWT_SECRET: &[u8] = b"TEST_SECRET";

type HmacSha256 = Hmac<Sha256>;

/// Create a test JWT for a given role.
pub fn test_jwt(role: Role) -> String {
    let key = HmacSha256::new_from_slice(TEST_JWT_SECRET).unwrap();
    let expiration = Utc::now() + Duration::minutes(15);
    let claims = JWTClaims {
        sub: "somebody".to_owned(),
        role,
        exp: expiration.timestamp(),
    };
    format!("Bearer {}", claims.sign_with_key(&key).unwrap())
}

/// Create a rocket server
pub fn rocket(db: Arc<dyn UserDatabaseDynSafe>) -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .attach(fairings::RequestIdFairing)
        .attach(fairings::LoggerFairing)
        .attach(fairings::RequestTimer)
        .manage(db)
        .mount(
            "/api/v1/user",
            routes![
                routes::count_genders,
                routes::download,
                routes::get_user,
                routes::save_user,
                routes::find_users,
                routes::update_user,
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
}
