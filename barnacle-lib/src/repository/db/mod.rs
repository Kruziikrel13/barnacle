use std::sync::Arc;

use agdb::{DbAny, DbError, QueryBuilder};
use derive_more::Deref;
use parking_lot::RwLock;

use crate::{
    fs::state_dir,
    repository::db::models::{CURRENT_MODEL_VERSION, ModelVersion},
};

pub(crate) mod models;

#[derive(Debug, Clone, Deref)]
pub(crate) struct Db {
    #[deref]
    db: Arc<RwLock<DbAny>>,
}

impl Db {
    pub fn new() -> Self {
        let path = state_dir().join("data.db");
        let path_str = path.to_str().unwrap();

        let mut db = Self {
            db: Arc::new(RwLock::new(DbAny::new_file(path_str).unwrap())),
        };

        db.init();

        db
    }

    fn init(&mut self) {
        let alias_count = self
            .db
            .read()
            .exec(QueryBuilder::select().aliases().query())
            .unwrap()
            .result;

        if alias_count == 0 {
            self.db
                .write()
                .transaction_mut(|t| -> Result<(), DbError> {
                    t.exec_mut(
                        // Insert aliases if they don't exist
                        QueryBuilder::insert()
                            .nodes()
                            .aliases([
                                // Root element nodes
                                "games",
                                "mod_entries",
                                "mods",
                                "profiles",
                                "tools",
                                // State nodes
                                "active_game",
                                "active_profile",
                                "model_version",
                                "next_uid",
                            ])
                            .query(),
                    )?;

                    // Signifies what the UID should be for a newly inserted element. It gets
                    // incremented with every new element.
                    t.exec_mut(
                        QueryBuilder::insert()
                            .values([[("next_uid", 0).into()]])
                            .ids("next_uid")
                            .query(),
                    )?;

                    Ok(())
                })
                .unwrap();
        }

        // Fetch the current model version (if any)
        let result = self
            .db
            .read()
            .exec(
                QueryBuilder::select()
                    .elements::<ModelVersion>()
                    .search()
                    .from("model_version")
                    .where_()
                    .neighbor()
                    .query(),
            )
            .unwrap();

        let model_version: Option<ModelVersion> = result.try_into().into_iter().next();

        if let Some(mv) = model_version {
            if mv.version() < CURRENT_MODEL_VERSION {
                self.backup();
                self.migrate();
            }
        } else {
            // Insert default ModelVersion if missing
            self.db
                .write()
                .transaction_mut(|t| -> Result<(), agdb::DbError> {
                    let model_version_id = t
                        .exec_mut(
                            QueryBuilder::insert()
                                .element(ModelVersion::default())
                                .query(),
                        )?
                        .elements
                        .first()
                        .unwrap()
                        .id;

                    t.exec_mut(
                        QueryBuilder::insert()
                            .edges()
                            .from("model_version")
                            .to(model_version_id)
                            .query(),
                    )?;

                    Ok(())
                })
                .unwrap();
        }
    }

    /// Perform a backup of the database
    fn backup(&self) {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let path = state_dir().join(format!("data-{}.db.bak", timestamp));
        let path_str = path.to_str().unwrap();

        self.db.write().backup(path_str).unwrap();
    }

    /// Perform database migrations
    fn migrate(&self) {
        todo!()
    }

    /// Create a memory backed database for use in tests
    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        let mut db = Self {
            db: Arc::new(RwLock::new(DbAny::new_memory("test").unwrap())),
        };

        db.init();

        db
    }
}
