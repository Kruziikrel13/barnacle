use std::path::PathBuf;

use agdb::{DbElement, DbId};

use crate::repository::entities::Uid;

#[derive(Debug, Clone, DbElement, PartialEq, PartialOrd)]
pub struct ToolModel {
    db_id: Option<DbId>,
    uid: u64,
    /// A human friendly display name
    name: String,
    /// The path to the tool's executable
    path: PathBuf,
    /// Additional command-line arguments
    args: Option<String>,
}

impl ToolModel {
    pub fn new(uid: Uid, name: &str, path: PathBuf, args: Option<&str>) -> Self {
        Self {
            db_id: None,
            uid: uid.0,
            name: name.to_string(),
            path,
            args: args.map(str::to_string),
        }
    }
}
