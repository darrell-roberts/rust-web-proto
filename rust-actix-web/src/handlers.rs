//! Router handler functions
use crate::types::{AdminAccess, HandlerError, UserAccess};
use actix_http::{ResponseBuilder, StatusCode};
use actix_web::{
    get, post, put,
    web::{self, Bytes},
    HttpResponse, HttpResponseBuilder, Responder, Result,
};
use futures::{stream, StreamExt as _, TryStreamExt};
use std::{future, sync::Arc};
use tracing::{debug, error};
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

#[get("download")]
pub async fn download_users(db: Database, _claims: AdminAccess) -> HttpResponse {
    let header = stream::iter(std::iter::once(Ok(Bytes::from_static(b"["))));
    let footer = stream::iter(std::iter::once(Ok(Bytes::from_static(b"]"))));

    let body = db
        .download()
        .await
        .inspect_err(|err| error!("Failed to read user record {err}"))
        .filter_map(|r| future::ready(r.ok()))
        .enumerate()
        .map(|(index, u)| {
            serde_json::to_vec(&u)
                .map(|bytes| {
                    if index > 0 {
                        Vec::from_iter([b','].into_iter().chain(bytes))
                    } else {
                        bytes
                    }
                })
                .map(Bytes::from)
        });

    HttpResponseBuilder::new(StatusCode::OK)
        .append_header(("Content-Type", "application/json"))
        .streaming(header.chain(body).chain(footer))
}
