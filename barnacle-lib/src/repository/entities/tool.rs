use std::{
    fmt::{self, Debug, Display, Formatter},
    path::PathBuf,
};

use agdb::DbValue;

use crate::repository::{
    config::Cfg,
    db::Db,
    entities::{EntityId, Result, get_field, set_field},
};

/// Represents a tool entity in the Barnacle system.
///
/// Provides methods to inspect and modify this tool's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Tool {
    id: EntityId,
    db: Db,
    cfg: Cfg,
}

impl Tool {
    pub(crate) fn load(id: EntityId, db: Db, cfg: Cfg) -> Result<Self> {
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
        set_field(&self.db, self.id, field, value)
    }
}

impl Display for Tool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name().unwrap_or_else(|_| "<invalid game name>".into())
        )
    }
}

impl PartialEq for Tool {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
