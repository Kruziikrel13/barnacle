use std::fmt::Debug;

use agdb::{DbId, DbValue, QueryBuilder};

use crate::repository::{
    db::{DbHandle, Uid},
    entities::{Error, Result, uid},
};

/// Represents a mod entry in the Barnacle system.
///
/// Provides methods to inspect and modify this mod entry's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct ModEntry {
    /// The ID of the ModEntryModel
    pub(crate) entry_db_id: DbId,
    pub(crate) entry_uid: Uid,
    /// The ID of the ModModel the entry points to
    pub(crate) mod_db_id: DbId,
    pub(crate) db: DbHandle,
}

impl ModEntry {
    pub(crate) fn from_id(entry_db_id: DbId, mod_db_id: DbId, db: DbHandle) -> Result<Self> {
        Ok(Self {
            entry_db_id,
            mod_db_id,
            entry_uid: uid(&db, entry_db_id)?,
            db,
        })
    }

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn enabled(&self) -> Result<bool> {
        self.get_field("enabled")
    }

    pub fn notes(&self) -> Result<String> {
        self.get_field("notes")
    }

    fn get_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        let mut values = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .values([[field, "uid"]])
                    .ids(self.entry_db_id)
                    .query(),
            )?
            .elements
            .pop()
            .expect("successful queries should not be empty")
            .values;

        let uid = values
            .pop()
            .expect("successful queries should not be empty")
            .value
            .to_u64()?;

        if uid != self.entry_uid {
            return Err(Error::StaleEntity);
        }

        let value = values
            .pop()
            .expect("successful queries should not be empty")
            .value;

        Ok(T::try_from(value)
        .expect("Conversion from a `DbValue` must succeed. Perhaps the wrong type was expected from this field."))
    }
}
