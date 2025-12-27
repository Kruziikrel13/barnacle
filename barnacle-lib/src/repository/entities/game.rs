use std::{
    fmt::{self, Debug, Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

use super::Error;
use agdb::{DbId, DbValue, QueryBuilder, QueryId};
use heck::ToSnakeCase;
use tracing::debug;

use crate::{
    Repository,
    repository::{
        Cfg,
        db::{
            Db,
            models::{DeployKind, GameModel, ModModel, ProfileModel},
        },
        entities::{EntityId, Result, Uid, get_field, mod_::Mod, profile::Profile, set_field},
    },
};

/// Represents a game entity in the Barnacle system.
///
/// Provides methods to inspect and modify this game's data, including
/// managing profiles and mods. Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Game {
    pub(crate) id: EntityId,
    pub(crate) db: Db,
    pub(crate) cfg: Cfg,
}

impl Game {
    /// Load some existing [`Game`] from the database
    pub(crate) fn load(db_id: DbId, db: Db, cfg: Cfg) -> Result<Self> {
        let id = EntityId::load(&db, db_id)?;
        Ok(Self { id, db, cfg })
    }

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    // TODO: Perform unique violation checking
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

    pub fn targets(&self) -> Result<Vec<PathBuf>> {
        self.get_field("targets")
    }

    pub fn deploy_kind(&self) -> Result<DeployKind> {
        self.get_field("deploy_kind")
    }

    pub fn set_deploy_kind(&self, new_deploy_kind: DeployKind) -> Result<()> {
        if new_deploy_kind == self.deploy_kind()? {
            return Ok(());
        }

        self.set_field("deploy_kind", new_deploy_kind)
    }

    pub fn dir(&self) -> Result<PathBuf> {
        Ok(self
            .cfg
            .read()
            .library_dir()
            .join(self.name()?.to_snake_case()))
    }

    pub fn remove(self) -> Result<()> {
        for p in self.profiles()? {
            let profile_name = p.name().unwrap();
            p.remove()
                .or_else(|err| match err {
                    Error::RemovedEntity => Ok(()), // if id is stale assume already removed
                    other => Err(other),
                })
                .unwrap_or_else(|_| {
                    panic!(
                        "Failed to remove profile: {} during game cleanup",
                        profile_name
                    )
                })
        }

        for m in self.mods()? {
            let mod_name = m.name().unwrap();
            m.remove()
                .or_else(|err| match err {
                    Error::RemovedEntity => Ok(()), // ditto
                    other => Err(other),
                })
                .unwrap_or_else(|_| {
                    panic!("Failed to remove mod: {} during game cleanup", mod_name)
                })
        }

        let name = self.name()?;
        let dir = self.dir()?;

        let id = self.id.db_id(&self.db)?;
        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(id).query())?;

        fs::remove_dir_all(dir).unwrap();

        debug!("Removed game: {name}");

        Ok(())
    }

    pub fn add_profile(&self, name: &str) -> Result<Profile> {
        let model = ProfileModel::new(Uid::new(&self.db)?, name);
        if self
            .profiles()?
            .iter()
            .any(|p: &Profile| p.name().unwrap() == model.name)
        {
            // return Err(Error::UniqueViolation(UniqueConstraint::ProfileName));
            panic!("Unique violation")
        }

        let game_id = self.id.db_id(&self.db)?;
        let profile_id = self.db.write().transaction_mut(|t| -> Result<DbId> {
            let profile_id = t
                .exec_mut(QueryBuilder::insert().element(model).query())?
                .elements
                .first()
                .expect("A successful query should not be empty")
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

        let profile = Profile::load(profile_id, self.db.clone(), self.cfg.clone())?;

        fs::create_dir_all(profile.dir()?).unwrap();

        Ok(profile)
    }

    pub fn profiles(&self) -> Result<Vec<Profile>> {
        Profile::list(&self.db, &self.cfg, self)
    }

    pub fn mods(&self) -> Result<Vec<Mod>> {
        let db_id = self.id.db_id(&self.db)?;
        Ok(self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModModel>()
                    .search()
                    .from(db_id)
                    .where_()
                    .neighbor()
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| Mod::load(e.id, self.db.clone(), self.cfg.clone()).unwrap())
            .collect())
    }

    pub fn add_mod(&self, name: &str, path: Option<&Path>) -> Result<Mod> {
        Mod::add(self.db.clone(), self.cfg.clone(), self, name, path)
    }

    /// Insert a new [`Game`] into the database. The [`Game`] must have a unique name.
    pub(crate) fn add(db: &Db, cfg: Cfg, name: &str, deploy_kind: DeployKind) -> Result<Self> {
        if Game::list(db.clone(), cfg.clone())?
            .iter()
            .any(|g| g.name().unwrap() == name)
        {
            // return Err(Error::UniqueViolation(UniqueConstraint::GameName));
            panic!("UniqueViolation");
        }

        let model = GameModel::new(Uid::new(db)?, name, deploy_kind);
        let db_id = db.write().transaction_mut(|t| -> Result<DbId> {
            let game_id = t
                .exec_mut(QueryBuilder::insert().element(model).query())
                .unwrap()
                .elements
                .first()
                .unwrap()
                .id;

            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from("games")
                    .to(game_id)
                    .query(),
            )
            .unwrap();

            Ok(game_id)
        })?;

        let game = Game::load(db_id, db.clone(), cfg.clone())?;

        fs::create_dir_all(game.dir().unwrap()).unwrap();

        debug!("Created new game: {}", game.name()?);

        Ok(game)
    }

    pub(crate) fn list(db: Db, cfg: Cfg) -> Result<Vec<Game>> {
        Ok(db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("games")
                    .query(),
            )?
            .elements
            .iter()
            .map(|e| Game::load(e.id, db.clone(), cfg.clone()).unwrap())
            .collect())
    }

    /// Make this game the active one
    pub fn make_active(&self) -> Result<()> {
        let db_id = self.id.db_id(&self.db)?;

        self.db.write().transaction_mut(|t| -> Result<()> {
            // Delete existing active_game, if it exists
            t.exec_mut(
                QueryBuilder::remove()
                    .search()
                    .from("active_game")
                    .where_()
                    .edge()
                    .query(),
            )?;
            // Insert a new edge from active_game to new game_id
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from("active_game")
                    .to(db_id)
                    .query(),
            )?;

            Ok(())
        })?;

        if let Some(first_profile) = self.profiles()?.first() {
            first_profile.make_active()?;
        };

        Ok(())
    }

    pub(crate) fn active(db: Db, cfg: Cfg) -> Result<Option<Game>> {
        let elements = db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("active_game")
                    .where_()
                    .neighbor()
                    .query(),
            )?
            .elements;

        let games = Game::list(db.clone(), cfg.clone())?;

        // If we have a set active game, load it
        if let Some(active) = elements.first() {
            return Ok(Some(Game::load(active.id, db, cfg)?));
        }

        // Bootstrap the active game if it isn't set and we have games
        if let Some(first) = games.first() {
            first.make_active()?;
            return Ok(Some(first.clone()));
        }

        // No active game and no games at all
        Ok(None)
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

impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            // TODO: This doesn't feel right
            self.name().unwrap_or_else(|_| "<invalid game name>".into())
        )
    }
}

impl PartialEq for Game {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod test {
    use crate::Repository;

    use super::*;

    #[test]
    fn test_add() {
        let repo = Repository::mock();

        let game1 = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();

        let games = repo.games().unwrap();

        assert!(game1.dir().unwrap().exists());
        assert_eq!(games.len(), 2);
        assert_eq!(games.first().unwrap().name().unwrap(), "Morrowind");
        assert_eq!(
            games.last().unwrap().deploy_kind().unwrap(),
            DeployKind::CreationEngine
        );
    }

    #[test]
    fn test_remove() {
        let repo = Repository::mock();

        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let profile = game.add_profile("test_profile_1").unwrap();
        let _mod = game.add_mod("test_mod", None).unwrap();

        assert_eq!(repo.games().unwrap().len(), 1);

        let dir = game.dir().unwrap();

        game.remove().unwrap();

        // Attempt to remove already removed profile and mod entries
        assert!(matches!(profile.remove(), Err(Error::RemovedEntity)));
        assert!(matches!(_mod.remove(), Err(Error::RemovedEntity)));

        assert!(!dir.exists());
        assert_eq!(repo.games().unwrap().len(), 0);
    }

    #[test]
    fn test_list() {
        let repo = Repository::mock();

        assert_eq!(repo.games().unwrap().len(), 0);

        repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();

        assert_eq!(repo.games().unwrap().len(), 1);
    }

    #[test]
    fn test_name() {
        let repo = Repository::mock();

        let game = repo
            .add_game("Fallout: New Vegas", DeployKind::Gamebryo)
            .unwrap();

        game.name().unwrap();
    }

    #[test]
    fn test_set_name() {
        let repo = Repository::mock();

        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();

        assert_eq!(game.name().unwrap(), "Skyrim");

        game.set_name("Skyrim 3: Electric Boogaloo").unwrap();

        assert_eq!(game.name().unwrap(), "Skyrim 3: Electric Boogaloo");
    }

    #[test]
    fn test_deploy_kind() {
        let repo = Repository::mock();

        let game = repo
            .add_game("Fallout: New Vegas", DeployKind::Gamebryo)
            .unwrap();

        game.deploy_kind().unwrap();
    }

    #[test]
    fn test_dir() {
        let repo = Repository::mock();

        let game = repo
            .add_game("Fallout: New Vegas", DeployKind::Gamebryo)
            .unwrap();

        let expected_dir = repo
            .cfg
            .read()
            .library_dir()
            .join(game.name().unwrap().to_snake_case());

        assert_eq!(game.dir().unwrap(), expected_dir);
    }
}
