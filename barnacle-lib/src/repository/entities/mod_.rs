use std::{fmt::Debug, fs, path::PathBuf};

use agdb::{DbId, DbValue, QueryBuilder};
use heck::ToSnakeCase;
use tracing::debug;

use crate::repository::{
    CoreConfigHandle,
    db::{DbHandle, Uid, models::GameModel},
    entities::{Error, Result, game::Game, uid},
};

/// Represents a mod entity in the Barnacle system.
///
/// Provides methods to inspect and modify this mod's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Mod {
    pub(crate) db_id: DbId,
    pub(crate) uid: Uid,
    pub(crate) db: DbHandle,
    pub(crate) cfg: CoreConfigHandle,
}

impl Mod {
    pub(crate) fn from_id(db_id: DbId, db: DbHandle, cfg: CoreConfigHandle) -> Result<Self> {
        Ok(Self {
            db_id,
            uid: uid(&db, db_id)?,
            db,
            cfg,
        })
    }

    pub fn name(&self) -> Result<String> {
        self.get_field("name")
    }

    pub fn dir(&self) -> Result<PathBuf> {
        Ok(self.parent()?.dir()?.join(self.name()?.to_snake_case()))
    }

    /// Returns the parent [`Game`] of this [`Mod`]
    pub fn parent(&self) -> Result<Game> {
        let parent_game_id = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("games")
                    .to(self.db_id)
                    .query(),
            )?
            .elements
            .pop()
            .expect("A successful query should not be empty")
            .id;

        Game::from_id(parent_game_id, self.db.clone(), self.cfg.clone())
    }

    pub(crate) fn remove(self) -> Result<()> {
        let name = self.name()?;
        let dir = self.dir()?;

        self.db
            .write()
            .exec_mut(QueryBuilder::remove().ids(self.db_id).query())?;

        fs::remove_dir_all(dir).unwrap();

        debug!("Removed mod: {name}");

        Ok(())
    }

    fn get_field<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<DbValue>,
        T::Error: Debug,
    {
        let mut values = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .values([[field, "uid"]])
                    .ids(self.db_id)
                    .query(),
            )?
            .elements
            .pop()
            .expect("successful queries should not be empty")
            .values;

        let uid = values
            .pop()
            .expect("successful queries should not be empty")
            .value
            .to_u64()?;

        if uid != self.uid {
            return Err(Error::StaleEntity);
        }

        let value = values
            .pop()
            .expect("successful queries should not be empty")
            .value;

        Ok(T::try_from(value)
        .expect("Conversion from a `DbValue` must succeed. Perhaps the wrong type was expected from this field."))
    }
}
