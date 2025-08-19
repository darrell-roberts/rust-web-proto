//! Router handler functions
use crate::types::{AdminAccess, HandlerError, UserAccess};
use actix_http::{ResponseBuilder, StatusCode};
use actix_web::{get, post, put, web, Responder, Result};
use std::sync::Arc;
use tracing::debug;
use user_database::{
    database::UserDatabaseDynSafe,
    types::{UpdateUser, User, UserKey, UserSearch},
};

/// Database api from application state
type Database = web::Data<Arc<dyn UserDatabaseDynSafe>>;

#[get("{id}")]
pub async fn get_user(
    db: Database,
    id: web::Path<UserKey>,
    claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    debug!("Received id: {id:?} with claims: {claims:?}");
    let user = db.get_user(&id).await?;
    debug!("db result: {user:?}");
    Ok(web::Json(user))
}

#[post("")]
pub async fn save_user(
    user: web::Json<User>,
    db: Database,
    _claims: UserAccess,
) -> Result<impl Responder, HandlerError> {
    debug!("saving user: {user:?}");
    let saved_user = db.save_user(&user).await?;
    Ok(web::Json(saved_user))
}

#[put("")]
pub async fn update_user(
    db: Database,
    user: web::Json<UpdateUser>,
    _claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    debug!("updating user with {user:?}");
    db.update_user(&user).await?;
    Ok(ResponseBuilder::new(StatusCode::OK))
}

#[post("/search")]
pub async fn search_users(
    user_search: web::Json<UserSearch>,
    db: Database,
    _claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    debug!("Searching for users with {user_search:?}");
    let results = db.search_users(&user_search).await?;
    Ok(web::Json(results))
}

#[get("counts")]
pub async fn count_users(
    db: Database,
    claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    debug!("Claims: {claims:?}");
    let counts = db.count_genders().await?;
    debug!("User counts: {counts:?}");
    Ok(web::Json(counts))
}
