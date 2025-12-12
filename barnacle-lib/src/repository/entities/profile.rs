use std::{fmt::Debug, fs, path::PathBuf};

use agdb::{CountComparison, DbId, DbValue, QueryBuilder};
use heck::ToSnakeCase;
use tracing::debug;

use crate::repository::{
    CoreConfigHandle,
    db::{
        DbHandle,
        models::{GameModel, ModEntryModel, ModModel, ProfileModel},
    },
    entities::{ElementId, Result, game::Game, mod_::Mod, mod_entry::ModEntry},
};

/// Represents a profile entity in the Barnacle system.
///
/// Provides methods to inspect and modify this profile's data, including
/// managing mod entries. Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Profile {
    pub(crate) id: ElementId,
    pub(crate) db: DbHandle,
    pub(crate) cfg: CoreConfigHandle,
}

impl Profile {
    pub(crate) fn load(id: ElementId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
        Ok(Self { id, db, cfg })
    }

    // Fields

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn set_name(&mut self, new_name: &str) -> Result<()> {
        if new_name == self.name()? {
            return Ok(());
        }

        let old_dir = self.dir()?;

        self.set_field("name", new_name)?;

        let new_dir = self.dir()?;
        fs::rename(old_dir, new_dir).unwrap();

        Ok(())
    }

    // Utility

    pub fn dir(&self) -> Result<PathBuf> {
        Ok(self.parent()?.dir()?.join(self.name()?.to_snake_case()))
    }

    pub(crate) fn set_current(db: DbHandle, profile: &Profile) -> Result<()> {
        db.write().transaction_mut(|t| {
            // Delete existing current_profile, if it exists
            t.exec_mut(
                QueryBuilder::remove()
                    .search()
                    .from("current_profile")
                    .where_()
                    .edge()
                    .query(),
            )?;
            // Insert a new edge from current_profile to new profile_id
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from("current_profile")
                    .to(profile.id.db_id(&db)?)
                    .query(),
            )?;

            Ok(())
        })
    }

    pub(crate) fn current(db: DbHandle, cfg: CoreConfigHandle) -> Result<Profile> {
        let db_id = db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ProfileModel>()
                    .search()
                    .from("current_profile")
                    .where_()
                    .neighbor()
                    .query(),
            )?
            .elements
            .first()
            .expect("A successful query should not be empty")
            .id;

        Profile::load(ElementId::load(&db, db_id)?, db.clone(), cfg.clone())
    }

    /// Returns the parent [`Game`] of this [`Profile`]
    pub fn parent(&self) -> Result<Game> {
        let parent_game_id = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("games")
                    .to(self.id.db_id(&self.db)?)
                    .query(),
            )?
            .elements
            .pop()
            .expect("A successful query should not be empty")
            .id;

        let id = ElementId::load(&self.db, parent_game_id)?;
        Game::load(id, self.db.clone(), self.cfg.clone())
    }

    // Operations

    /// Add a new [`ModEntry`] to a [`Profile`] that points to the [`Mod`] given by ID.
    pub fn add_mod_entry(&mut self, mod_: Mod) -> Result<()> {
        let maybe_last_entry_db_id = self
            .mod_entries()?
            .last()
            .map(|e| e.entry_id.db_id(&self.db).unwrap());

        self.db.write().transaction_mut(|t| -> Result<()> {
            let mod_entry = ModEntryModel::default();
            let mod_entry_id = t
                .exec_mut(QueryBuilder::insert().element(&mod_entry).query())?
                .elements
                .first()
                .expect("A successful query should not be empty")
                .id;

            match maybe_last_entry_db_id {
                Some(last_entry_id) => {
                    // Connect last entry in list to new entry
                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from(last_entry_id)
                            .to(mod_entry_id)
                            .query(),
                    )?;
                }
                None => {
                    // Connect profile node to new entry (first entry in the list)
                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from(self.id.db_id(&self.db)?)
                            .to(mod_entry_id)
                            .query(),
                    )?;
                }
            }

            // Connect new entry to target mod
            let mod_db_id = mod_.id.db_id(&self.db)?;
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from(mod_entry_id)
                    .to(mod_db_id)
                    .query(),
            )?;

            // TODO: Return ModEntry
            Ok(())
        })?;

        Ok(())
    }

    pub fn mod_entries(&self) -> Result<Vec<ModEntry>> {
        let mod_entry_ids: Vec<DbId> = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModEntryModel>()
                    .search()
                    .from(self.id.db_id(&self.db)?)
                    .where_()
                    .node()
                    .and()
                    .neighbor()
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| e.id)
            .collect();

        let mod_ids: Vec<DbId> = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModModel>()
                    .search()
                    .from(self.id.db_id(&self.db)?)
                    .where_()
                    .node()
                    .and()
                    // Skip the Profile node and the first ModEntry node
                    .distance(CountComparison::GreaterThan(2))
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
                ModEntry::load(
                    ElementId::load(&self.db, entry_db_id).unwrap(),
                    ElementId::load(&self.db, mod_db_id).unwrap(),
                    self.db.clone(),
                )
                .unwrap()
            })
            .collect())
    }

    pub(crate) fn remove(self) -> Result<()> {
        let name = self.name()?;
        let dir = self.dir()?;

        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(self.id.db_id(&self.db)?).query())?;

        fs::remove_dir_all(dir).unwrap();

        debug!("Removed profile: {name}");

        Ok(())
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

#[cfg(test)]
mod test {
    use crate::{Repository, repository::DeployKind};

    #[test]
    fn test_add() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        game.add_profile("Test").unwrap();
    }
}
