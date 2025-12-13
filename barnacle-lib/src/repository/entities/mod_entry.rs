use std::fmt::Debug;

use agdb::{DbId, DbValue};

use crate::repository::{
    db::DbHandle,
    entities::{EntityId, Result, get_field, set_field},
};

/// Represents a mod entry in the Barnacle system.
///
/// Provides methods to inspect and modify this mod entry's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct ModEntry {
    /// The ID of the ModEntryModel
    pub(crate) entry_id: EntityId,
    /// The ID of the ModModel the entry points to
    pub(crate) mod_id: EntityId,
    pub(crate) db: DbHandle,
}

impl ModEntry {
    pub(crate) fn load(entry_db_id: DbId, mod_db_id: DbId, db: DbHandle) -> Result<Self> {
        Ok(Self {
            entry_id: EntityId::load(&db, entry_db_id)?,
            mod_id: EntityId::load(&db, mod_db_id)?,
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
        self.get_field(self.mod_id, field)
    }

    fn get_entry_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        self.get_field(self.entry_id, field)
    }

    fn get_field<T>(&self, id: EntityId, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        get_field(&self.db, id, field)
    }

    pub(crate) fn set_field<T>(&mut self, id: EntityId, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        set_field(&mut self.db, id, field, value)
    }
}

#[cfg(test)]
mod test {
    use crate::{Repository, repository::DeployKind};

    #[test]
    fn test_add() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let mut profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        profile.add_mod_entry(mod_).unwrap();

        assert_eq!(profile.mod_entries().unwrap().len(), 1);
    }

    #[test]
    fn test_name() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let mut profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        profile.add_mod_entry(mod_).unwrap().name().unwrap();
    }

    #[test]
    fn test_enabled() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let mut profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        profile.add_mod_entry(mod_).unwrap().enabled().unwrap();
    }
}
