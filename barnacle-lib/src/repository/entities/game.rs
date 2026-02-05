use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use super::Error;
use agdb::{CountComparison, DbId, DbValue, QueryBuilder};
use heck::ToSnakeCase;
use tracing::info;

use crate::repository::{
    Cfg,
    db::{
        Db,
        models::{DeployKind, GameModel, ModModel},
    },
    entities::{EntityId, Result, Uid, get_field, mod_::Mod, profile::Profile, set_field},
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

        let old_dir = self.library_dir()?;

        self.set_field("name", new_name)?;

        let new_dir = self.library_dir()?;
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

    pub fn library_dir(&self) -> Result<PathBuf> {
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

        // We have to store these so we can still access them once the game is deleted
        let name = self.name()?;
        let dir = self.library_dir()?;
        let id = self.id.db_id(&self.db)?;
        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(id).query())?;

        fs::remove_dir_all(dir).unwrap();

        // Bootstrap active game if there isn't one set
        if Game::active(self.db.clone(), self.cfg.clone())?.is_none()
            && let Some(first_game) = Game::list(self.db.clone(), self.cfg.clone())?.first()
        {
            first_game.activate()?;
        }

        info!("Removed game: {name}");

        Ok(())
    }

    pub fn add_profile(&self, name: &str) -> Result<Profile> {
        Profile::add(&self.db, &self.cfg, self, name)
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
            return Err(Error::DuplicateName);
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

        fs::create_dir_all(game.library_dir().unwrap()).unwrap();

        // Bootstrap active game if there isn't one set
        if Game::active(db.clone(), cfg.clone())?.is_none()
            && let Some(first_game) = Game::list(db.clone(), cfg.clone())?.first()
        {
            first_game.activate()?;
        }

        info!("Created new game: {}", game.name()?);

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

    /// Search for a game by name
    pub(crate) fn search(db: Db, cfg: Cfg, name: &str) -> Result<Option<Game>> {
        db.read()
            .exec(
                QueryBuilder::select()
                    .element::<GameModel>()
                    .search()
                    .from("games")
                    .where_()
                    .key("name")
                    .value(name)
                    .query(),
            )?
            .elements
            .first()
            .map(|g| Game::load(g.id, db.clone(), cfg.clone()))
            .transpose()
    }

    /// Make this game the active one
    pub fn activate(&self) -> Result<()> {
        let db_id = self.id.db_id(&self.db)?;
        self.db.write().transaction_mut(|t| -> Result<()> {
            // Delete existing active_game, if it exists
            t.exec_mut(
                QueryBuilder::remove()
                    .search()
                    .from("active_game")
                    .where_()
                    .edge()
                    .and()
                    // Only delete the first edge. We don't want to accidentally wipe out all edges
                    // coming from active_game
                    .distance(CountComparison::Equal(1))
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

        Ok(())
    }

    pub fn is_active(&self) -> Result<bool> {
        Ok(Game::active(self.db.clone(), self.cfg.clone())? == Some(self.clone()))
    }

    pub(crate) fn active(db: Db, cfg: Cfg) -> Result<Option<Game>> {
        db.read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("active_game")
                    .where_()
                    .neighbor()
                    .query(),
            )?
            .elements
            .first()
            .map(|g| Game::load(g.id, db.clone(), cfg.clone()))
            .transpose()
    }

    pub fn active_profile(&self) -> Result<Option<Profile>> {
        Profile::active(self.db.clone(), self.cfg.clone(), self.clone())
    }

    /// Search for the given profile by name
    pub fn search_profile(&self, name: &str) -> Result<Option<Profile>> {
        Profile::search(self.db.clone(), self.cfg.clone(), self, name)
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

        assert!(game1.library_dir().unwrap().exists());
        assert_eq!(games.len(), 2);
        assert_eq!(games.first().unwrap().name().unwrap(), "Morrowind");
        assert_eq!(
            games.last().unwrap().deploy_kind().unwrap(),
            DeployKind::CreationEngine
        );
    }

    #[test]
    fn test_add_duplicate() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();

        assert!(matches!(
            repo.add_game("Morrowind", DeployKind::OpenMW),
            Err(Error::DuplicateName)
        ));
    }

    #[test]
    fn test_remove() {
        let repo = Repository::mock();

        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let profile = game.add_profile("test_profile_1").unwrap();
        let _mod = game.add_mod("test_mod", None).unwrap();

        assert_eq!(repo.games().unwrap().len(), 1);

        let dir = game.library_dir().unwrap();

        game.remove().unwrap();

        // Attempt to remove already removed profile and mod entries
        assert!(matches!(profile.remove(), Err(Error::RemovedEntity)));
        assert!(matches!(_mod.remove(), Err(Error::RemovedEntity)));

        assert!(!dir.exists());
        assert_eq!(repo.games().unwrap().len(), 0);
    }

    #[test]
    fn test_remove_made_next_game_active() {
        let repo = Repository::mock();
        let game1 = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let game2 = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();

        game1.activate().unwrap();
        assert!(game1.is_active().unwrap());

        game1.remove().unwrap();
        assert!(game2.is_active().unwrap());
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

        assert_eq!(game.library_dir().unwrap(), expected_dir);
    }

    #[test]
    fn test_activate() {
        let repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();

        game.activate().unwrap();

        assert!(game.is_active().unwrap());
        assert_eq!(repo.active_game().unwrap().unwrap(), game);
    }
}
