use agdb::{DbElement, DbId};

use crate::repository::entities::Uid;

#[derive(Debug, Clone, DbElement, PartialEq, PartialOrd)]
pub(crate) struct ModEntryModel {
    db_id: Option<DbId>,
    uid: u64,
    enabled: bool,
    notes: String,
}

impl ModEntryModel {
    pub fn new(uid: Uid) -> Self {
        Self {
            db_id: None,
            uid: uid.0,
            enabled: true,
            notes: "".into(),
        }
    }
}
