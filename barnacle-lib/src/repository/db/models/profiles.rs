use agdb::{DbElement, DbId};

use crate::repository::entities::Uid;

#[derive(Debug, Clone, DbElement, PartialEq, PartialOrd)]
pub(crate) struct ProfileModel {
    db_id: Option<DbId>,
    uid: u64,
    name: String,
}

impl ProfileModel {
    pub fn new(uid: Uid, name: &str) -> Self {
        Self {
            db_id: None,
            uid: uid.0,
            name: name.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
