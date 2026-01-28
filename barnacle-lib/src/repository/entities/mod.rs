//! Core domain entities for Barnacle.
//!
//! These types represent games, profiles, mods, and other elements managed by
//! the system. They provide a unified interface for inspecting and mutating
//! these elements, handling all necessary operations behind the scenes.

use std::fmt::Debug;

use agdb::{DbId, DbValue, QueryBuilder};
use derive_more::PartialEq;
use thiserror::Error;

use crate::repository::db::Db;

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
    #[error("This entity has been deleted")]
    RemovedEntity,
    #[error("The profile you are trying to make active is not a child of the active game")]
    ParentGameMismatch,
    #[error("An entity with the given name already exists")]
    DuplicateName,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityId {
    #[partial_eq(ignore)]
    db_id: DbId,
    /// A unique idenifier that specifies a particular entity
    uid: Uid,
}

impl EntityId {
    /// Load an [`ElementId`] from an existing element.
    pub fn load(db: &Db, db_id: DbId) -> Result<Self> {
        Ok(Self {
            db_id,
            uid: Uid::load(db, db_id)?,
        })
    }

    /// Get the underlying [`DbId`]. This will check to make sure it isn't stale before returning.
    pub fn db_id(&self, db: &Db) -> Result<DbId> {
        let uid = Uid::load(db, self.db_id).map_err(|err| {
            match err {
                Error::Internal(e) => {
                    // TODO: Match on DbError kind once the following is completed:
                    // https://github.com/agnesoft/agdb/issues/1687
                    let not_found = format!("Id '{}' not found", self.db_id.as_index());
                    if e.description == not_found {
                        Error::RemovedEntity
                    } else {
                        Error::Internal(e)
                    }
                }
                other => other,
            }
        })?;

        // If the UID changed, that means this DbId now refers to a different or deleted entity
        if uid != self.uid {
            return Err(Error::RemovedEntity);
        }

        Ok(self.db_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Uid(pub u64);

impl Uid {
    fn new(db: &Db) -> Result<Self> {
        db.write().transaction_mut(|t| {
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

            Ok(Uid(uid))
        })
    }

    fn load(db: &Db, db_id: DbId) -> Result<Self> {
        Ok(Uid(db
            .read()
            .exec(QueryBuilder::select().values("uid").ids(db_id).query())?
            .elements
            .pop()
            .expect("a database element should have a UID field")
            .values
            .pop()
            .expect("a database element should have a UID field")
            .value
            .to_u64()?))
    }
}

pub(crate) fn get_field<T>(db: &Db, id: EntityId, field: &str) -> Result<T>
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
        .expect("the given field must exist")
        .values
        .pop()
        .expect("the given field must have a value")
        .value;

    Ok(T::try_from(value).expect("conversion from a `DbValue` must succeed"))
}

pub(crate) fn set_field<T>(db: &Db, id: EntityId, field: &str, value: T) -> Result<()>
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
