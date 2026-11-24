//! Folder entity: the container hierarchy above projects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ContainerId, FolderId};
use crate::time::now;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: FolderId,
    pub name: String,
    #[serde(default)]
    pub parent: Option<FolderId>,
    #[serde(default)]
    pub child_order: Vec<ContainerId>,
    /// Opt every project and task in this folder into full-screen reminders.
    #[serde(default)]
    pub fullscreen_reminder: bool,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub modified_at: DateTime<Utc>,
}

impl Folder {
    pub fn new(name: impl Into<String>) -> Self {
        let at = now();
        Self {
            id: FolderId::new(),
            name: name.into(),
            parent: None,
            child_order: Vec::new(),
            fullscreen_reminder: false,
            created_at: at,
            modified_at: at,
        }
    }
}
