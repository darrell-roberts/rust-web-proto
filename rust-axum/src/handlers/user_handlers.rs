//! Route handles for the user API.
use crate::{
    extractors::{hashing::HashedValidatingJson, validator::ValidatingJson},
    security::hashing::{HashableVector, HashingResponse},
    types::{
        handler::{HandlerError, Persist},
        jwt::{AdminAccess, UserAccess},
    },
    AppConfig,
};
use axum::{
    body::Body,
    extract::{Extension, Json, Path},
    response::IntoResponse,
};
use futures::stream::{self, StreamExt};
use http::{Response, StatusCode};
use serde_json::{to_string, Value};
use std::{fmt::Display, sync::Arc};
use tracing::debug;
use user_persist::{
    mongo_persistence::MongoPersistence,
    persistence::UserPersistence,
    types::{UpdateUser, User, UserKey, UserSearch},
};

type HandlerResult<T> = Result<T, HandlerError>;
type AppCfg = Extension<Arc<AppConfig>>;

/// Get user handler.
pub async fn get_user<P>(
    db: Persist<P>,
    Path(id): Path<UserKey>,
    claims: AdminAccess,
    Extension(app_config): AppCfg,
) -> impl IntoResponse
where
    P: UserPersistence,
{
    debug!("Received id: {id} with claims: {claims}");

    let user = db.get_user(&id).await?;

    debug!("db result: {}", {
        let s: &dyn std::fmt::Display = match user {
            Some(ref u) => u,
            None => &"No User" as &(dyn Display + 'static),
        };
        s
    });

    user.map(|u| HashingResponse::new(app_config, u))
        .ok_or(HandlerError::ResourceNotFound)
}

/// Save user handler.
/// #[axum_macros::debug_handler]
pub async fn save_user<P>(
    db: Persist<P>,
    _claims: UserAccess,
    Extension(app_config): AppCfg,
    ValidatingJson(user): ValidatingJson<User>,
) -> impl IntoResponse
where
    P: UserPersistence,
{
    debug!("saving user: {user}");
    db.save_user(&user)
        .await
        .map_err(HandlerError::from)
        .map(|u| HashingResponse::new(app_config, u))
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
    Extension(app_config): AppCfg,
    ValidatingJson(user_search): ValidatingJson<UserSearch>,
) -> impl IntoResponse
where
    P: UserPersistence,
{
    debug!("Searching for users with {user_search} and claims {claims}");
    db.search_users(&user_search)
        .await
        .map(|v| HashableVector::new(app_config, v))
        .map_err(HandlerError::from)
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

    let response_stream = header.chain(stream).chain(footer);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from_stream(response_stream))
        .unwrap())
}
