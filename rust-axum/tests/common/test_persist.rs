//! A mocked User persistence test api.
use mongodb::bson::oid::ObjectId;
use serde_json::{json, Value};
use std::{collections::HashMap, ops::Deref, sync::Arc, sync::RwLock};
use user_persist::persistence::PersistenceResult;
use user_persist::{
    persistence::{PersistenceError, UserPersistence},
    types::{Email, Gender, UpdateUser, User, UserKey, UserSearch},
};

/// Create a test user.
pub fn test_user(id: Option<UserKey>) -> User {
    User {
        id,
        name: String::from("Test User"),
        email: Email(String::from("test@test.com")),
        age: 100,
        gender: Gender::Male,
    }
}

#[derive(Debug, Clone)]
pub struct TestPersistence(Arc<RwLock<HashMap<UserKey, User>>>);

impl Deref for TestPersistence {
    type Target = Arc<RwLock<HashMap<UserKey, User>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TestPersistence {
    pub fn new() -> Self {
        // Setup some test data.
        let mut map = HashMap::new();
        let key = "61c0d1954c6b974ca7000000".parse::<UserKey>().unwrap();
        map.insert(key.clone(), test_user(Some(key)));
        Self(Arc::new(RwLock::new(map)))
    }
}

impl Default for TestPersistence {
    fn default() -> Self {
        TestPersistence::new()
    }
}

// A test implementation of the UserPersistence layer.
impl UserPersistence for TestPersistence {
    async fn get_user(&self, id: &UserKey) -> Result<Option<User>, PersistenceError> {
        let guard = self.read().unwrap();
        let user = guard.get(id).map(|u| u.to_owned());
        Ok(user)
    }

    async fn save_user(&self, user: &User) -> Result<User, PersistenceError> {
        let mut updated_user = user.clone();
        let user_key = UserKey(ObjectId::new().to_string());
        updated_user.id = Some(user_key.clone());
        self.write().unwrap().insert(user_key, updated_user.clone());
        Ok(updated_user)
    }

    async fn update_user(&self, user: &UpdateUser) -> Result<(), PersistenceError> {
        let mut guard = self.write().unwrap();
        if let Some(old_user) = guard.get_mut(&user.id) {
            old_user.name.clone_from(&user.name);
            old_user.age = user.age;
        };
        Ok(())
    }

    async fn remove_user(&self, user: &UserKey) -> PersistenceResult<()> {
        let mut m = self.write().unwrap();
        m.remove(user);
        Ok(())
    }

    async fn search_users(&self, _user_search: &UserSearch) -> Result<Vec<User>, PersistenceError> {
        Ok(vec![test_user(Some(
            "61c0d1954c6b974ca7000000".parse().unwrap(),
        ))])
    }

    async fn count_genders(&self) -> Result<Vec<Value>, PersistenceError> {
        Ok(vec![
            json!({
                "_id": "Male",
                "count": 6
            }),
            json!({
                "_id": "Female",
                "count": 12
            }),
        ])
    }
}
