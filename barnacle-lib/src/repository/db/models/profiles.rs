use agdb::{DbElement, DbId};

use crate::repository::db::Uid;

#[derive(Debug, Clone, DbElement, PartialEq, PartialOrd)]
pub(crate) struct ProfileModel {
    pub(crate) db_id: Option<DbId>,
    pub(crate) uid: Uid,
    pub(crate) name: String,
}

impl ProfileModel {
    pub fn new(uid: Uid, name: &str) -> Self {
        Self {
            db_id: None,
            uid,
            name: name.to_string(),
        }
    }
}
