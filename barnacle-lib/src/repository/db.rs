use std::sync::Arc;

use agdb::{DbAny, QueryBuilder};
use derive_more::Deref;
use parking_lot::RwLock;

use crate::{
    fs::state_dir,
    repository::models::{CURRENT_MODEL_VERSION, ModelVersion},
};

#[derive(Debug, Clone, Deref)]
pub(crate) struct DbHandle {
    #[deref]
    db: Arc<RwLock<DbAny>>,
}

impl DbHandle {
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
        // Insert aliases if they don't exist
        if self
            .db
            .read()
            .exec(QueryBuilder::select().aliases().query())
            .unwrap()
            .result
            == 0
        {
            self.db
                .write()
                .exec_mut(
                    QueryBuilder::insert()
                        .nodes()
                        .aliases([
                            "games",
                            "profiles",
                            "mods",
                            "tools",
                            // State
                            "current_profile",
                            "model_version",
                        ])
                        .query(),
                )
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
