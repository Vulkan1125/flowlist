//! Task entity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::repeat::RepeatRule;
use crate::ids::{ChecklistId, ProjectId, TagId, TaskId};
use crate::time::{Stamp, now};

/// A lightweight step inside a task.
///
/// Checklists are deliberately not tasks: they never appear in perspectives,
/// have no project, dates or tags, and can be ticked off independently while
/// the parent task is still open. They exist for the small "get ready" steps
/// that would be noise as first-class tasks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: ChecklistId,
    pub title: String,
    #[serde(default)]
    pub completed: bool,
}

impl ChecklistItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: ChecklistId::new(),
            title: title.into(),
            completed: false,
        }
    }
}

/// How important a task is. `None` on the task means no priority was set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub const ALL: [Priority; 3] = [Priority::High, Priority::Medium, Priority::Low];

    pub fn label(self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|priority| priority.slug() == value)
    }
}

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
    /// Small steps that belong to this task and are ticked off in place.
    #[serde(default)]
    pub checklist: Vec<ChecklistItem>,
    #[serde(default)]
    pub due: Option<Stamp>,
    #[serde(default)]
    pub defer: Option<Stamp>,
    #[serde(default)]
    pub estimate_min: Option<u32>,
    #[serde(default)]
    pub flagged: bool,
    /// Optional importance, independent of the flag.
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub repeat: Option<RepeatRule>,
    /// When true, children are worked through in order.
    #[serde(default)]
    pub sequential: bool,
    /// Local file paths for images or PDFs attached to the task. Files are
    /// referenced, never copied, so the library stays a single JSON file.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// When the task was moved to Trash. Trashed tasks stay in the library so
    /// they can be restored, but every normal perspective hides them.
    #[serde(default)]
    pub trashed_at: Option<DateTime<Utc>>,
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
            checklist: Vec::new(),
            due: None,
            defer: None,
            estimate_min: None,
            flagged: false,
            priority: None,
            completed: false,
            completed_at: None,
            repeat: None,
            sequential: false,
            attachments: Vec::new(),
            trashed_at: None,
            child_order: Vec::new(),
            created_at: at,
            modified_at: at,
        }
    }

    pub fn is_inbox(&self) -> bool {
        self.project.is_none()
    }

    pub fn is_trashed(&self) -> bool {
        self.trashed_at.is_some()
    }

    pub fn has_children(&self) -> bool {
        !self.child_order.is_empty()
    }

    /// Whether this task is deferred until later.
    pub fn is_deferred(&self, at: DateTime<Utc>) -> bool {
        self.defer.is_some_and(|stamp| stamp.at > at)
    }

    /// `(completed, total)` checklist items, for progress badges.
    pub fn checklist_progress(&self) -> (usize, usize) {
        let done = self.checklist.iter().filter(|item| item.completed).count();
        (done, self.checklist.len())
    }

    pub fn checklist_item(&self, id: &ChecklistId) -> Option<&ChecklistItem> {
        self.checklist.iter().find(|item| &item.id == id)
    }

    /// Resolve a full id, prefix or short suffix to a checklist item.
    pub fn resolve_checklist(&self, spec: &str) -> Option<&ChecklistItem> {
        self.checklist
            .iter()
            .find(|item| item.id.matches_spec(spec))
    }
}
