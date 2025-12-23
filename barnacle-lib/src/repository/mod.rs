use std::sync::Arc;

use parking_lot::RwLock;

use crate::{
    Result,
    repository::{
        config::{Cfg, CoreConfig},
        db::Db,
    },
};

mod db;

pub mod config;
pub mod entities;

pub use db::models::DeployKind;
pub use entities::{Game, Mod, ModEntry, Profile, Tool};

/// Central access point for all persistent data.
///
/// The [`Repository`] handles both on-disk filesystem operations and all
/// database and configuration file queries. It provides a single, consistent interface
/// for reading and writing game data, mods, and profiles.
#[derive(Clone, Debug)]
pub struct Repository {
    db: Db,
    cfg: Cfg,
}

impl Repository {
    pub fn new() -> Self {
        Self {
            db: Db::new(),
            cfg: Arc::new(RwLock::new(CoreConfig::load())),
        }
    }

    pub fn add_game(&self, name: &str, deploy_kind: DeployKind) -> Result<Game> {
        Ok(Game::add(
            &self.db.clone(),
            self.cfg.clone(),
            name,
            deploy_kind,
        )?)
    }

    pub fn games(&self) -> Result<Vec<Game>> {
        Ok(Game::list(self.db.clone(), self.cfg.clone())?)
    }

    pub fn set_current_profile(&self, profile: &Profile) -> Result<()> {
        Ok(Profile::set_current(self.db.clone(), profile)?)
    }

    pub fn current_profile(&self) -> Result<Option<Profile>> {
        Ok(Profile::current(self.db.clone(), self.cfg.clone())?)
    }

    pub fn current_game(&self) -> Result<Option<Game>> {
        Game::current(self)
    }

    #[cfg(test)]
    /// Return are mock version of a [`Repository`] with an in-memory database and configuration
    /// file.
    pub(crate) fn mock() -> Self {
        Self {
            db: Db::in_memory(),
            cfg: Arc::new(RwLock::new(CoreConfig::mock())),
        }
    }
}

impl Default for Repository {
    fn default() -> Self {
        Self::new()
    }
}
