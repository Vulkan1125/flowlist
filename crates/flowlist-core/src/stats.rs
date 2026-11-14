//! Aggregate counters for the status bar, dashboard and `flowlist stats`.

use chrono::{DateTime, Utc};

use crate::db::Db;
use crate::model::ProjectStatus;
use crate::query::availability::{self, Availability};
use crate::time::local_date;

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub total_tasks: usize,
    pub open_tasks: usize,
    pub completed_tasks: usize,
    pub inbox: usize,
    pub available: usize,
    pub blocked: usize,
    pub deferred: usize,
    pub flagged: usize,
    pub due_today: usize,
    pub overdue: usize,
    pub completed_today: usize,
    pub active_projects: usize,
    pub on_hold_projects: usize,
    pub projects_due_review: usize,
    pub tags: usize,
    pub folders: usize,
    pub trashed_tasks: usize,
}

pub fn compute(db: &Db, at: DateTime<Utc>) -> Stats {
    let today = local_date(at);
    let mut stats = Stats {
        total_tasks: db.tasks.values().filter(|task| !task.is_trashed()).count(),
        tags: db.tags.len(),
        folders: db.folders.len(),
        ..Stats::default()
    };

    for task in db.tasks.values() {
        if task.is_trashed() {
            stats.trashed_tasks += 1;
            continue;
        }
        if task.completed {
            stats.completed_tasks += 1;
            if task
                .completed_at
                .is_some_and(|completed| local_date(completed) == today)
            {
                stats.completed_today += 1;
            }
            continue;
        }
        stats.open_tasks += 1;
        if task.project.is_none() && task.parent.is_none() {
            stats.inbox += 1;
        }
        if task.flagged {
            stats.flagged += 1;
        }
        if let Some(due) = task.due {
            let date = due.local_date();
            if date < today {
                stats.overdue += 1;
            } else if date == today {
                stats.due_today += 1;
            }
        }
        match availability::availability(db, &task.id, at) {
            Availability::Available => stats.available += 1,
            Availability::Deferred(_) | Availability::ProjectDeferred(_, _) => stats.deferred += 1,
            Availability::Completed => {}
            _ => stats.blocked += 1,
        }
    }

    for project in db.projects.values() {
        match project.status {
            ProjectStatus::Active => {
                stats.active_projects += 1;
                if project.next_review <= today {
                    stats.projects_due_review += 1;
                }
            }
            ProjectStatus::OnHold => {
                stats.on_hold_projects += 1;
                if project.next_review <= today {
                    stats.projects_due_review += 1;
                }
            }
            _ => {}
        }
    }

    stats
}
