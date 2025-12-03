use agdb::{DbId, DbType};

use crate::repository::db::Uid;

#[derive(Debug, Clone, DbType, Default, PartialEq, PartialOrd)]
pub(crate) struct ModEntryModel {
    db_id: Option<DbId>,
    uid: Uid,
    enabled: bool,
    notes: String,
}
