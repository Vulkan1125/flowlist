//! Ordering of query rows. All comparators are stable: when two rows compare
//! equal the incoming manual order is preserved.

use std::cmp::Ordering;

use crate::db::Db;
use crate::ids::ProjectId;
use crate::model::{Priority, SortBy};
use crate::query::TaskRow;

pub fn sort_rows(db: &Db, rows: &mut [TaskRow], sort: SortBy) {
    match sort {
        SortBy::Manual => {}
        SortBy::Due => rows.sort_by(|a, b| {
            let left = db.effective_due(&a.id).map(|s| s.at);
            let right = db.effective_due(&b.id).map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Defer => rows.sort_by(|a, b| {
            let left = db.effective_defer(&a.id).map(|s| s.at);
            let right = db.effective_defer(&b.id).map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Planned => rows.sort_by(|a, b| {
            let left = db.effective_planned(&a.id).map(|s| s.at);
            let right = db.effective_planned(&b.id).map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Name => rows.sort_by_cached_key(|row| {
            db.tasks
                .get(&row.id)
                .map(|task| task.title.to_lowercase())
                .unwrap_or_default()
        }),
        SortBy::Priority => rows.sort_by(|a, b| {
            let rank = |id: &crate::ids::TaskId| {
                db.tasks
                    .get(id)
                    .and_then(|task| task.priority)
                    .map(|priority| match priority {
                        Priority::High => 0u8,
                        Priority::Medium => 1,
                        Priority::Low => 2,
                    })
                    .unwrap_or(3)
            };
            rank(&a.id).cmp(&rank(&b.id))
        }),
        SortBy::Flagged => rows.sort_by(|a, b| {
            let left = db.tasks.get(&a.id).is_some_and(|task| task.flagged);
            let right = db.tasks.get(&b.id).is_some_and(|task| task.flagged);
            right.cmp(&left)
        }),
        SortBy::Estimate => rows.sort_by(|a, b| {
            let left = db
                .tasks
                .get(&a.id)
                .and_then(|task| task.estimate_min)
                .unwrap_or(u32::MAX);
            let right = db
                .tasks
                .get(&b.id)
                .and_then(|task| task.estimate_min)
                .unwrap_or(u32::MAX);
            left.cmp(&right)
        }),
        SortBy::Created => rows.sort_by(|a, b| {
            let left = db.tasks.get(&a.id).map(|task| task.created_at);
            let right = db.tasks.get(&b.id).map(|task| task.created_at);
            option_cmp(left, right)
        }),
        SortBy::CompletedAt => rows.sort_by(|a, b| {
            let left = db.tasks.get(&a.id).and_then(|task| task.completed_at);
            let right = db.tasks.get(&b.id).and_then(|task| task.completed_at);
            option_cmp(right, left)
        }),
        SortBy::DueFlagged => rows.sort_by(|a, b| {
            let rank = |id: &crate::ids::TaskId| {
                let task = db.tasks.get(id);
                let flagged = task.is_some_and(|task| task.flagged);
                let due = db.effective_due(id);
                match (flagged, due.is_some()) {
                    (true, true) => 0u8,
                    (true, false) => 1,
                    (false, true) => 2,
                    (false, false) => 3,
                }
            };
            rank(&a.id)
                .cmp(&rank(&b.id))
                .then_with(|| {
                    option_cmp(
                        db.effective_due(&a.id).map(|stamp| stamp.at),
                        db.effective_due(&b.id).map(|stamp| stamp.at),
                    )
                })
        }),
        SortBy::Tag => rows.sort_by_cached_key(|row| {
            let mut names: Vec<String> = db
                .tasks
                .get(&row.id)
                .map(|task| crate::query::perspective::effective_tags(db, task))
                .unwrap_or_default()
                .into_iter()
                .filter_map(|id| db.tags.get(&id).map(|tag| tag.name.to_lowercase()))
                .collect();
            names.sort();
            names.join(" · ")
        }),
        SortBy::AddedDate => rows.sort_by(|a, b| {
            let left = db.tasks.get(&a.id).map(|task| task.created_at);
            let right = db.tasks.get(&b.id).map(|task| task.created_at);
            option_cmp(right, left)
        }),
        SortBy::ChangedDate => rows.sort_by(|a, b| {
            let left = db.tasks.get(&a.id).map(|task| task.modified_at);
            let right = db.tasks.get(&b.id).map(|task| task.modified_at);
            option_cmp(right, left)
        }),
        SortBy::DroppedDate => rows.sort_by(|a, b| {
            let dropped = |row: &TaskRow| {
                db.tasks.get(&row.id).and_then(|task| task.trashed_at).or_else(|| {
                    row.project
                        .as_ref()
                        .and_then(|id| db.projects.get(id))
                        .filter(|project| project.status == crate::model::ProjectStatus::Dropped)
                        .and_then(|project| project.dropped_at)
                })
            };
            option_cmp(dropped(b), dropped(a))
        }),
        SortBy::NextReview => rows.sort_by(|a, b| {
            let left = db
                .effective_project(&a.id)
                .and_then(|id| db.projects.get(&id))
                .map(|project| project.next_review);
            let right = db
                .effective_project(&b.id)
                .and_then(|id| db.projects.get(&id))
                .map(|project| project.next_review);
            option_cmp(left, right)
        }),
    }
}

/// Order project rows. Task-only sort keys fall back to library order.
pub fn sort_projects(db: &Db, projects: &mut [ProjectId], sort: SortBy) {
    match sort {
        SortBy::Manual => {}
        SortBy::Name => projects.sort_by_cached_key(|id| {
            db.projects
                .get(id)
                .map(|project| project.name.to_lowercase())
                .unwrap_or_default()
        }),
        SortBy::Due => projects.sort_by(|a, b| {
            let left = db.projects.get(a).and_then(|project| project.due).map(|s| s.at);
            let right = db.projects.get(b).and_then(|project| project.due).map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Defer => projects.sort_by(|a, b| {
            let left = db.projects.get(a).and_then(|project| project.defer).map(|s| s.at);
            let right = db.projects.get(b).and_then(|project| project.defer).map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Planned => projects.sort_by(|a, b| {
            let left = db
                .projects
                .get(a)
                .and_then(|project| project.planned)
                .map(|s| s.at);
            let right = db
                .projects
                .get(b)
                .and_then(|project| project.planned)
                .map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Created => projects.sort_by(|a, b| {
            let left = db.projects.get(a).map(|project| project.created_at);
            let right = db.projects.get(b).map(|project| project.created_at);
            option_cmp(left, right)
        }),
        SortBy::AddedDate => projects.sort_by(|a, b| {
            let left = db.projects.get(a).map(|project| project.created_at);
            let right = db.projects.get(b).map(|project| project.created_at);
            option_cmp(right, left)
        }),
        SortBy::ChangedDate => projects.sort_by(|a, b| {
            let left = db.projects.get(a).map(|project| project.modified_at);
            let right = db.projects.get(b).map(|project| project.modified_at);
            option_cmp(right, left)
        }),
        SortBy::DroppedDate => projects.sort_by(|a, b| {
            let left = db.projects.get(a).and_then(|project| project.dropped_at);
            let right = db.projects.get(b).and_then(|project| project.dropped_at);
            option_cmp(right, left)
        }),
        SortBy::NextReview => projects.sort_by(|a, b| {
            let left = db.projects.get(a).map(|project| project.next_review);
            let right = db.projects.get(b).map(|project| project.next_review);
            option_cmp(left, right)
        }),
        SortBy::Priority
        | SortBy::Flagged
        | SortBy::Estimate
        | SortBy::CompletedAt
        | SortBy::DueFlagged
        | SortBy::Tag => {}
    }
}

fn option_cmp<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
