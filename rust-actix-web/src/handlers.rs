use crate::{
    common::USER_MS_TARGET,
    types::{AdminAccess, HandlerError, UserAccess},
};
use actix_http::{ResponseBuilder, StatusCode};
use actix_web::{get, post, put, web, Responder, Result};
use std::sync::Arc;
use tracing::{event, Level};
use user_persist::{
    persistence::UserPersistence,
    types::{UpdateUser, User, UserKey, UserSearch},
};

type Persist = web::Data<Arc<dyn UserPersistence>>;

#[get("{id}")]
pub async fn get_user(
    db: Persist,
    id: web::Path<UserKey>,
    claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    event!(
      target: USER_MS_TARGET,
      Level::DEBUG,
      "Received id: {id:?} with claims: {claims:?}"
    );

    let user = db.get_user(&id).await?;

    event!(target: USER_MS_TARGET, Level::DEBUG, "db result: {user:?}");

    Ok(web::Json(user))
}

#[post("")]
pub async fn save_user(
    user: web::Json<User>,
    db: Persist,
    _claims: UserAccess,
) -> Result<impl Responder, HandlerError> {
    event!(
      target: USER_MS_TARGET,
      Level::DEBUG,
      "saving user: {user:?}"
    );
    let saved_user = db.save_user(&user).await?;
    Ok(web::Json(saved_user))
}

#[put("")]
pub async fn update_user(
    db: Persist,
    user: web::Json<UpdateUser>,
    _claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    event!(
      target: USER_MS_TARGET,
      Level::DEBUG,
      "updating user with {user:?}"
    );
    db.update_user(&user).await?;
    Ok(ResponseBuilder::new(StatusCode::OK))
}

#[post("/search")]
pub async fn search_users(
    user_search: web::Json<UserSearch>,
    db: Persist,
    _claims: AdminAccess,
) -> Result<impl Responder, HandlerError> {
    event!(
      target: USER_MS_TARGET,
      Level::DEBUG,
      "Searching for users with {user_search:?}"
    );
    let results = db.search_users(&user_search).await?;
    Ok(web::Json(results))
}

#[get("counts")]
pub async fn count_users(db: Persist, claims: AdminAccess) -> Result<impl Responder, HandlerError> {
    event!(target: USER_MS_TARGET, Level::DEBUG, "Claims: {claims:?}");
    let counts = db.count_genders().await?;
    event!(
      target: USER_MS_TARGET,
      Level::DEBUG,
      "User counts: {counts:?}"
    );
    Ok(web::Json(counts))
}
