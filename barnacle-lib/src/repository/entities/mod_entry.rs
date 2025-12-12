use std::fmt::Debug;

use agdb::{DbValue, QueryBuilder};

use crate::repository::{
    db::DbHandle,
    entities::{ElementId, Result},
};

/// Represents a mod entry in the Barnacle system.
///
/// Provides methods to inspect and modify this mod entry's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct ModEntry {
    /// The ID of the ModEntryModel
    pub(crate) entry_id: ElementId,
    /// The ID of the ModModel the entry points to
    pub(crate) mod_id: ElementId,
    pub(crate) db: DbHandle,
}

impl ModEntry {
    pub(crate) fn load(entry_id: ElementId, mod_id: ElementId, db: DbHandle) -> Result<Self> {
        Ok(Self {
            entry_id,
            mod_id,
            db,
        })
    }

    pub fn name(&self) -> Result<String> {
        self.get_mod_field("name")
    }

    pub fn enabled(&self) -> Result<bool> {
        self.get_entry_field("enabled")
    }

    pub fn notes(&self) -> Result<String> {
        self.get_entry_field("notes")
    }

    fn get_mod_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        self.get_field(&self.mod_id, field)
    }

    fn get_entry_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        self.get_field(&self.entry_id, field)
    }

    fn get_field<T>(&self, id: &ElementId, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        let value = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .values(field)
                    .ids(id.db_id(&self.db)?)
                    .query(),
            )?
            .elements
            .pop()
            .expect("successful queries should not be empty")
            .values
            .pop()
            .expect("successful queries should not be empty")
            .value;

        Ok(T::try_from(value).expect("conversion from a `DbValue` must succeed"))
    }

    pub(crate) fn set_field<T>(&mut self, id: &ElementId, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        let element_id = id.db_id(&self.db)?;
        self.db.write().exec_mut(
            QueryBuilder::insert()
                .values([[(field, value).into()]])
                .ids(element_id)
                .query(),
        )?;

        Ok(())
    }
}
