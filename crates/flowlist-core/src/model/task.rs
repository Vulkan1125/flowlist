//! Task entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::repeat::RepeatRule;
use crate::ids::{ProjectId, TagId, TaskId};
use crate::time::{Stamp, now};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub project: Option<ProjectId>,
    #[serde(default)]
    pub parent: Option<TaskId>,
    #[serde(default)]
    pub tags: BTreeSet<TagId>,
    #[serde(default)]
    pub due: Option<Stamp>,
    #[serde(default)]
    pub defer: Option<Stamp>,
    #[serde(default)]
    pub estimate_min: Option<u32>,
    #[serde(default)]
    pub flagged: bool,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub repeat: Option<RepeatRule>,
    /// When true, children are worked through in order.
    #[serde(default)]
    pub sequential: bool,
    #[serde(default)]
    pub child_order: Vec<TaskId>,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub modified_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: impl Into<String>) -> Self {
        let at = now();
        Self {
            id: TaskId::new(),
            title: title.into(),
            notes: String::new(),
            project: None,
            parent: None,
            tags: BTreeSet::new(),
            due: None,
            defer: None,
            estimate_min: None,
            flagged: false,
            completed: false,
            completed_at: None,
            repeat: None,
            sequential: false,
            child_order: Vec::new(),
            created_at: at,
            modified_at: at,
        }
    }

    pub fn is_inbox(&self) -> bool {
        self.project.is_none()
    }

    pub fn has_children(&self) -> bool {
        !self.child_order.is_empty()
    }

    /// Whether this task is deferred until later.
    pub fn is_deferred(&self, at: DateTime<Utc>) -> bool {
        self.defer.is_some_and(|stamp| stamp.at > at)
    }
}
