use std::{
    fmt::{self, Debug, Display, Formatter},
    fs,
    path::PathBuf,
};

use agdb::{DbId, DbValue, QueryBuilder};
use heck::ToSnakeCase;
use tracing::debug;

use crate::repository::{
    CoreConfigHandle,
    db::{
        DbHandle,
        models::{GameModel, ModEntryModel, ModModel, ProfileModel},
    },
    entities::{
        EntityId, Result, game::Game, get_field, mod_::Mod, mod_entry::ModEntry, next_uid,
        set_field,
    },
};

/// Represents a profile entity in the Barnacle system.
///
/// Provides methods to inspect and modify this profile's data, including
/// managing mod entries. Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Profile {
    pub(crate) id: EntityId,
    pub(crate) db: DbHandle,
    pub(crate) cfg: CoreConfigHandle,
}

impl Profile {
    pub(crate) fn load(db_id: DbId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
        let id = EntityId::load(&db, db_id)?;
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
        Ok(self
            .parent()?
            .dir()?
            .join("profiles")
            .join(self.name()?.to_snake_case()))
    }

    pub(crate) fn set_current(db: DbHandle, profile: &Profile) -> Result<()> {
        let db_id = profile.id.db_id(&db)?;
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
                    .to(db_id)
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

        Profile::load(db_id, db.clone(), cfg.clone())
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

        Game::load(parent_game_id, self.db.clone(), self.cfg.clone())
    }

    // Operations

    /// Add a new [`ModEntry`] to a [`Profile`] that points to the [`Mod`] given by ID.
    pub fn add_mod_entry(&self, mod_: Mod) -> Result<ModEntry> {
        ModEntry::add(&self.db, self, mod_)
    }

    pub fn mod_entries(&self) -> Result<Vec<ModEntry>> {
        ModEntry::list(&self.db, self)
    }

    pub(crate) fn remove(self) -> Result<()> {
        let name = self.name()?;
        let dir = self.dir()?;

        let db_id = self.id.db_id(&self.db)?;
        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(db_id).query())?;

        fs::remove_dir_all(dir).unwrap();

        debug!("Removed profile: {name}");

        Ok(())
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

impl Display for Profile {
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

        assert!(profile.dir().unwrap().exists());
    }

    #[test]
    fn test_remove() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let profile = game.add_profile("Test").unwrap();

        assert_eq!(game.profiles().unwrap().len(), 1);

        let dir = profile.dir().unwrap();

        game.remove_profile(profile).unwrap();

        assert!(!dir.exists());
        assert_eq!(game.profiles().unwrap().len(), 0);
    }

    #[test]
    fn test_set_current() {
        let mut repo = Repository::mock();

        let mut game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();

        repo.set_current_profile(&profile).unwrap();
        repo.current_profile().unwrap();
    }
}
