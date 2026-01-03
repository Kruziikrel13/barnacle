use std::{
    fmt::{self, Debug, Display, Formatter},
    fs,
    path::PathBuf,
};

use super::Error;
use agdb::{CountComparison, DbId, DbValue, QueryBuilder};
use heck::ToSnakeCase;
use tracing::debug;

use crate::repository::{
    Cfg,
    db::{
        Db,
        models::{GameModel, ProfileModel},
    },
    entities::{
        EntityId, Result, game::Game, get_field, mod_::Mod, mod_entry::ModEntry, set_field,
    },
};

/// Represents a profile entity in the Barnacle system.
///
/// Provides methods to inspect and modify this profile's data, including
/// managing mod entries. Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Profile {
    pub(crate) id: EntityId,
    pub(crate) db: Db,
    pub(crate) cfg: Cfg,
}

impl Profile {
    pub(crate) fn load(db_id: DbId, db: Db, cfg: Cfg) -> Result<Self> {
        let id = EntityId::load(&db, db_id)?;
        Ok(Self { id, db, cfg })
    }

    // Fields

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn set_name(&self, new_name: &str) -> Result<()> {
        if new_name == self.name()? {
            return Ok(());
        }

        let old_dir = self.dir()?;

        self.set_field("name", new_name)?;

        let new_dir = self.dir()?;
        fs::rename(old_dir, new_dir).unwrap();

        Ok(())
    }

    pub fn dir(&self) -> Result<PathBuf> {
        Ok(self
            .parent()?
            .dir()?
            .join("profiles")
            .join(self.name()?.to_snake_case()))
    }

    /// Make this profile the active one
    pub fn make_active(&self) -> Result<()> {
        let active_game = Game::active(self.db.clone(), self.cfg.clone())?;
        if Some(self.parent()?) != active_game {
            return Err(Error::ParentGameMismatch);
        }

        let db_id = self.id.db_id(&self.db)?;
        self.db.write().transaction_mut(|t| {
            // Delete existing active_profile, if it exists
            t.exec_mut(
                QueryBuilder::remove()
                    .search()
                    .from("active_profile")
                    .where_()
                    .edge()
                    .and()
                    // Only delete the first edge. We don't want to accidentally wipe out all edges
                    // coming from active_profile
                    .distance(CountComparison::Equal(1))
                    .query(),
            )?;
            // Insert a new edge from active_profile to new profile_id
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from("active_profile")
                    .to(db_id)
                    .query(),
            )?;

            Ok(())
        })
    }

    pub fn is_active(&self) -> Result<bool> {
        Ok(Profile::active(self.db.clone(), self.cfg.clone())? == Some(self.clone()))
    }

    pub(crate) fn active(db: Db, cfg: Cfg) -> Result<Option<Profile>> {
        let query = db.read().exec(
            QueryBuilder::select()
                .elements::<ProfileModel>()
                .search()
                .from("active_profile")
                .where_()
                .neighbor()
                .query(),
        )?;

        if let Some(db_id) = query.elements.first().map(|p| p.id) {
            Profile::load(db_id, db.clone(), cfg.clone()).map(Some)
        } else {
            Ok(None)
        }
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
        ModEntry::add(&self.db, &self.cfg, self, mod_)
    }

    pub fn mod_entries(&self) -> Result<Vec<ModEntry>> {
        ModEntry::list(&self.db, &self.cfg, self)
    }

    pub fn remove(self) -> Result<()> {
        for entry in self.mod_entries()? {
            let entry_id = entry.entry_id;
            entry
                .remove()
                .or_else(|err| match err {
                    Error::RemovedEntity => Ok(()), // if id is stale assume already removed
                    other => Err(other),
                })
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed to remove mod entry: {:?}: {} during profile cleanup",
                        entry_id, err
                    )
                })
        }

        if self.is_active()?
            && let Some(profile) = self.parent()?.profiles()?.first()
        {
            profile.make_active()?;
        }

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

    pub(crate) fn list(db: &Db, cfg: &Cfg, game: &Game) -> Result<Vec<Self>> {
        let db_id = game.id.db_id(db)?;
        Ok(db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ProfileModel>()
                    .search()
                    .from(db_id)
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| Profile::load(e.id, db.clone(), cfg.clone()).unwrap())
            .collect())
    }

    fn get_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        get_field(&self.db, self.id, field)
    }

    pub(crate) fn set_field<T>(&self, field: &str, value: T) -> Result<()>
    where
        T: Into<DbValue>,
    {
        set_field(&self.db, self.id, field, value)
    }
}

