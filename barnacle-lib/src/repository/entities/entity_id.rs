use agdb::DbId;
use derive_more::PartialEq;

use crate::repository::{
    entities::{Error, Result},
    {db::Db, entities::Uid},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityId {
    #[partial_eq(ignore)]
    db_id: DbId,
    /// A unique idenifier that specifies a particular entity
    uid: Uid,
}

impl EntityId {
    /// Load an [`ElementId`] from an existing element.
    pub fn load(db: &Db, db_id: DbId) -> Result<Self> {
        Ok(Self {
            db_id,
            uid: Uid::load(db, db_id)?,
        })
    }

    /// Get the underlying [`DbId`]. This will check to make sure it isn't stale before returning.
    pub fn db_id(&self, db: &Db) -> Result<DbId> {
        let uid = Uid::load(db, self.db_id).map_err(|err| {
            match err {
                Error::Internal(e) => {
                    // TODO: Match on DbError kind once the following is completed:
                    // https://github.com/agnesoft/agdb/issues/1687
                    let not_found = format!("Id '{}' not found", self.db_id.as_index());
                    if e.description == not_found {
                        Error::RemovedEntity
                    } else {
                        Error::Internal(e)
                    }
                }
                other => other,
            }
        })?;

        // If the UID changed, that means this DbId now refers to a different or deleted entity
        if uid != self.uid {
            return Err(Error::RemovedEntity);
        }

        Ok(self.db_id)
    }
}
