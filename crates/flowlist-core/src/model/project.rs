//! Project entity.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::ids::{FolderId, TagId, TaskId};
use crate::time::{Stamp, local_date, now};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    #[default]
    Active,
    OnHold,
    Done,
    Dropped,
}

impl ProjectStatus {
    pub fn label(self) -> &'static str {
        match self {
            ProjectStatus::Active => "Active",
            ProjectStatus::OnHold => "On Hold",
            ProjectStatus::Done => "Done",
            ProjectStatus::Dropped => "Dropped",
        }
    }

    pub fn is_open(self) -> bool {
        matches!(self, ProjectStatus::Active | ProjectStatus::OnHold)
    }

    pub fn is_closed(self) -> bool {
        matches!(self, ProjectStatus::Done | ProjectStatus::Dropped)
    }
}

/// OmniFocus-style project types. Parallel projects release every task at
/// once, sequential projects release one at a time, and single-action lists
/// are buckets of independent one-off tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    #[default]
    Parallel,
    Sequential,
    SingleActions,
}

impl ProjectKind {
    pub const ALL: [ProjectKind; 3] = [
        ProjectKind::Parallel,
        ProjectKind::Sequential,
        ProjectKind::SingleActions,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::Parallel => "Parallel",
            ProjectKind::Sequential => "Sequential",
            ProjectKind::SingleActions => "Single actions",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ProjectKind::Parallel => "parallel",
            ProjectKind::Sequential => "sequential",
            ProjectKind::SingleActions => "single_actions",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value.replace('-', "_").as_str() {
            "parallel" => Some(ProjectKind::Parallel),
            "sequential" => Some(ProjectKind::Sequential),
            "single_actions" | "single" | "single_action" => Some(ProjectKind::SingleActions),
            _ => None,
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ProjectKind::Parallel => "Every task is available at once.",
            ProjectKind::Sequential => "Tasks unlock one after another, top to bottom.",
            ProjectKind::SingleActions => "A bucket for one-off actions with no order.",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ProjectKind::Parallel => ProjectKind::Sequential,
            ProjectKind::Sequential => ProjectKind::SingleActions,
            ProjectKind::SingleActions => ProjectKind::Parallel,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: crate::ids::ProjectId,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub folder: Option<FolderId>,
    #[serde(default)]
    pub status: ProjectStatus,
    #[serde(default)]
    pub tags: BTreeSet<TagId>,
    #[serde(default)]
    pub due: Option<Stamp>,
    #[serde(default)]
    pub defer: Option<Stamp>,
    #[serde(default = "default_review_days")]
    pub review_interval_days: u32,
    #[serde(default = "today")]
    pub next_review: NaiveDate,
    /// Legacy boolean kept for older files. New writes use `kind`.
    #[serde(default)]
    pub sequential: bool,
    #[serde(default)]
    pub kind: ProjectKind,
    #[serde(default)]
    pub task_order: Vec<TaskId>,
    #[serde(default = "now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "now")]
    pub modified_at: DateTime<Utc>,
}

fn default_review_days() -> u32 {
    7
}

fn today() -> NaiveDate {
    local_date(now())
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let at = now();
        Self {
            id: crate::ids::ProjectId::new(),
            name: name.into(),
            notes: String::new(),
            folder: None,
            status: ProjectStatus::Active,
            tags: BTreeSet::new(),
            due: None,
            defer: None,
            review_interval_days: default_review_days(),
            next_review: local_date(at) + chrono::Duration::days(default_review_days() as i64),
            sequential: false,
            kind: ProjectKind::Parallel,
            task_order: Vec::new(),
            created_at: at,
            modified_at: at,
        }
    }

    /// The kind to show and apply, including projects written before `kind`
    /// existed with only the `sequential` boolean set.
    pub fn effective_kind(&self) -> ProjectKind {
        if self.sequential && self.kind == ProjectKind::Parallel {
            ProjectKind::Sequential
        } else {
            self.kind
        }
    }

    pub fn is_sequential(&self) -> bool {
        self.effective_kind() == ProjectKind::Sequential
    }

    pub fn is_due_for_review(&self, date: NaiveDate) -> bool {
        self.status.is_open() && self.next_review <= date
    }
}
