use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use agdb::{DbId, QueryBuilder};
use heck::ToSnakeCase;

use crate::repository::{
    CoreConfigHandle,
    db::DbHandle,
    entities::{Error, Result, game::Game, get_field},
    models::GameModel,
};

/// Represents a mod entity in the Barnacle system.
///
/// Provides methods to inspect and modify this mod's data.
/// Always reflects the current database state.
#[derive(Debug, Clone)]
pub struct Mod {
    pub(crate) id: DbId,
    valid: Arc<AtomicBool>,
    pub(crate) db: DbHandle,
    pub(crate) cfg: CoreConfigHandle,
}

impl Mod {
    pub(crate) fn from_id(id: DbId, db: DbHandle, cfg: CoreConfigHandle) -> Self {
        Self {
            id,
            valid: Arc::new(AtomicBool::new(true)),
            db,
            cfg,
        }
    }

    pub fn name(&self) -> Result<String> {
        self.is_valid()?;

        get_field(&self.db, self.id, "name")
    }

    pub fn dir(&self) -> Result<PathBuf> {
        self.is_valid()?;

        Ok(self.parent()?.dir()?.join(self.name()?.to_snake_case()))
    }

    /// Returns the parent [`Game`] of this [`Mod`]
    pub fn parent(&self) -> Result<Game> {
        self.is_valid()?;

        let parent_game_id = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<GameModel>()
                    .search()
                    .from("games")
                    .to(self.id)
                    .query(),
            )?
            .elements
            .pop()
            .expect("A successful query should not be empty")
            .id;

        Ok(Game::from_id(
            parent_game_id,
            self.db.clone(),
            self.cfg.clone(),
        ))
    }

    /// Ensure that the entity is pointing to an existent model in the database
    fn is_valid(&self) -> Result<()> {
        if self.valid.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err(Error::StaleEntity)
        }
    }
}