impl PartialEq for Profile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod test {
    use crate::{
        Repository,
        repository::{DeployKind, entities::Error},
    };

    #[test]
    fn test_add() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();

        assert!(profile.dir().unwrap().exists());
    }

    #[test]
    fn test_remove() {
        let repo = Repository::mock();
        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let _mod = game.add_mod("test_mod", None).unwrap();

        let profile = game.add_profile("Test").unwrap();
        let mod_entry = profile.add_mod_entry(_mod).unwrap();

        assert_eq!(game.profiles().unwrap().len(), 1);

        let dir = profile.dir().unwrap();

        profile.remove().unwrap();

        assert!(matches!(mod_entry.remove(), Err(Error::RemovedEntity)));
        assert!(!dir.exists());
        assert_eq!(game.profiles().unwrap().len(), 0);
    }

    #[test]
    fn test_remove_made_next_profile_active() {
        let repo = Repository::mock();
        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let profile1 = game.add_profile("Test").unwrap();
        let profile2 = game.add_profile("Test2").unwrap();

        profile1.make_active().unwrap();
        assert!(profile1.is_active().unwrap());

        profile1.remove().unwrap();
        assert!(profile2.is_active().unwrap());
    }

    #[test]
    fn test_list() {
        let repo = Repository::mock();
        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();

        assert_eq!(game.profiles().unwrap().len(), 0);

        game.add_profile("Cool Profile").unwrap();

        assert_eq!(repo.games().unwrap().len(), 1);
    }

    #[test]
    fn test_parent() {
        let repo = Repository::mock();

        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let profile = game.add_profile("Test").unwrap();

        assert_eq!(profile.parent().unwrap(), game);
    }

    #[test]
    fn test_make_active() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let profile = game.add_profile("Test").unwrap();

        game.make_active().unwrap();
        profile.make_active().unwrap();

        assert_eq!(repo.active_profile().unwrap().unwrap(), profile);
        assert!(profile.is_active().unwrap());
        assert_eq!(repo.active_game().unwrap().unwrap(), game);
    }

    #[test]
    fn test_make_active_game_mismatch() {
        let repo = Repository::mock();

        let morrowind = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let skyrim = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();

        let profile = morrowind.add_profile("Test").unwrap();

        skyrim.make_active().unwrap();

        assert!(matches!(
            profile.make_active(),
            Err(Error::ParentGameMismatch)
        ))
    }

    /// Make sure the query for deleting the old edge from active_profile is wiping out any other
    /// edges.
    #[test]
    fn test_make_active_old_mod_entries_not_deleted() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();

        let mod1 = game.add_mod("BIG BOOBA", None).unwrap();
        let mod2 = game.add_mod("BIG BOOBA 2", None).unwrap();

        let profile1 = game.add_profile("Test").unwrap();
        profile1.make_active().unwrap();
        profile1.add_mod_entry(mod1).unwrap();
        profile1.add_mod_entry(mod2).unwrap();

        assert_eq!(profile1.mod_entries().unwrap().len(), 2);

        let profile2 = game.add_profile("Test2").unwrap();
        profile2.make_active().unwrap();

        assert_eq!(profile1.mod_entries().unwrap().len(), 2);
    }
}
