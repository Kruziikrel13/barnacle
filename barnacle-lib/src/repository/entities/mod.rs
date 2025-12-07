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
    #[error("This entity refers to a model that has been deleted")]
    StaleEntity,
}

pub(crate) fn set_field<T>(db: &mut DbHandle, id: DbId, field: &str, value: T) -> Result<()>
where
    T: Into<DbValue>,
{
    db.write().exec_mut(
        QueryBuilder::insert()
            .values([[(field, value).into()]])
            .ids(id)
            .query(),
    )?;

    Ok(())
}

/// Get a [`Uid`] to be used with a newly inserted element.
pub(crate) fn next_uid(db: &mut DbHandle) -> Result<Uid> {
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
        Ok(uid)
    })
}

/// Get the [`Uid`] for a particular model
pub(crate) fn uid(db: &DbHandle, db_id: DbId) -> Result<Uid> {
    Ok(db
        .read()
        .exec(QueryBuilder::select().values("uid").ids(db_id).query())?
        .elements
        .pop()
        .expect("successful queries should not be empty")
        .values
        .pop()
        .expect("successful queries should not be empty")
        .value
        .to_u64()?)
}
