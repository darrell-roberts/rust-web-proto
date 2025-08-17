//! This module provides data access to a a mongodb user collection.
use crate::{
    database::{DatabaseResult, UserDatabase},
    init_mongo_client,
    types::{Email, Gender, UpdateUser, User, UserKey, UserSearch},
    MongoArgs,
};
use futures::stream::{Stream, TryStreamExt};
use mongodb::{
    bson::{doc, oid::ObjectId, Bson, Document},
    results::InsertOneResult,
    Collection, Database,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;
use tracing::{debug, instrument};

const COLLECTION_NAME: &str = "users";

/// An implementation of UserDatabase for MongoDB.
#[derive(Debug, Clone)]
pub struct MongoDatabase(Database);

impl Deref for MongoDatabase {
    type Target = Database;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MongoDatabase {
    /// Creates a new MongoDatabase API.
    pub async fn new(options: MongoArgs) -> DatabaseResult<Self> {
        let db = init_mongo_client(options).await?;
        Ok(Self(db))
    }
}

impl UserDatabase for MongoDatabase {
    async fn get_user(&self, id: &UserKey) -> DatabaseResult<Option<User>> {
        let user = self
            .user_collection()
            .find_one(doc! {"_id": ObjectId::try_from(id)?})
            .await?
            .map(User::from);

        Ok(user)
    }

    async fn save_user(&self, user: &User) -> DatabaseResult<User> {
        let mongo_user = MongoUser::from(user.to_owned());

        let InsertOneResult { inserted_id, .. } =
            self.user_collection().insert_one(mongo_user).await?;

        let key = match inserted_id {
            Bson::ObjectId(k) => Some(k),
            _ => None,
        };

        Ok(User {
            id: key.map(UserKey::from),
            ..user.clone()
        })
    }

    async fn update_user(&self, user: &UpdateUser) -> DatabaseResult<()> {
        let query = doc! {"_id": ObjectId::try_from(&user.id)?};
        let update_fields = doc! {"name": &user.name, "age": &user.age, "email": &user.email};
        let update = doc! {"$set": update_fields};

        let updated = self.user_collection().update_one(query, update).await?;

        debug!("update result: {updated:?}",);

        Ok(())
    }

    async fn remove_user(&self, key: &UserKey) -> DatabaseResult<()> {
        let result = self
            .user_collection()
            .delete_one(doc! {
              "_id": ObjectId::try_from(key)?
            })
            .await?;
        debug!("delete result: {result:?}");
        Ok(())
    }

    #[instrument(skip_all, level = "debug", target = "database", name = "search-span")]
    async fn search_users(&self, user_search: &UserSearch) -> DatabaseResult<Vec<User>> {
        let search = doc! { "email": &user_search.email, "gender": &user_search.gender,
            "name": &user_search.name
        };

        let filtered_null = search
            .into_iter()
            .filter(|(_, value)| value != &Bson::Null)
            .collect::<Document>();

        debug!("mongo search query: {filtered_null}",);

        let result = self
            .user_collection()
            .find(filtered_null)
            .await?
            .try_collect::<Vec<MongoUser>>()
            .await?
            .into_iter()
            .map(User::from)
            .collect::<Vec<_>>();

        Ok(result)
    }

    async fn count_genders(&self) -> DatabaseResult<Vec<Value>> {
        let pipeline = vec![doc! {
          "$group": {"_id": "$gender", "count": {"$count": {}}}
        }];

        let docs = self
            .collection::<Document>(COLLECTION_NAME)
            .aggregate(pipeline.into_iter())
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(Bson::from)
            .map(Value::from)
            .collect();

        Ok(docs)
    }

    async fn download(
        &self,
    ) -> DatabaseResult<impl Stream<Item = DatabaseResult<User>> + 'static + Send> {
        Ok(self
            .user_collection()
            .find(doc! {})
            .await?
            .map_ok(User::from)
            .map_err(Into::into))
    }
}

impl MongoDatabase {
    /// Get the user collection.
    pub(crate) fn user_collection(&self) -> Collection<MongoUser> {
        self.collection::<MongoUser>(COLLECTION_NAME)
    }
}

impl From<UserKey> for Bson {
    fn from(user_key: UserKey) -> Self {
        ObjectId::parse_str(user_key.0)
            .map(Bson::ObjectId)
            .unwrap_or_else(|_| Bson::Null)
    }
}

impl From<Gender> for Bson {
    fn from(gender: Gender) -> Self {
        match gender {
            Gender::Male => Bson::String(String::from("Male")),
            Gender::Female => Bson::String(String::from("Female")),
        }
    }
}

impl From<Email> for Bson {
    fn from(email: Email) -> Self {
        Bson::String(email.0)
    }
}

/// User type as it is saved in mongodb.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MongoUser {
    #[serde(skip_serializing)]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub age: u32,
    pub email: String,
    pub gender: Gender,
}

impl From<MongoUser> for User {
    fn from(mongo_user: MongoUser) -> Self {
        User {
            id: mongo_user._id.as_ref().map(|u| UserKey::from(*u)),
            name: mongo_user.name,
            age: mongo_user.age,
            email: Email(mongo_user.email),
            gender: mongo_user.gender,
        }
    }
}

impl From<User> for MongoUser {
    fn from(user: User) -> Self {
        MongoUser {
            _id: None,
            name: user.name,
            age: user.age,
            email: user.email.0,
            gender: user.gender,
        }
    }
}

impl TryFrom<&UserKey> for ObjectId {
    type Error = mongodb::bson::oid::Error;
    fn try_from(user_key: &UserKey) -> Result<Self, Self::Error> {
        ObjectId::parse_str(&user_key.0)
    }
}
