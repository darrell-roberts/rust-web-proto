use crate::{
    fairings::RequestId,
    types::{AdminAccess, ErrorResponder, JsonValidation, UserAccess, UserKeyReq, USER_MS_TARGET},
};
use futures::StreamExt as _;
use mongodb::bson::doc;
use rocket::{response::stream::ByteStream, serde::json::Json, State};
use serde_json::Value;
use std::sync::Arc;
use tracing::{event, Level};
use user_database::{
    database::UserDatabaseDynSafe,
    types::{UpdateUser, User, UserSearch},
};

type JsonUser = Json<User>;
type HandlerResult<T> = Result<T, ErrorResponder<'static>>;
type UserDatabase = State<Arc<dyn UserDatabaseDynSafe>>;

// Gets a single user document by primary key.
#[get("/<id>")]
pub async fn get_user(
    id: UserKeyReq,
    req_id: RequestId,
    db: &UserDatabase,
    role: AdminAccess,
) -> HandlerResult<Option<JsonUser>> {
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "claims: {role:?}");
    let user = db.get_user(&id.0).await?;
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "fetched user: {user:?}");
    Ok(user.map(Json))
}

// Creates a new user record.
#[post("/", format = "json", data = "<user>")]
pub async fn save_user(
    user: JsonValidation<User>,
    req_id: RequestId,
    db: &UserDatabase,
    _role: UserAccess,
) -> HandlerResult<JsonUser> {
    let JsonValidation(u) = user;
    let saved_user = db.save_user(&u).await?;
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "Saved user {saved_user:?}");
    Ok(Json(saved_user))
}

// Updates a user with the UpdateUser criteria.
#[put("/", format = "json", data = "<user>")]
pub async fn update_user(
    db: &UserDatabase,
    req_id: RequestId,
    user: JsonValidation<UpdateUser>,
    #[allow(unused)] role: AdminAccess,
) -> HandlerResult<()> {
    let JsonValidation(u) = user;
    db.update_user(&u).await?;
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "Updated user {u:?}");
    Ok(())
}

// Runs an aggregation pipeline to group the users by gender
// and summarize counts.
#[get("/counts")]
pub async fn count_genders(
    db: &UserDatabase,
    req_id: RequestId,
    #[allow(unused)] role: UserAccess,
) -> HandlerResult<Json<Vec<Value>>> {
    let docs = db.count_genders().await?;
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "User counts: {docs:?}");
    Ok(Json(docs))
}

// Searches for users with the UserSearch criteria.
#[tracing::instrument(skip(db), level = "debug", target = "user-ms", name = "search-span")]
#[post("/search", format = "json", data = "<user_search>")]
pub async fn find_users(
    user_search: JsonValidation<UserSearch>,
    req_id: RequestId,
    db: &UserDatabase,
    role: AdminAccess,
) -> HandlerResult<Json<Vec<User>>> {
    let search = user_search.0;
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "Searching with {search:?}");
    let result = db.search_users(&search).await?;
    event!(target: USER_MS_TARGET, Level::DEBUG, %req_id, "Found {result:?}");
    Ok(Json(result))
}

// Stream all users as json.
#[get("/download")]
pub async fn download(
    db: &UserDatabase,
    _req_id: RequestId,
    _role: AdminAccess,
) -> ByteStream![Vec<u8>] {
    let stream = db.download().await.map(|result| match result {
        Ok(user) => serde_json::to_vec(&user).unwrap_or_default(),
        Err(err) => {
            error!("Failed to stream user: {err}");
            Vec::new()
        }
    });

    ByteStream::from(stream)
}
