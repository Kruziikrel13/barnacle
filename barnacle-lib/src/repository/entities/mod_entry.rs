use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use agdb::DbId;

use crate::repository::{
    db::DbHandle,
    entities::{Error, Result, get_field},
};

/// Represents a mod entry in the Barnacle system.
///
/// Provides methods to inspect and modify this mod entry's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct ModEntry {
    /// The ID of the ModEntryModel
    pub(crate) entry_id: DbId,
    /// The ID of the ModModel the entry points to
    pub(crate) mod_id: DbId,
    valid: Arc<AtomicBool>,
    pub(crate) db: DbHandle,
}

impl ModEntry {
    pub(crate) fn from_id(entry_id: DbId, mod_id: DbId, db: DbHandle) -> Self {
        Self {
            entry_id,
            mod_id,
            valid: Arc::new(AtomicBool::new(true)),
            db,
        }
    }

    pub fn name(&self) -> Result<String> {
        self.is_valid()?;

        get_field(&self.db, self.mod_id, "name")
    }

    pub fn enabled(&self) -> Result<bool> {
        self.is_valid()?;

        get_field(&self.db, self.entry_id, "enabled")
    }

    pub fn notes(&self) -> Result<String> {
        self.is_valid()?;

        get_field(&self.db, self.entry_id, "notes")
    }

    /// Ensure that the entity is pointing to an existent model in the database
    fn is_valid(&self) -> Result<()> {
        if self.valid.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err(Error::StaleEntity)
        }
    }
}
