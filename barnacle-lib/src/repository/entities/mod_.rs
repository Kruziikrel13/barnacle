use std::{fmt::Debug, fs, path::PathBuf};

use agdb::{DbValue, QueryBuilder};
use heck::ToSnakeCase;
use tracing::debug;

use crate::repository::{
    CoreConfigHandle,
    db::{DbHandle, models::GameModel},
    entities::{ElementId, Result, game::Game},
};

/// Represents a mod entity in the Barnacle system.
///
/// Provides methods to inspect and modify this mod's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Mod {
    pub(crate) id: ElementId,
    pub(crate) db: DbHandle,
    pub(crate) cfg: CoreConfigHandle,
}

impl Mod {
    pub(crate) fn load(id: ElementId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
        Ok(Self { id, db, cfg })
    }

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn dir(&self) -> Result<PathBuf> {
        Ok(self.parent()?.dir()?.join(self.name()?.to_snake_case()))
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

        let id = ElementId::load(&self.db, parent_game_id)?;
        Game::load(id, self.db.clone(), self.cfg.clone())
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
        let db_id = self.id.db_id(&self.db)?;
        let value = self
            .db
            .read()
            .exec(QueryBuilder::select().values(field).ids(db_id).query())?
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
        let db_id = self.id.db_id(&self.db)?;
        self.db.write().exec_mut(
            QueryBuilder::insert()
                .values([[(field, value).into()]])
                .ids(db_id)
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
        game.add_mod("Test", None).unwrap();
    }
}
