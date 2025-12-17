use std::{
    fmt::{self, Debug, Display, Formatter},
    fs::{self, File},
    path::{Path, PathBuf},
};

use agdb::{DbId, DbValue, QueryBuilder, QueryId};
use compress_tools::{Ownership, uncompress_archive};
use heck::ToSnakeCase;
use tracing::debug;

use crate::{
    fs::{Permissions, change_dir_permissions},
    repository::{
        Cfg,
        db::{
            Db,
            models::{GameModel, ModModel},
        },
        entities::{EntityId, Result, Uid, game::Game, get_field, set_field},
    },
};

/// Represents a mod entity in the Barnacle system.
///
/// Provides methods to inspect and modify this mod's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Mod {
    pub(crate) id: EntityId,
    pub(crate) db: Db,
    pub(crate) cfg: Cfg,
}

impl Mod {
    pub(crate) fn load(db_id: DbId, db: Db, cfg: Cfg) -> Result<Self> {
        let id = EntityId::load(&db, db_id)?;
        Ok(Self { id, db, cfg })
    }

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn dir(&self) -> Result<PathBuf> {
        Ok(self
            .parent()?
            .dir()?
            .join("mods")
            .join(self.name()?.to_snake_case()))
    }

    /// Returns the parent [`Game`] of this [`Mod`]
    pub fn parent(&self) -> Result<Game> {
        let db_id = self.id.db_id(&self.db)?;
        let parent_game_id = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("games")
                    .to(db_id)
                    .query(),
            )?
            .elements
            .pop()
            .expect("A successful query should not be empty")
            .id;

        Game::load(parent_game_id, self.db.clone(), self.cfg.clone())
    }

    pub(crate) fn add(
        db: Db,
        cfg: Cfg,
        game: &Game,
        name: &str,
        path: Option<&Path>,
    ) -> Result<Self> {
        let game_id = game.id.db_id(&db)?;

        let model = ModModel::new(Uid::new(&db)?, name);
        let mod_id = db.write().transaction_mut(|t| -> Result<DbId> {
            let mod_id = t
                .exec_mut(QueryBuilder::insert().element(model).query())?
                .elements
                .first()
                .expect("A successful query should not be empty")
                .id;

            // Link Mod to the specified Game node and root "mods" node
            t.exec_mut(
                QueryBuilder::insert()
                    .edges()
                    .from([QueryId::from("mods"), QueryId::from(game_id)])
                    .to(mod_id)
                    .query(),
            )?;

            Ok(mod_id)
        })?;

        let mod_ = Mod::load(mod_id, db.clone(), cfg.clone())?;

        // TODO: Only attempt to open the archive if the input_path is an archive
        if let Some(path) = path {
            let archive = File::open(path).unwrap();
            uncompress_archive(archive, &mod_.dir()?, Ownership::Preserve).unwrap();
            change_dir_permissions(&mod_.dir()?, Permissions::ReadOnly);
        } else {
            let path = mod_.dir()?;
            fs::create_dir_all(path).unwrap();
        };

        Ok(mod_)
    }

    pub(crate) fn remove(self) -> Result<()> {
        let name = self.name()?;
        let dir = self.dir()?;

        let db_id = self.id.db_id(&self.db)?;
        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(db_id).query())?;

        fs::remove_dir_all(dir).unwrap();

        debug!("Removed mod: {name}");

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
        set_field(&self.db, self.id, field, value)
    }
}

impl Display for Mod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name().unwrap_or_else(|_| "<invalid mod name>".into())
        )
    }
}

impl PartialEq for Mod {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod test {
    use crate::{Repository, repository::DeployKind};

    #[test]
    fn test_add() {
        let mut repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let mod_ = game.add_mod("Test", None).unwrap();

        assert!(mod_.dir().unwrap().exists());
    }

    #[test]
    fn test_remove() {
        let mut repo = Repository::mock();

        let game = repo.add_game("Skyrim", DeployKind::CreationEngine).unwrap();
        let mod_ = game.add_mod("Test", None).unwrap();

        assert_eq!(game.mods().unwrap().len(), 1);

        let dir = mod_.dir().unwrap();

        game.remove_mod(mod_).unwrap();

        assert_eq!(game.mods().unwrap().len(), 0);
        assert!(!dir.exists())
    }

    #[test]
    fn test_parent() {
        let mut repo = Repository::mock();

        let game = repo.add_game("Morrowind", DeployKind::OpenMW).unwrap();
        let mod_ = game.add_mod("Test", None).unwrap();

        assert_eq!(mod_.parent().unwrap(), game);
    }

    #[test]
    fn test_name() {
        let mut repo = Repository::mock();

        repo.add_game("Fallout: New Vegas", DeployKind::Gamebryo)
            .unwrap()
            .add_mod("Test", None)
            .unwrap()
            .name()
            .unwrap();
    }
}
