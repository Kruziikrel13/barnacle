use agdb::{DbId, DbType};

use crate::repository::db::Uid;

#[derive(Debug, Clone, DbType, PartialEq, PartialOrd)]
pub(crate) struct ModEntryModel {
    db_id: Option<DbId>,
    uid: Uid,
    enabled: bool,
    notes: String,
}

impl ModEntryModel {
    pub fn new(uid: Uid) -> Self {
        Self {
            db_id: None,
            uid,
            enabled: true,
            notes: "".into(),
        }
    }
}
