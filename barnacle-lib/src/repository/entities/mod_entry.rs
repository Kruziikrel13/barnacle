use std::fmt::{self, Debug, Display, Formatter};

use agdb::{DbId, DbValue, QueryBuilder};

use crate::repository::{
    Mod, Profile,
    db::{
        DbHandle,
        models::{ModEntryModel, ModModel},
    },
    entities::{EntityId, Result, get_field, next_uid, set_field},
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

    pub fn set_enabled(&mut self, value: bool) -> Result<()> {
        self.set_entry_field("enabled", value)
    }

    pub fn notes(&self) -> Result<String> {
        self.get_entry_field("notes")
    }

    pub(crate) fn add(db: &DbHandle, profile: &Profile, mod_: Mod) -> Result<Self> {
        let profile_db_id = profile.id.db_id(db)?;
        let mod_db_id = mod_.id.db_id(db)?;

        let maybe_last_entry_db_id = profile
            .mod_entries()?
            .last()
            .map(|e| e.entry_id.db_id(db).unwrap());

        let model = ModEntryModel::new(next_uid(db)?);
        let entry_db_id = db.write().transaction_mut(|t| -> Result<DbId> {
            let entry_db_id = t
                .exec_mut(QueryBuilder::insert().element(&model).query())?
                .elements
                .first()
                .expect("A successful query should not be empty")
                .id;

            match maybe_last_entry_db_id {
                Some(last_entry_db_id) => {
                    // Connect last entry in list to new entry
                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from(last_entry_db_id)
                            .to(entry_db_id)
                            .query(),
                    )?;
                }
                None => {
                    // Connect profile node to new entry (first entry in the list)
                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from(profile_db_id)
                            .to(entry_db_id)
                            .query(),
                    )?;
                }
            }

            // Connect new entry to target mod
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from(entry_db_id)
                    .to(mod_db_id)
                    .query(),
            )?;

            Ok(entry_db_id)
        })?;

        ModEntry::load(entry_db_id, mod_db_id, db.clone())
    }

    pub(crate) fn list(db: &DbHandle, profile: &Profile) -> Result<Vec<Self>> {
        let db_id = profile.id.db_id(db)?;
        let mod_entry_ids: Vec<DbId> = db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModEntryModel>()
                    .search()
                    .from(db_id)
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| e.id)
            .collect();

        let mod_ids: Vec<DbId> = db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModModel>()
                    .search()
                    .from(db_id)
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| e.id)
            .collect();

        Ok(mod_entry_ids
            .into_iter()
            .zip(mod_ids)
            .map(|(entry_db_id, mod_db_id)| {
                ModEntry::load(entry_db_id, mod_db_id, db.clone()).unwrap()
            })
            .collect())
    }

    fn get_entry_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        self.get_field(self.entry_id, field)
    }

    fn get_mod_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        self.get_field(self.mod_id, field)
    }

    fn set_entry_field<T>(&mut self, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        self.set_field(self.entry_id, field, value)
    }

    fn set_mod_field<T>(&mut self, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        self.set_field(self.mod_id, field, value)
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

impl Display for ModEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name().unwrap_or_else(|_| "<invalid game name>".into())
        )
    }
}

#[cfg(test)]
mod test {
    use crate::{Repository, repository::DeployKind};

    #[test]
    fn test_add() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();

        let mod1 = game.add_mod("Super Duper Mod", None).unwrap();
        let mod2 = game.add_mod("Super Duper Mod: 2", None).unwrap();

        profile.add_mod_entry(mod1).unwrap();
        profile.add_mod_entry(mod2).unwrap();

        assert_eq!(profile.mod_entries().unwrap().len(), 2);
    }

    #[test]
    fn test_name() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        profile.add_mod_entry(mod_).unwrap().name().unwrap();
    }

    #[test]
    fn test_enabled() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        let mut entry = profile.add_mod_entry(mod_).unwrap();

        assert!(entry.enabled().unwrap());

        entry.set_enabled(false).unwrap();

        assert!(!entry.enabled().unwrap());
    }
}
