use std::{fmt::Debug, path::PathBuf};

use agdb::DbValue;

use crate::repository::{
    config::CoreConfigHandle,
    db::DbHandle,
    entities::{EntityId, Result, get_field, set_field},
};

/// Represents a tool entity in the Barnacle system.
///
/// Provides methods to inspect and modify this tool's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Tool {
    id: EntityId,
    db: DbHandle,
    cfg: CoreConfigHandle,
}

impl Tool {
    pub(crate) fn from_id(id: EntityId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
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
        get_field(&self.db, self.id, field)
    }

    pub(crate) fn set_field<T>(&mut self, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        set_field(&mut self.db, self.id, field, value)
    }
}
