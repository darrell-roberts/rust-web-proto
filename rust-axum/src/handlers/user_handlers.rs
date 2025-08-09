//! Route handles for the user API.
use crate::{
    extractors::{hashing::HashedValidatingJson, validator::ValidatingJson},
    types::{
        handler::{HandlerError, Persist},
        jwt::{AdminAccess, UserAccess},
    },
};
use axum::{
    body::Body,
    extract::{Extension, Json, Path},
    response::IntoResponse,
};
use futures::stream::{self, StreamExt};
use http::{Response, StatusCode};
use serde_json::{to_string, Value};
use std::sync::Arc;
use tracing::debug;
use user_persist::{
    mongo_persistence::MongoPersistence,
    persistence::UserPersistence,
    types::{UpdateUser, User, UserKey, UserSearch},
};

type HandlerResult<T> = Result<T, HandlerError>;

/// Get user handler.
pub async fn get_user<P>(
    db: Persist<P>,
    Path(id): Path<UserKey>,
    claims: AdminAccess,
) -> Result<Json<User>, HandlerError>
where
    P: UserPersistence,
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
    db: Persist<P>,
    _claims: UserAccess,
    ValidatingJson(user): ValidatingJson<User>,
) -> Result<Json<User>, HandlerError>
where
    P: UserPersistence,
{
    debug!("saving user: {user}");
    let user = db.save_user(&user).await.map_err(HandlerError::from)?;
    Ok(Json(user))
}

/// Update user handler.
pub async fn update_user<P>(
    db: Persist<P>,
    _claims: AdminAccess,
    HashedValidatingJson(user): HashedValidatingJson<UpdateUser>,
) -> HandlerResult<StatusCode>
where
    P: UserPersistence,
{
    debug!("updating user with {user}");
    db.update_user(&user)
        .await
        .map(|_| StatusCode::OK)
        .map_err(HandlerError::from)
}

/// Search users handler.
pub async fn search_users<P>(
    db: Persist<P>,
    claims: AdminAccess,
    ValidatingJson(user_search): ValidatingJson<UserSearch>,
) -> Result<Json<Vec<User>>, HandlerError>
where
    P: UserPersistence,
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
    db: Persist<P>,
    Path(id): Path<UserKey>,
    _claims: AdminAccess,
) -> impl IntoResponse
where
    P: UserPersistence,
{
    match db.remove_user(&id).await {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(e) => HandlerError::from(e).into_response(),
    }
}

/// Count users handler.
pub async fn count_users<P>(db: Persist<P>, claims: AdminAccess) -> HandlerResult<Json<Vec<Value>>>
where
    P: UserPersistence,
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
pub async fn download_users(
    db: Extension<Arc<MongoPersistence>>,
    claims: AdminAccess,
) -> HandlerResult<impl IntoResponse> {
    debug!("Streaming users for {claims}");

    // Chain my stream with a header and footer
    // in order to reconstitute a json array for
    // the mongodb stream of documents returned.
    let header = stream::iter(vec![Ok("[".to_string())]);
    let footer = stream::iter(vec![Ok("]".to_string())]);

    let stream = db
        .download()
        .await?
        .filter_map(|r| async { r.ok() })
        .map(|u| to_string(&u).map(|s| format!("{s},")));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from_stream(header.chain(stream).chain(footer)))
        .unwrap())
}
