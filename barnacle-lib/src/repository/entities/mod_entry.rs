use std::fmt::{self, Debug, Display, Formatter};

use agdb::{DbId, DbValue, QueryBuilder, QueryId};

use crate::repository::{
    Mod, Profile,
    db::{
        Db,
        models::{ModEntryModel, ModModel},
    },
    entities::{EntityId, Result, Uid, get_field, set_field},
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
    pub(crate) db: Db,
}

impl ModEntry {
    pub(crate) fn load(entry_db_id: DbId, mod_db_id: DbId, db: Db) -> Result<Self> {
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

    pub fn set_enabled(&self, value: bool) -> Result<()> {
        self.set_entry_field("enabled", value)
    }

    pub fn notes(&self) -> Result<String> {
        self.get_entry_field("notes")
    }

    pub(crate) fn add(db: &Db, profile: &Profile, mod_: Mod) -> Result<Self> {
        let profile_id = profile.id.db_id(db)?;
        let mod_id = mod_.id.db_id(db)?;

        let maybe_last_entry_id = profile
            .mod_entries()?
            .last()
            .map(|e| e.entry_id.db_id(db).unwrap());

        let model = ModEntryModel::new(Uid::new(db)?);
        let entry_id = db.write().transaction_mut(|t| -> Result<DbId> {
            let entry_id = t
                .exec_mut(QueryBuilder::insert().element(&model).query())?
                .elements
                .first()
                .expect("A successful query should not be empty")
                .id;

            match maybe_last_entry_id {
                Some(last_entry_id) => {
                    // Connect last entry in list to new entry
                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from([QueryId::from("mod_entries"), QueryId::from(last_entry_id)])
                            .to(entry_id)
                            .query(),
                    )?;
                }
                // First entry
                None => {
                    // Connect profile node to new entry
                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from([QueryId::from("mod_entries"), QueryId::from(profile_id)])
                            .to(entry_id)
                            .query(),
                    )?;
                }
            }

            // Connect new entry to target mod
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from(entry_id)
                    .to(mod_id)
                    .query(),
            )?;

            Ok(entry_id)
        })?;

        ModEntry::load(entry_id, mod_id, db.clone())
    }

    /// Remove the given [`ModEntry`] from the list
    pub(crate) fn remove(self, profile: &Profile) -> Result<()> {
        let id = self.entry_id.db_id(&self.db)?;
        let profile_id = profile.id.db_id(&self.db)?;
        let entry_ids: Vec<DbId> = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModEntryModel>()
                    .search()
                    .from(profile_id)
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| e.id)
            .collect();

        let mut iter = entry_ids.into_iter().peekable();
        let mut prev = None;
        while let Some(curr) = iter.next() {
            if curr == id {
                let next = iter.peek().copied();

                match (prev, next) {
                    // First element
                    (None, Some(next)) => self.db.write().transaction_mut(|t| -> Result<()> {
                        t.exec_mut(QueryBuilder::remove().ids(curr).query())?;

                        // Connect profile to new first element
                        t.exec_mut(
                            QueryBuilder::insert()
                                .edges()
                                .from(profile_id)
                                .to(next)
                                .query(),
                        )?;

                        Ok(())
                    })?,
                    // Middle element
                    (Some(prev), Some(next)) => {
                        self.db.write().transaction_mut(|t| -> Result<()> {
                            t.exec_mut(QueryBuilder::remove().ids(curr).query())?;

                            // Connect previous element to next element
                            t.exec_mut(QueryBuilder::insert().edges().from(prev).to(next).query())?;

                            Ok(())
                        })?
                    }
                    // Last or only element
                    (Some(_), None) | (None, None) => {
                        self.db
                            .write()
                            .exec_mut(QueryBuilder::remove().ids(curr).query())?;
                    }
                }

                break;
            }

            prev = Some(curr);
        }

        Ok(())
    }

    pub(crate) fn list(db: &Db, profile: &Profile) -> Result<Vec<Self>> {
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

    fn set_entry_field<T>(&self, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        self.set_field(self.entry_id, field, value)
    }

    fn set_mod_field<T>(&self, field: &str, value: T) -> Result<()>
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

    pub(crate) fn set_field<T>(&self, id: EntityId, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        set_field(&self.db, id, field, value)
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

impl PartialEq for ModEntry {
    fn eq(&self, other: &Self) -> bool {
        self.entry_id == other.entry_id && self.mod_id == other.mod_id
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Repository, repository::DeployKind};

    #[test]
    fn test_add() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();

        let mod1 = game.add_mod("Super Duper Mod", None).unwrap();
        let mod2 = game.add_mod("Super Duper Mod: 2", None).unwrap();

        profile.add_mod_entry(mod1).unwrap();
        profile.add_mod_entry(mod2).unwrap();

        assert_eq!(profile.mod_entries().unwrap().len(), 2);
    }

    #[test]
    fn test_remove() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();

        let mod_entries: Vec<_> = (1..=6)
            .map(|i| {
                let m = game.add_mod(&format!("Mod{i}"), None).unwrap();
                profile.add_mod_entry(m).unwrap()
            })
            .collect();

        assert_eq!(profile.mod_entries().unwrap().len(), 6);

        let remove_and_check = |entry: &ModEntry| {
            profile.remove_mod_entry(entry.clone()).unwrap();
            let entries = profile.mod_entries().unwrap();
            assert!(!entries.contains(entry));
        };

        remove_and_check(mod_entries.first().unwrap()); // first
        remove_and_check(mod_entries.get(3).unwrap()); // middle
        remove_and_check(mod_entries.get(5).unwrap()); // last

        // Check remaining entries are exactly the ones we expect
        let remaining: Vec<&ModEntry> = mod_entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match i {
                // Filter out the entries we removed
                0 | 3 | 5 => None,
                // These are the ones we expect to be here
                _ => Some(e),
            })
            .collect();
        assert_eq!(
            profile.mod_entries().unwrap().iter().collect::<Vec<_>>(),
            remaining
        );
    }

    #[test]
    fn test_name() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        profile.add_mod_entry(mod_).unwrap().name().unwrap();
    }

    #[test]
    fn test_enabled() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();
        let mod_ = game.add_mod("Super Duper Mod", None).unwrap();

        let entry = profile.add_mod_entry(mod_).unwrap();

        assert!(entry.enabled().unwrap());

        entry.set_enabled(false).unwrap();

        assert!(!entry.enabled().unwrap());
    }
}
