use std::{fmt::Debug, fs, path::PathBuf};

use super::Error;
use agdb::{CountComparison, DbId, DbValue, QueryBuilder, QueryId};
use heck::ToSnakeCase;
use tracing::debug;

use crate::repository::{
    Cfg,
    db::{
        Db,
        models::{GameModel, ProfileModel},
    },
    entities::{
        EntityId, Result, Uid, game::Game, get_field, mod_::Mod, mod_entry::ModEntry, set_field,
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
    pub fn activate(&self) -> Result<()> {
        let parent_db_id = self.parent()?.id.db_id(&self.db)?;
        let db_id = self.id.db_id(&self.db)?;
        self.db.write().transaction_mut(|t| {
            // Remove `active` field from edge pointing to existing active profile, if present
            // BUG: Is this responsible for wiping out the active profile?
            t.exec_mut(
                QueryBuilder::remove()
                    .values("active")
                    .search()
                    .from(parent_db_id)
                    .where_()
                    .edge()
                    // .and()
                    // Only delete the `active` field on edges terminating at profiles.
                    // .distance(CountComparison::Equal(1))
                    .query(),
            )?;
            // Add `active` field to edge pointing to this profile
            t.exec_mut(
                QueryBuilder::insert()
                    .values([[("active", true).into()]])
                    .search()
                    .from(parent_db_id)
                    .to(db_id)
                    .where_()
                    .edge()
                    .query(),
            )?;

            Ok(())
        })
    }

    pub fn is_active(&self) -> Result<bool> {
        Ok(
            Profile::active(self.db.clone(), self.cfg.clone(), self.parent()?)?
                == Some(self.clone()),
        )
    }

    pub(crate) fn active(db: Db, cfg: Cfg, game: Game) -> Result<Option<Profile>> {
        let game_id = game.id.db_id(&db)?;
        let elements = db
            .read()
            .exec(
                QueryBuilder::select()
                    .search()
                    .from(game_id)
                    .where_()
                    .beyond()
                    .where_()
                    .keys("active")
                    .or()
                    .node()
                    .end_where()
                    .and()
                    .element::<ProfileModel>()
                    .query(),
            )?
            .elements;

        if elements.len() > 1 {
            panic!("there should only be one active profile");
        }

        // If we have an active profile, load it
        if let Some(active) = elements.first() {
            return Ok(Some(Profile::load(active.id, db, cfg)?));
        }

        // No active profile and no profiles at all
        Ok(None)
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
            .expect("a Profile should have a parent Game")
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

        // We have to store these so we can still access them once the profile is deleted
        let parent_game = self.parent()?;
        let name = self.name()?;
        let dir = self.dir()?;

        let db_id = self.id.db_id(&self.db)?;
        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(db_id).query())?;

        fs::remove_dir_all(dir).unwrap();

        // Bootstrap active profile if there isn't one set
        if Profile::active(self.db.clone(), self.cfg.clone(), parent_game.clone())?.is_none()
            && let Some(first_profile) =
                Profile::list(&self.db.clone(), &self.cfg.clone(), &parent_game.clone())?.first()
        {
            first_profile.activate()?;
        }

        debug!("Removed profile: {name}");

        Ok(())
    }

    pub(crate) fn add(db: &Db, cfg: &Cfg, game: &Game, name: &str) -> Result<Self> {
        let model = ProfileModel::new(Uid::new(db)?, name);
        if game
            .profiles()?
            .iter()
            .any(|p: &Profile| p.name().unwrap() == model.name)
        {
            return Err(Error::DuplicateName);
        }

        let game_id = game.id.db_id(db)?;
        let profile_id = db.write().transaction_mut(|t| -> Result<DbId> {
            let profile_id = t
                .exec_mut(QueryBuilder::insert().element(model).query())?
                .elements
                .first()
                .expect("ProfileModel insertion should return the ID as the first element")
                .id;

            // Link Profile to the specified Game node and root "profiles" node
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from([QueryId::from("profiles"), QueryId::from(game_id)])
                    .to(profile_id)
                    .query(),
            )?;

            Ok(profile_id)
        })?;

        let profile = Profile::load(profile_id, db.clone(), cfg.clone())?;

        fs::create_dir_all(profile.dir()?).unwrap();

        // Bootstrap active profile if there isn't one set
        if Profile::active(db.clone(), cfg.clone(), game.clone())?.is_none()
            && let Some(first_profile) =
                Profile::list(&db.clone(), &cfg.clone(), &game.clone())?.first()
        {
            first_profile.activate()?;
            return Ok(first_profile.clone());
        }

        Ok(profile)
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

    /// Search for a profile under the given game by name
    pub(crate) fn search(db: Db, cfg: Cfg, game: &Game, name: &str) -> Result<Option<Profile>> {
        let game_id = game.id.db_id(&db)?;
        db.read()
            .exec(
                QueryBuilder::select()
                    .element::<ProfileModel>()
                    .search()
                    .from(game_id)
                    .where_()
                    .key("name")
                    .value(name)
                    .query(),
            )?
            .elements
            .first()
            .map(|p| Profile::load(p.id, db.clone(), cfg.clone()))
            .transpose()
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
    fn test_add_duplicate() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        game.add_profile("Test").unwrap();

        assert!(matches!(
            game.add_profile("Test"),
            Err(Error::DuplicateName)
        ));
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
    fn test_activate() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();

        let profile1 = game.add_profile("Test1").unwrap();
        let profile2 = game.add_profile("Test2").unwrap();

        // First profile should have been automatically set as active
        assert!(profile1.is_active().unwrap());

        profile2.activate().unwrap();

        assert!(profile2.is_active().unwrap());
    }

    #[test]
    fn test_remove_made_next_profile_active() {
        let repo = Repository::mock();
        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();

        let profile1 = game.add_profile("Test1").unwrap();
        let profile2 = game.add_profile("Test2").unwrap();

        profile1.activate().unwrap();
        assert!(profile1.is_active().unwrap());

        profile1.remove().unwrap();
        assert!(profile2.is_active().unwrap());
    }
}
