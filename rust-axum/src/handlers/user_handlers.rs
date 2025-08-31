//! Route handles for the user API.
use crate::{
    extractors::{hashing::HashedValidatingJson, validator::ValidatingJson},
    types::{
        handler::{Database, HandlerError},
        jwt::{AdminAccess, UserAccess},
    },
};
use axum::{
    body::{Body, Bytes},
    extract::{Json, Path},
    response::IntoResponse,
};
use futures::{
    stream::{self, StreamExt},
    TryStreamExt,
};
use http::{Response, StatusCode};
use serde_json::Value;
use tracing::{debug, error};
use user_database::{
    database::UserDatabase,
    types::{UpdateUser, User, UserKey, UserSearch},
};

/// Handler result that fails with `HandlerError`.
type HandlerResult<T> = Result<T, HandlerError>;

/// Get user handler.
pub async fn get_user<P>(
    db: Database<P>,
    Path(id): Path<UserKey>,
    claims: AdminAccess,
) -> HandlerResult<Json<User>>
where
    P: UserDatabase,
{
    debug!("Received id: {id} with claims: {claims}");
    let user = db
        .get_user(&id)
        .await
        .map_err(HandlerError::from)?
        .ok_or(HandlerError::ResourceNotFound)?;
    debug!("db result: {user}");
    Ok(Json(user))
}

/// Save user handler.
/// #[axum_macros::debug_handler]
pub async fn save_user<P>(
    db: Database<P>,
    _claims: UserAccess,
    ValidatingJson(user): ValidatingJson<User>,
) -> HandlerResult<Json<User>>
where
    P: UserDatabase,
{
    debug!("saving user: {user}");
    let user = db.save_user(&user).await.map_err(HandlerError::from)?;
    Ok(Json(user))
}

/// Update user handler.
pub async fn update_user<P>(
    db: Database<P>,
    _claims: AdminAccess,
    HashedValidatingJson(user): HashedValidatingJson<UpdateUser>,
) -> HandlerResult<StatusCode>
where
    P: UserDatabase,
{
    debug!("updating user with {user}");
    db.update_user(&user)
        .await
        .map(|_| StatusCode::OK)
        .map_err(HandlerError::from)
}

/// Search users handler.
pub async fn search_users<P>(
    db: Database<P>,
    claims: AdminAccess,
    ValidatingJson(user_search): ValidatingJson<UserSearch>,
) -> HandlerResult<Json<Vec<User>>>
where
    P: UserDatabase,
{
    debug!("Searching for users with {user_search} and claims {claims}");
    let users = db
        .search_users(&user_search)
        .await
        .map_err(HandlerError::from)?;
    Ok(Json(users))
}

/// Delete user handler.
pub async fn delete_user<P>(
    db: Database<P>,
    Path(id): Path<UserKey>,
    _claims: AdminAccess,
) -> HandlerResult<StatusCode>
where
    P: UserDatabase,
{
    debug!("Deleting user: {id}");
    db.remove_user(&id).await.map_err(HandlerError::from)?;
    Ok(StatusCode::OK)
}

/// Count users handler.
pub async fn count_users<P>(db: Database<P>, claims: AdminAccess) -> HandlerResult<Json<Vec<Value>>>
where
    P: UserDatabase,
{
    debug!("Claims: {claims}");
    let counts = db.count_genders().await?;
    debug!("User counts: {counts:?}");
    Ok(Json(counts))
}

// This gets a stream of MongoUser types that are
// streamed from the mongodb cursor. The stream is
// transformed to it's JSON form and wrapped in a
// StreamBody resulting in a Stream from mongodb back
// to http client.

/// Download users handler
pub async fn download_users<P>(
    db: Database<P>,
    claims: AdminAccess,
) -> HandlerResult<impl IntoResponse>
where
    P: UserDatabase,
{
    debug!("Streaming users for {claims}");

    // Chain my stream with a header and footer
    // in order to reconstitute a json array for
    // the mongodb stream of documents returned.
    let header = stream::iter(vec![Ok(Bytes::from_static(b"["))]);
    let footer = stream::iter(vec![Ok(Bytes::from_static(b"]"))]);

    let body = db
        .download()
        .await
        .inspect_err(|err| error!("Failed to read user record {err}"))
        .filter_map(|r| async { r.ok() })
        .enumerate()
        .map(|(index, u)| {
            serde_json::to_string(&u)
                .map(|s| if index > 0 { format!(",{s}") } else { s })
                .map(Bytes::from)
        });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from_stream(header.chain(body).chain(footer)))
        .unwrap())
}
