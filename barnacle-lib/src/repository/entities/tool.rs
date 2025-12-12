use std::{fmt::Debug, path::PathBuf};

use agdb::{DbValue, QueryBuilder};

use crate::repository::{
    config::CoreConfigHandle,
    db::DbHandle,
    entities::{ElementId, Result},
};

/// Represents a tool entity in the Barnacle system.
///
/// Provides methods to inspect and modify this tool's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Tool {
    id: ElementId,
    db: DbHandle,
    cfg: CoreConfigHandle,
}

impl Tool {
    pub(crate) fn from_id(id: ElementId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
        Ok(Self { id, db, cfg })
    }

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn path(&self) -> Result<PathBuf> {
        self.get_field("path")
    }

    // TODO: This can actually be Option<String>
    pub fn args(&self) -> Result<String> {
        self.get_field("args")
    }

    fn get_field<T>(&self, field: &str) -> Result<T>
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
                    .ids(self.id.db_id(&self.db)?)
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

    pub(crate) fn set_field<T>(&mut self, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        let element_id = self.id.db_id(&self.db)?;
        self.db.write().exec_mut(
            QueryBuilder::insert()
                .values([[(field, value).into()]])
                .ids(element_id)
                .query(),
        )?;

        Ok(())
    }
}
