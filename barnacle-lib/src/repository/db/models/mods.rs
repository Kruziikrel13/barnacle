use agdb::{DbElement, DbId};

use crate::repository::entities::Uid;

#[derive(Debug, Clone, DbElement, PartialEq, PartialOrd)]
pub(crate) struct ModModel {
    db_id: Option<DbId>,
    uid: u64,
    /// A human friendly display name
    name: String,
}

impl ModModel {
    pub fn new(uid: Uid, name: &str) -> Self {
        Self {
            db_id: None,
            uid: uid.0,
            name: name.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
