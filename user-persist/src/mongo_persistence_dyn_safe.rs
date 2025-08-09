/*!
This module provides data access to a a mongodb user collection.

This implements the trait via async_trait. This is needed for actix-web and rocket that don't
support generics in the routes.
*/
use crate::{
    mongo_persistence::{MongoPersistence, MongoUser},
    persistence::{PersistenceResult, UserPersistenceDynSafe},
    types::{UpdateUser, User, UserKey, UserSearch},
    PERSISTENCE_TARGET,
};
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Bson, Document},
    results::InsertOneResult,
};
use serde_json::Value;
use tracing::{debug, instrument};

const COLLECTION_NAME: &str = "users";

#[async_trait]
impl UserPersistenceDynSafe for MongoPersistence {
    async fn get_user(&self, id: &UserKey) -> PersistenceResult<Option<User>> {
        let user = self
            .user_collection()
            .find_one(doc! {"_id": ObjectId::try_from(id)?})
            .await?
            .map(User::from);

        Ok(user)
    }

    async fn save_user(&self, user: &User) -> PersistenceResult<User> {
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

    async fn update_user(&self, user: &UpdateUser) -> PersistenceResult<()> {
        let query = doc! {"_id": ObjectId::try_from(&user.id)?};
        let update_fields = doc! {"name": &user.name, "age": &user.age, "email": &user.email};
        let update = doc! {"$set": update_fields};

        let updated = self.user_collection().update_one(query, update).await?;

        debug!(target: PERSISTENCE_TARGET, "update result: {updated:?}",);

        Ok(())
    }

    async fn remove_user(&self, key: &UserKey) -> PersistenceResult<()> {
        let result = self
            .user_collection()
            .delete_one(doc! {
              "_id": ObjectId::try_from(key)?
            })
            .await?;
        debug!(target: PERSISTENCE_TARGET, "delete result: {result:?}");
        Ok(())
    }

    #[instrument(
        skip_all,
        level = "debug",
        target = "persistence",
        name = "search-span"
    )]
    async fn search_users(&self, user_search: &UserSearch) -> PersistenceResult<Vec<User>> {
        let search = doc! { "email": &user_search.email, "gender": &user_search.gender,
            "name": &user_search.name
        };

        let filtered_null = search
            .into_iter()
            .filter(|(_, value)| value != &Bson::Null)
            .collect::<Document>();

        debug!(
          target: PERSISTENCE_TARGET,
          "mongo search query: {filtered_null}",
        );

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

    async fn count_genders(&self) -> PersistenceResult<Vec<Value>> {
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
}
