//! Core domain entities for Barnacle.
//!
//! These types represent games, profiles, mods, and other elements managed by
//! the system. They provide a unified interface for inspecting and mutating
//! these elements, handling all necessary operations behind the scenes.

use std::fmt::Debug;

use agdb::{DbId, DbValue, QueryBuilder};
use thiserror::Error;

use crate::repository::db::{DbHandle, Uid};

mod game;
mod mod_;
mod mod_entry;
mod profile;
mod tool;

pub use game::Game;
pub use mod_::Mod;
pub use mod_entry::ModEntry;
pub use profile::Profile;
pub use tool::Tool;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Internal database error {0}")]
    Internal(#[from] agdb::DbError),
    #[error("This EntityId refers to a model that has been deleted")]
    StaleEntityId,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityId {
    db_id: DbId,
    /// A unique idenifier that specifies a particular entity
    uid: Uid,
}

impl EntityId {
    /// Creates a new [`EntityId`] for a freshly inserted element.
    ///
    /// Allocates a new UID for the entity in the database and guarantees that the
    /// resulting [`EntityId`] refers to a valid, unique entity. The provided closure
    /// is called with the newly allocated UID and can be used to perform any initialization
    /// logic for the entity (e.g. linking edges). This is to prevent a caller clobbering an
    /// existing entity's UID by calling this function.
    pub fn create<F>(db: &DbHandle, insert_element: F) -> Result<Self>
    where
        F: FnOnce(u64) -> Result<DbId>,
    {
        let uid = db.write().transaction_mut(|t| -> Result<u64> {
            let uid = t
                .exec(
                    QueryBuilder::select()
                        .values("next_uid")
                        .ids("next_uid")
                        .query(),
                )?
                .elements
                .pop()
                .unwrap()
                .values
                .pop()
                .unwrap()
                .value
                .to_u64()
                .unwrap();
            t.exec_mut(
                QueryBuilder::insert()
                    .values([[("next_uid", uid + 1).into()]])
                    .ids("next_uid")
                    .query(),
            )?;

            Ok(uid)
        })?;

        let db_id = insert_element(uid)?;
        Ok(Self { db_id, uid })
    }

    /// Load an [`ElementId`] from an existing element.
    pub fn load(db: &DbHandle, db_id: DbId) -> Result<Self> {
        let uid = db
            .read()
            .exec(QueryBuilder::select().values("uid").ids(db_id).query())?
            .elements
            .pop()
            .expect("successful queries should not be empty")
            .values
            .pop()
            .expect("successful queries should not be empty")
            .value
            .to_u64()?;

        Ok(Self { db_id, uid })
    }

    /// Get the underlying [`DbId`]. This will check to make sure it isn't stale before returning.
    pub fn db_id(&self, db: &DbHandle) -> Result<DbId> {
        let mut values = db
            .read()
            .exec(QueryBuilder::select().values("uid").ids(self.db_id).query())?
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
            return Err(Error::StaleEntityId);
        } else {
            Ok(self.db_id)
        }
    }
}

fn get_field<T>(db: &DbHandle, id: EntityId, field: &str) -> Result<T>
where
    T: TryFrom<DbValue>,
    T::Error: Debug,
{
    let db_id = id.db_id(db)?;
    let value = db
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

pub(crate) fn set_field<T>(db: &mut DbHandle, id: EntityId, field: &str, value: T) -> Result<()>
where
    T: Into<DbValue>,
{
    let db_id = id.db_id(db)?;
    db.write().exec_mut(
        QueryBuilder::insert()
            .values([[(field, value).into()]])
            .ids(db_id)
            .query(),
    )?;

    Ok(())
}
