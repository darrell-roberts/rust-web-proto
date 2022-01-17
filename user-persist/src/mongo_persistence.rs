use crate::persistence::{UserPersistence, PersistenceResult};
use crate::types::{Email, Gender, UpdateUser, User, UserKey, UserSearch};
use crate::PERSISTENCE_TARGET;
use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{Stream, TryStreamExt};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, Bson, Document};
use mongodb::options::AggregateOptions;
use mongodb::results::InsertOneResult;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};

const COLLECTION_NAME: &str = "users";

// An implementation of persistence for MongoDB.
#[derive(Debug, Clone)]
pub struct MongoPersistence {
  db: Database,
}

impl MongoPersistence {
  pub fn new(db: Database) -> Self {
    Self { db }
  }
}

#[async_trait]
impl UserPersistence for MongoPersistence {
  async fn get_user(&self, id: &UserKey) -> PersistenceResult<Option<User>> {
    let some_user = self
      .db
      .collection::<MongoUser>(COLLECTION_NAME)
      .find_one(doc! {"_id": id}, None)
      .await?;

    let user = some_user.map(User::from);

    Ok(user)
  }

  async fn save_user(&self, user: &User) -> PersistenceResult<User> {
    let mongo_user = MongoUser::from(user);

    let InsertOneResult { inserted_id, .. } = self
      .db
      .collection::<MongoUser>(COLLECTION_NAME)
      .insert_one(mongo_user, None)
      .await?;

    let key = match inserted_id {
      Bson::ObjectId(k) => Some(k),
      _ => None,
    };

    Ok(User {
      id: key.map(|k| k.to_string()),
      ..user.clone()
    })
  }

  async fn update_user(&self, user: &UpdateUser) -> PersistenceResult<()> {
    let oid = ObjectId::parse_str(&user.id.0)?;
    let query = doc! {"_id": oid};
    let update_fields = doc! {"name": user.name.clone(), "age": user.age};
    let update = doc! {"$set": update_fields};

    let updated = self
      .db
      .collection::<MongoUser>(COLLECTION_NAME)
      .update_one(query, update, None)
      .await?;

    event!(
      target: PERSISTENCE_TARGET,
      Level::DEBUG,
      "update result: {:?}",
      updated
    );

    Ok(())
  }

  #[instrument(
    skip_all,
    level = "debug",
    target = "persistence",
    name = "search-span"
  )]
  async fn search_users(
    &self,
    user_search: &UserSearch,
  ) -> PersistenceResult<Vec<User>> {
    let search = doc! { "email": &user_search.email, "gender": &user_search.gender,
        "name": &user_search.name
    };
    event!(
      target: PERSISTENCE_TARGET,
      Level::DEBUG,
      "search query: {search}"
    );
    let filtered_null = search
      .into_iter()
      .filter(|(_, value)| value != &Bson::Null)
      .collect::<Document>();

    event!(
      target: PERSISTENCE_TARGET,
      Level::DEBUG,
      "search query: {}",
      filtered_null
    );

    let result = self
      .db
      .collection::<MongoUser>(COLLECTION_NAME)
      .find(filtered_null, None)
      .await?
      .try_collect::<Vec<MongoUser>>()
      .await?
      .into_iter()
      .map(User::from)
      .collect::<Vec<User>>();

    Ok(result)
  }

  async fn count_genders(&self) -> PersistenceResult<Vec<Value>> {
    let pipeline = vec![doc! {
      "$group": {"_id": "$gender", "count": {"$count": {}}}
    }];

    let docs = self
      .db
      .collection::<Document>(COLLECTION_NAME)
      .aggregate(
        pipeline.into_iter(),
        AggregateOptions::builder().allow_disk_use(true).build(),
      )
      .await?
      .try_collect::<Vec<Document>>()
      .await?
      .into_iter()
      .map(Bson::from)
      .map(Value::from)
      .collect();

    Ok(docs)
  }
}

impl MongoPersistence {

  /// Extra capabilities outside of the Persistence trait.
  /// Download all users from the mongo collection.
  pub async fn download(
    &self,
  ) -> PersistenceResult<impl Stream<Item = MongoUser>> {
    let mut cursor = self
      .db
      .collection::<MongoUser>(COLLECTION_NAME)
      .find(doc! {}, None)
      .await?;

    Ok(stream! {
      loop {
        match cursor.try_next().await {
          Ok(Some(doc)) => yield doc,
          Ok(None) => break,
          Err(e) => {
            event!(target: PERSISTENCE_TARGET, Level::ERROR,
              "Failed while reading from cursor: {e}");
            break;
          }
        }
      }
    })
  }
}

impl From<UserKey> for mongodb::bson::Bson {
  fn from(user_key: UserKey) -> Self {
    ObjectId::parse_str(&user_key.0)
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
      id: mongo_user._id.map(|oid| oid.to_string()),
      name: mongo_user.name,
      age: mongo_user.age,
      email: Email(mongo_user.email),
      gender: mongo_user.gender,
    }
  }
}

impl From<&User> for MongoUser {
  fn from(user: &User) -> Self {
    MongoUser {
      _id: None,
      name: user.name.clone(),
      age: user.age,
      email: user.email.0.clone(),
      gender: user.gender.clone(),
    }
  }
}
