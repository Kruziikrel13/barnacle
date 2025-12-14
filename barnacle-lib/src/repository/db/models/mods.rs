use agdb::{DbElement, DbId};

use crate::repository::db::Uid;

#[derive(Debug, Clone, DbElement, PartialEq, PartialOrd)]
pub(crate) struct ModModel {
    pub(crate) db_id: Option<DbId>,
    pub(crate) uid: Uid,
    /// A human friendly display name
    pub(crate) name: String,
}

impl ModModel {
    pub fn new(uid: Uid, name: &str) -> Self {
        Self {
            db_id: None,
            uid,
            name: name.into(),
        }
    }
}
