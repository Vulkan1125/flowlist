//! Evaluating saved filters against tasks.

use chrono::{DateTime, Duration, NaiveDate, Utc};

use crate::db::Db;
use crate::model::{Condition, DueWindow, FilterNode, Task, TaskStatus};
use crate::query::availability::{self, Availability};
use crate::time::{Stamp, local_date};

pub fn matches(db: &Db, task: &Task, node: &FilterNode, at: DateTime<Utc>) -> bool {
    match node {
        FilterNode::All(nodes) => nodes.iter().all(|node| matches(db, task, node, at)),
        FilterNode::Any(nodes) => nodes.iter().any(|node| matches(db, task, node, at)),
        FilterNode::Not(inner) => !matches(db, task, inner, at),
        FilterNode::Condition(condition) => condition_matches(db, task, condition, at),
    }
}

pub fn condition_matches(db: &Db, task: &Task, condition: &Condition, at: DateTime<Utc>) -> bool {
    match condition {
        Condition::Project { id } => db.effective_project(&task.id).as_ref() == Some(id),
        Condition::Folder {
            id,
            include_children,
        } => db
            .effective_project(&task.id)
            .is_some_and(|project| db.project_is_in_folder(&project, id, *include_children)),
        Condition::Tag {
            id,
            include_children,
        } => {
            let mut wanted = vec![id.clone()];
            if *include_children {
                wanted.extend(db.tag_descendants(id));
            }
            let mut tagged = task.tags.iter().cloned().collect::<Vec<_>>();
            if let Some(project_id) = &task.project {
                if let Some(project) = db.projects.get(project_id) {
                    tagged.extend(project.tags.iter().cloned());
                }
            }
            tagged.iter().any(|tag| wanted.contains(tag))
        }
        Condition::InInbox => db.effective_project(&task.id).is_none(),
        Condition::Flagged { value } => task.flagged == *value,
        Condition::Priority { value } => task.priority == Some(*value),
        Condition::Status { value } => status_matches(db, task, *value, at),
        Condition::Due { window } => window_matches(task.due, *window, at),
        Condition::Deferred { window } => window_matches(task.defer, *window, at),
        Condition::HasRepeat { value } => task.repeat.is_some() == *value,
        Condition::FullscreenReminder { value } => {
            db.wants_fullscreen_reminder(&task.id) == *value
        }
        Condition::Estimate {
            min_minutes,
            max_minutes,
        } => {
            let Some(estimate) = task.estimate_min else {
                return false;
            };
            min_minutes.is_none_or(|min| estimate >= min)
                && max_minutes.is_none_or(|max| estimate <= max)
        }
        Condition::Text { value } => {
            let needle = value.trim().to_lowercase();
            if needle.is_empty() {
                return true;
            }
            task.title.to_lowercase().contains(&needle)
                || task.notes.to_lowercase().contains(&needle)
        }
    }
}

fn status_matches(db: &Db, task: &Task, status: TaskStatus, at: DateTime<Utc>) -> bool {
    match status {
        TaskStatus::Any => true,
        TaskStatus::Completed => task.completed,
        TaskStatus::Available => availability::availability(db, &task.id, at).is_available(),
        TaskStatus::Blocked => {
            !task.completed && !availability::availability(db, &task.id, at).is_available()
        }
        TaskStatus::Deferred => {
            if task.is_deferred(at) {
                return true;
            }
            if let Some(project_id) = &task.project {
                if let Some(project) = db.projects.get(project_id) {
                    return project.defer.is_some_and(|stamp| stamp.at > at);
                }
            }
            matches!(
                availability::availability(db, &task.id, at),
                Availability::Deferred(_) | Availability::ProjectDeferred(_, _)
            )
        }
    }
}

fn window_matches(stamp: Option<Stamp>, window: DueWindow, at: DateTime<Utc>) -> bool {
    let today = local_date(at);
    let date = stamp.map(|stamp| stamp.local_date());
    match window {
        DueWindow::Overdue => date.is_some_and(|date| date < today),
        DueWindow::Today => date == Some(today),
        DueWindow::Tomorrow => date == Some(today + Duration::days(1)),
        DueWindow::Next7Days => {
            date.is_some_and(|date| date >= today && date <= today + Duration::days(7))
        }
        DueWindow::Next30Days => {
            date.is_some_and(|date| date >= today && date <= today + Duration::days(30))
        }
        DueWindow::NoDate => date.is_none(),
        DueWindow::On(target) => date == Some(target),
        DueWindow::Before(target) => date.is_some_and(|date| date < target),
        DueWindow::After(target) => date.is_some_and(|date| date > target),
    }
}

/// Parse the `--due` style shortcuts used by the CLI into a window.
pub fn window_from_keyword(value: &str, today: NaiveDate) -> Option<DueWindow> {
    match value.trim().to_lowercase().as_str() {
        "overdue" => Some(DueWindow::Overdue),
        "today" => Some(DueWindow::Today),
        "tomorrow" => Some(DueWindow::Tomorrow),
        "next7" | "next-7" | "week" => Some(DueWindow::Next7Days),
        "next30" | "next-30" | "month" => Some(DueWindow::Next30Days),
        "none" | "nodate" | "no-date" => Some(DueWindow::NoDate),
        "before" => Some(DueWindow::Before(today)),
        "after" => Some(DueWindow::After(today)),
        _ => None,
    }
}
