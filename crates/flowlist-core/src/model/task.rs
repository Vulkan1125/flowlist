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

/// Which date a custom notification is anchored to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertEvent {
    Due,
    Defer,
    Planned,
}

impl AlertEvent {
    pub const ALL: [AlertEvent; 3] = [AlertEvent::Due, AlertEvent::Defer, AlertEvent::Planned];

    pub fn label(self) -> &'static str {
        match self {
            AlertEvent::Due => "Due",
            AlertEvent::Defer => "Available",
            AlertEvent::Planned => "Planned",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            AlertEvent::Due => "due",
            AlertEvent::Defer => "defer",
            AlertEvent::Planned => "planned",
        }
    }
}

/// One per-task notification. `offset_minutes` is relative to the event:
/// negative fires before it, zero at the event, positive after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAlert {
    pub event: AlertEvent,
    pub offset_minutes: i32,
}

impl TaskAlert {
    pub fn new(event: AlertEvent, offset_minutes: i32) -> Self {
        Self {
            event,
            offset_minutes,
        }
    }

    /// Parse compact alert text such as `due:-15m, defer, planned:+1h`.
    pub fn parse_list(value: &str) -> std::result::Result<Vec<Self>, String> {
        let value = value.trim();
        if value.is_empty()
            || value.eq_ignore_ascii_case("none")
            || value.eq_ignore_ascii_case("default")
        {
            return Ok(Vec::new());
        }
        let mut alerts = Vec::new();
        for token in value.split([',', ';']) {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let (event, offset) = token.split_once(':').unwrap_or((token, "0"));
            let event = match event.trim().to_lowercase().as_str() {
                "due" => AlertEvent::Due,
                "defer" | "available" => AlertEvent::Defer,
                "planned" | "plan" => AlertEvent::Planned,
                other => {
                    return Err(format!(
                        "Unknown alert event “{other}”; use due, defer or planned"
                    ));
                }
            };
            let offset = offset.trim();
            let offset_minutes = if offset.is_empty() || offset == "0" || offset == "+0" {
                0
            } else {
                let (sign, amount) = if let Some(amount) = offset.strip_prefix('-') {
                    (-1, amount)
                } else if let Some(amount) = offset.strip_prefix('+') {
                    (1, amount)
                } else {
                    (1, offset)
                };
                let minutes = crate::parse::parse_duration(amount.trim())
                    .map_err(|error| format!("Bad alert offset “{offset}”: {error}"))?;
                (minutes as i32) * sign
            };
            let alert = Self::new(event, offset_minutes);
            if !alerts.contains(&alert) {
                alerts.push(alert);
            }
        }
        if alerts.len() > 12 {
            return Err("Use at most 12 alerts per task".to_string());
        }
        Ok(alerts)
    }

    pub fn format_list(alerts: &[Self]) -> String {
        alerts
            .iter()
            .map(|alert| {
                if alert.offset_minutes == 0 {
                    alert.event.slug().to_string()
                } else {
                    let sign = if alert.offset_minutes < 0 { "-" } else { "+" };
                    format!(
                        "{}:{sign}{}",
                        alert.event.slug(),
                        format_alert_minutes(alert.offset_minutes.unsigned_abs())
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_alert_minutes(minutes: u32) -> String {
    if minutes % 60 == 0 {
        format!("{}h", minutes / 60)
    } else if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h{}m", minutes / 60, minutes % 60)
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
    /// A planning date that never changes availability.
    #[serde(default)]
    pub planned: Option<Stamp>,
    #[serde(default)]
    pub estimate_min: Option<u32>,
    #[serde(default)]
    pub flagged: bool,
    /// Take over the screen when this task comes due. Marks on an ancestor
    /// project, folder or parent task also opt the task in.
    #[serde(default)]
    pub fullscreen_reminder: bool,
    /// When the full-screen reminder should fire. Only used when
    /// `reminder_match_due` is false.
    #[serde(default)]
    pub reminder_at: Option<Stamp>,
    /// Use the due date and time instead of `reminder_at`.
    #[serde(default = "default_true")]
    pub reminder_match_due: bool,
    /// Per-task notification schedules. Empty means the default due reminder.
    #[serde(default)]
    pub alerts: Vec<TaskAlert>,
    /// Optional importance, independent of the flag.
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub repeat: Option<RepeatRule>,
    /// The first occurrence of a repeating series. Every occurrence carries the
    /// same value, so the whole chain can be found when deleting a series. A
    /// task that predates series tracking is treated as its own series.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_series: Option<TaskId>,
    /// When true, children are worked through in order.
    #[serde(default)]
    pub sequential: bool,
    /// When true, this task's steps are one-off checklist items instead of
    /// separate child tasks. The two modes are mutually exclusive.
    #[serde(default)]
    pub steps_one_off: bool,
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
            planned: None,
            estimate_min: None,
            flagged: false,
            fullscreen_reminder: false,
            reminder_at: None,
            reminder_match_due: true,
            alerts: Vec::new(),
            priority: None,
            completed: false,
            completed_at: None,
            repeat: None,
            repeat_series: None,
            sequential: false,
            steps_one_off: false,
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

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{AlertEvent, TaskAlert};

    #[test]
    fn alerts_parse_format_and_reject_unknown_events() {
        let alerts = TaskAlert::parse_list("due:-15m, defer, planned:+1h30m").unwrap();
        assert_eq!(alerts.len(), 3);
        assert_eq!(alerts[0], TaskAlert::new(AlertEvent::Due, -15));
        assert_eq!(alerts[1], TaskAlert::new(AlertEvent::Defer, 0));
        assert_eq!(alerts[2], TaskAlert::new(AlertEvent::Planned, 90));
        assert_eq!(
            TaskAlert::format_list(&alerts),
            "due:-15m, defer, planned:+1h30m"
        );
        assert!(TaskAlert::parse_list("sometime:-5m").is_err());
        assert!(TaskAlert::parse_list("none").unwrap().is_empty());
        assert_eq!(
            TaskAlert::parse_list("due:-5m, due:-5m").unwrap().len(),
            1
        );
    }
}
