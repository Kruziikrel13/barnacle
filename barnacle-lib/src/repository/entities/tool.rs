use std::{fmt::Debug, path::PathBuf};

use agdb::{DbId, DbValue, QueryBuilder};

use crate::repository::{
    config::CoreConfigHandle,
    db::{DbHandle, Uid},
    entities::{Error, Result, uid},
};

/// Represents a tool entity in the Barnacle system.
///
/// Provides methods to inspect and modify this tool's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Tool {
    db_id: DbId,
    uid: Uid,
    db: DbHandle,
    cfg: CoreConfigHandle,
}

impl Tool {
    pub(crate) fn from_id(db_id: DbId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
        Ok(Self {
            db_id,
            uid: uid(&db, db_id)?,
            db,
            cfg,
        })
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
        let mut values = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .values([[field, "uid"]])
                    .ids(self.db_id)
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

        if uid != self.uid {
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
