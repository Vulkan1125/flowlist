//! Ordering of query rows. All comparators are stable: when two rows compare
//! equal the incoming manual order is preserved.

use std::cmp::Ordering;

use crate::db::Db;
use crate::model::SortBy;
use crate::query::TaskRow;

pub fn sort_rows(db: &Db, rows: &mut [TaskRow], sort: SortBy) {
    match sort {
        SortBy::Manual => {}
        SortBy::Due => rows.sort_by(|a, b| {
            let left = db.tasks.get(&a.id).and_then(|task| task.due).map(|s| s.at);
            let right = db.tasks.get(&b.id).and_then(|task| task.due).map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Defer => rows.sort_by(|a, b| {
            let left = db
                .tasks
                .get(&a.id)
                .and_then(|task| task.defer)
                .map(|s| s.at);
            let right = db
                .tasks
                .get(&b.id)
                .and_then(|task| task.defer)
                .map(|s| s.at);
            option_cmp(left, right)
        }),
        SortBy::Name => rows.sort_by(|a, b| {
            let left = db
                .tasks
                .get(&a.id)
                .map(|task| task.title.to_lowercase())
                .unwrap_or_default();
            let right = db
                .tasks
                .get(&b.id)
                .map(|task| task.title.to_lowercase())
                .unwrap_or_default();
            left.cmp(&right)
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
