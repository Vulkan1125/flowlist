//! Query engine shared by the GUI, CLI and MCP server.
//!
//! A query produces `Group`s of `TaskRow`s. Rows keep their outline depth so
//! the GUI can indent subtasks, and carry a computed `Availability` so every
//! client agrees on what "available" means.

pub mod availability;
pub mod forecast;
pub mod perspective;
pub mod sort;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::collections::BTreeMap;

use crate::db::Db;
use crate::ids::{ContainerId, FolderId, ProjectId, TagId, TaskId};
use crate::model::{
    BuiltinPerspective, Condition, FilterNode, GroupBy, Perspective, ProjectStatus, SortBy,
};
use crate::time::{Stamp, human_date};

pub use availability::Availability;

#[derive(Clone, Debug)]
pub struct TaskRow {
    pub id: TaskId,
    pub project: Option<ProjectId>,
    /// Outline depth inside its project (or the inbox).
    pub depth: u32,
    pub availability: Availability,
}

#[derive(Clone, Debug)]
pub struct Group {
    /// Stable identity used by the GUI for expansion state.
    pub key: String,
    pub title: String,
    /// Ordering hint; groups are sorted by this, then title.
    pub position: i64,
    pub rows: Vec<TaskRow>,
    /// Projects whose own date falls on this group's day. Forecast paints
    /// these above the task rows; every other view leaves the list empty.
    pub projects: Vec<ProjectId>,
}

/// Every task in manual (outline) order: projects in library order, then the
/// inbox. This is the canonical ordering behind `SortBy::Manual`.
pub fn manual_order(db: &Db) -> Vec<TaskId> {
    let mut out = Vec::with_capacity(db.tasks.len());
    for row in db.flatten_containers() {
        if let ContainerId::Project(id) = row.container {
            for task_row in db.project_outline(&id, true) {
                out.push(task_row.task);
            }
        }
    }
    for task_row in db.inbox_outline(true) {
        out.push(task_row.task);
    }
    out
}

pub fn manual_rank(db: &Db) -> BTreeMap<TaskId, usize> {
    manual_order(db)
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect()
}

pub fn row_for(db: &Db, id: &TaskId, at: DateTime<Utc>) -> TaskRow {
    TaskRow {
        id: id.clone(),
        project: db.effective_project(id),
        depth: db.task_depth(id),
        availability: availability::availability(db, id, at),
    }
}

pub fn rows_for(db: &Db, ids: &[TaskId], at: DateTime<Utc>) -> Vec<TaskRow> {
    ids.iter().map(|id| row_for(db, id, at)).collect()
}

/// Filter, sort and group the whole library.
pub fn run(
    db: &Db,
    filter: &FilterNode,
    sort: SortBy,
    group_by: GroupBy,
    show_completed: bool,
    at: DateTime<Utc>,
) -> Vec<Group> {
    let mut rows: Vec<TaskRow> = Vec::new();
    for id in manual_order(db) {
        let Some(task) = db.tasks.get(&id) else {
            continue;
        };
        if task.is_trashed() {
            continue;
        }
        if !show_completed && task.completed {
            continue;
        }
        if !perspective::matches(db, task, filter, at) {
            continue;
        }
        rows.push(row_for(db, &id, at));
    }
    sort::sort_rows(db, &mut rows, sort);
    let projects: Vec<ProjectId> = db
        .flatten_containers()
        .into_iter()
        .filter_map(|row| match row.container {
            ContainerId::Project(id) => Some(id),
            ContainerId::Folder(_) => None,
        })
        .filter(|id| {
            let Some(project) = db.projects.get(id) else {
                return false;
            };
            if !show_completed && project.status.is_closed() {
                return false;
            }
            perspective::matches_project(db, project, filter, at)
        })
        .collect();
    let mut groups = group_rows(db, rows, group_by, at);
    if !projects.is_empty() {
        groups.insert(
            0,
            Group {
                projects,
                key: "projects".to_string(),
                title: "Projects".to_string(),
                position: i64::MIN,
                rows: Vec::new(),
            },
        );
    }
    groups
}

pub fn custom(db: &Db, perspective: &Perspective, at: DateTime<Utc>) -> Vec<Group> {
    run(
        db,
        &perspective.filter,
        perspective.sort,
        perspective.group_by,
        perspective.show_completed,
        at,
    )
}

/// Finished and dropped projects, in library order. The GUI's Archive lists
/// these as projects, grouped by folder; the CLI and MCP still answer in
/// tasks, one group per archived project.
pub fn archived_projects(db: &Db) -> Vec<ProjectId> {
    db.flatten_containers()
        .into_iter()
        .filter_map(|row| match row.container {
            ContainerId::Project(id) => Some(id),
            ContainerId::Folder(_) => None,
        })
        .filter(|id| {
            db.projects
                .get(id)
                .is_some_and(|project| project.status.is_closed())
        })
        .collect()
}

/// The eight built-in perspectives.
pub fn view(db: &Db, which: BuiltinPerspective, at: DateTime<Utc>) -> Vec<Group> {
    match which {
        BuiltinPerspective::Inbox => {
            let rows = db
                .inbox_unscheduled_outline(false)
                .into_iter()
                .map(|row| row_for(db, &row.task, at))
                .collect();
            vec![Group {
                projects: Vec::new(),
                key: "inbox".to_string(),
                title: "Inbox".to_string(),
                position: 0,
                rows,
            }]
        }
        BuiltinPerspective::Today => today(db, true, at),
        BuiltinPerspective::Projects => {
            let mut groups = Vec::new();
            for (index, row) in db.flatten_containers().into_iter().enumerate() {
                let ContainerId::Project(id) = row.container else {
                    continue;
                };
                let Some(project) = db.projects.get(&id) else {
                    continue;
                };
                if !project.status.is_open() {
                    continue;
                }
                let rows = db
                    .project_outline(&id, false)
                    .into_iter()
                    .map(|task_row| row_for(db, &task_row.task, at))
                    .collect();
                groups.push(Group {
                    projects: Vec::new(),
                    key: format!("project:{id}"),
                    title: db.project_path(&id),
                    position: index as i64,
                    rows,
                });
            }
            groups
        }
        BuiltinPerspective::Forecast => forecast_groups(db, at, true, false),
        BuiltinPerspective::Flagged => run(
            db,
            &FilterNode::condition(Condition::Flagged { value: true }),
            SortBy::Due,
            GroupBy::Project,
            false,
            at,
        ),
        BuiltinPerspective::Review => Vec::new(),
        BuiltinPerspective::Tags => {
            let mut groups = Vec::new();
            for (index, row) in db.flatten_tags().into_iter().enumerate() {
                let title = db.tag_path(&row.tag);
                let ids: Vec<TaskId> = db
                    .tasks_tagged(&row.tag, true)
                    .into_iter()
                    .filter(|id| db.tasks.get(id).is_some_and(|task| !task.completed))
                    .collect();
                if ids.is_empty() {
                    continue;
                }
                groups.push(Group {
                    projects: Vec::new(),
                    key: format!("tag:{}", row.tag),
                    title,
                    position: index as i64,
                    rows: rows_for(db, &ids, at),
                });
            }
            groups
        }
        BuiltinPerspective::Completed => {
            let cutoff_days = db.settings.show_completed_days.max(1) as i64;
            let cutoff = at - Duration::days(cutoff_days);
            let mut rows: Vec<TaskRow> = db
                .tasks
                .values()
                .filter(|task| {
                    task.completed
                        && !task.is_trashed()
                        && task
                            .completed_at
                            .is_some_and(|completed| completed >= cutoff)
                })
                .map(|task| row_for(db, &task.id, at))
                .collect();
            sort::sort_rows(db, &mut rows, SortBy::CompletedAt);
            vec![Group {
                projects: Vec::new(),
                key: "completed".to_string(),
                title: "Completed".to_string(),
                position: 0,
                rows,
            }]
        }
        BuiltinPerspective::Archive => {
            // A project index in the GUI; tasks here keep `list` and MCP
            // useful for clients that only speak in rows.
            let mut groups = Vec::new();
            for (index, id) in archived_projects(db).into_iter().enumerate() {
                let rows = db
                    .project_outline(&id, true)
                    .into_iter()
                    .map(|task_row| row_for(db, &task_row.task, at))
                    .collect();
                groups.push(Group {
                    projects: Vec::new(),
                    key: format!("archive:{id}"),
                    title: db.project_path(&id),
                    position: index as i64,
                    rows,
                });
            }
            groups
        }
        BuiltinPerspective::Fullscreen => {
            let filter = FilterNode::condition(Condition::FullscreenReminder { value: true });
            let mut groups = run(db, &filter, SortBy::Due, GroupBy::None, false, at);
            for group in &mut groups {
                group.rows.retain(|row| db.reminder_stamp(&row.id).is_some());
            }
            groups.retain(|group| !group.rows.is_empty());
            groups
        }
        BuiltinPerspective::Nearby => {
            let hits = nearby(db, at);
            let ids: Vec<TaskId> = hits.into_iter().map(|hit| hit.task).collect();
            vec![Group {
                projects: Vec::new(),
                key: "nearby".to_string(),
                title: "Nearby".to_string(),
                position: 0,
                rows: rows_for(db, &ids, at),
            }]
        }
        BuiltinPerspective::Changed => {
            let days = db.settings.changed_days.max(1) as i64;
            let cutoff = at - chrono::Duration::days(days);
            let mut ids: Vec<TaskId> = db
                .tasks
                .values()
                .filter(|task| !task.completed && !task.is_trashed() && task.modified_at >= cutoff)
                .map(|task| task.id.clone())
                .collect();
            ids.sort_by_key(|id| std::cmp::Reverse(db.tasks[id].modified_at));
            vec![Group {
                projects: Vec::new(),
                key: "changed".to_string(),
                title: format!("Changed in the last {days} day(s)"),
                position: 0,
                rows: rows_for(db, &ids, at),
            }]
        }
        BuiltinPerspective::Attachments => {
            let ids: Vec<TaskId> = manual_order(db)
                .into_iter()
                .filter(|id| {
                    db.tasks.get(id).is_some_and(|task| {
                        !task.completed && !task.is_trashed() && !task.attachments.is_empty()
                    })
                })
                .collect();
            vec![Group {
                projects: Vec::new(),
                key: "attachments".to_string(),
                title: "Tasks with attachments".to_string(),
                position: 0,
                rows: rows_for(db, &ids, at),
            }]
        }
        BuiltinPerspective::Trash => {
            let ids = db.trashed_roots();
            vec![Group {
                projects: Vec::new(),
                key: "trash".to_string(),
                title: "Trash".to_string(),
                position: 0,
                rows: rows_for(db, &ids, at),
            }]
        }
    }
}

/// Tasks that want attention today: due today, planned today, deferred to
/// today or earlier, and — unless hidden — everything with an overdue due
/// date or a planned date that has slipped. Completed tasks, trashed tasks and
/// tasks in closed projects are left out, matching Forecast.
pub fn today(db: &Db, include_past: bool, at: DateTime<Utc>) -> Vec<Group> {
    let today = crate::time::local_date(at);
    let mut overdue: Vec<TaskId> = Vec::new();
    let mut due_today: Vec<TaskId> = Vec::new();
    let mut deferred: Vec<TaskId> = Vec::new();
    let mut planned_today: Vec<TaskId> = Vec::new();
    let mut past_planned: Vec<TaskId> = Vec::new();

    for id in manual_order(db) {
        let Some(task) = db.tasks.get(&id) else {
            continue;
        };
        if task.completed || task.is_trashed() {
            continue;
        }
        if let Some(project_id) = &task.project {
            match db.projects.get(project_id).map(|project| project.status) {
                Some(ProjectStatus::Active) | None => {}
                _ => continue,
            }
        }
        // Planned today counts even when a later due date exists; the retains
        // below drop anything already shown in another bucket. A plan that has
        // slipped is kept in its own group rather than rolled forward.
        if let Some(planned) = db.effective_planned(&id) {
            let date = planned.local_date();
            if date == today {
                planned_today.push(id.clone());
            } else if date < today {
                past_planned.push(id.clone());
            }
        }
        let due = db.effective_due(&id);
        let defer = db.effective_defer(&id);
        match due {
            Some(due) if due.local_date() < today => overdue.push(id),
            Some(due) if due.local_date() == today => due_today.push(id),
            Some(_) => {}
            // A task deferred until today (or earlier) has come due even
            // without a deadline, so it belongs in the day's list.
            None if defer.is_some_and(|stamp| stamp.local_date() <= today) => {
                deferred.push(id)
            }
            None => {}
        }
    }

    overdue.sort_by_key(|id| db.effective_due(id).map(|stamp| stamp.at));
    past_planned.retain(|id| {
        !overdue.contains(id)
            && !due_today.contains(id)
            && !deferred.contains(id)
            && !planned_today.contains(id)
    });
    past_planned.sort_by_key(|id| db.effective_planned(id).map(|stamp| stamp.at));
    planned_today
        .retain(|id| !overdue.contains(id) && !due_today.contains(id) && !deferred.contains(id));
    due_today.extend(planned_today);
    due_today.sort_by_cached_key(|id| {
        (
            db.effective_due(id)
                .or_else(|| db.effective_planned(id))
                .map(|stamp| stamp.at),
            db.tasks[id].title.to_lowercase(),
        )
    });
    deferred.sort_by_key(|id| db.effective_defer(id).map(|stamp| stamp.at));
    due_today.extend(deferred);

    let mut groups = Vec::new();
    if include_past && !past_planned.is_empty() {
        groups.push(Group {
            projects: Vec::new(),
            key: "today:past".to_string(),
            title: "Past plans".to_string(),
            position: -2,
            rows: rows_for(db, &past_planned, at),
        });
    }
    if include_past && !overdue.is_empty() {
        groups.push(Group {
            projects: Vec::new(),
            key: "today:overdue".to_string(),
            title: "Overdue".to_string(),
            position: -1,
            rows: rows_for(db, &overdue, at),
        });
    }
    if !due_today.is_empty() {
        groups.push(Group {
            projects: Vec::new(),
            key: "today".to_string(),
            title: "Today".to_string(),
            position: 0,
            rows: rows_for(db, &due_today, at),
        });
    }
    groups
}

/// Tasks with a given priority, in manual order. Completed and trashed tasks
/// are left out; the GUI's priority rows use this for their own view.
pub fn priority(db: &Db, priority: crate::model::Priority, at: DateTime<Utc>) -> Vec<Group> {
    let ids: Vec<TaskId> = manual_order(db)
        .into_iter()
        .filter(|id| {
            db.tasks.get(id).is_some_and(|task| {
                !task.completed && !task.is_trashed() && task.priority == Some(priority)
            })
        })
        .collect();
    vec![Group {
        projects: Vec::new(),
        key: format!("priority:{}", priority.slug()),
        title: format!("{} priority", priority.label()),
        position: 0,
        rows: rows_for(db, &ids, at),
    }]
}

/// Tasks due, planned or deferred on a specific local date. Used when a
/// Forecast calendar date is clicked.
pub fn tasks_on_date(
    db: &Db,
    date: chrono::NaiveDate,
    _at: DateTime<Utc>,
    include_planned: bool,
    due_only: bool,
) -> Vec<TaskId> {
    let mut ids: Vec<TaskId> = manual_order(db)
        .into_iter()
        .filter(|id| {
            let Some(task) = db.tasks.get(id) else {
                return false;
            };
            if task.completed || task.is_trashed() {
                return false;
            }
            if let Some(project_id) = &task.project {
                match db.projects.get(project_id).map(|project| project.status) {
                    Some(ProjectStatus::Active) | None => {}
                    _ => return false,
                }
            }
            db.effective_due(id).is_some_and(|stamp| stamp.local_date() == date)
                || (!due_only
                    && (db
                        .effective_defer(id)
                        .is_some_and(|stamp| stamp.local_date() == date)
                        || (include_planned
                            && db
                                .effective_planned(id)
                                .is_some_and(|stamp| stamp.local_date() == date))))
        })
        .collect();
    ids.sort_by_cached_key(|id| {
        let task = &db.tasks[id];
        (
            db.effective_due(id)
                .or_else(|| db.effective_planned(id))
                .or_else(|| db.effective_defer(id))
                .map(|stamp| stamp.at),
            task.title.to_lowercase(),
        )
    });
    ids
}

/// The Forecast list. `show_planned` mirrors the Forecast header's Planned
/// toggle; the calendar and day lists use the same flag.
pub fn forecast_view(
    db: &Db,
    at: DateTime<Utc>,
    show_planned: bool,
    due_only: bool,
) -> Vec<Group> {
    forecast_groups(db, at, show_planned, due_only)
}

/// Everything before today: overdue due dates and, when planned dates are
/// shown, slipped plans. Deferred dates are deliberately excluded, matching
/// Forecast's Past tile.
pub fn forecast_past(
    db: &Db,
    at: DateTime<Utc>,
    show_planned: bool,
    due_only: bool,
) -> Vec<TaskId> {
    let today = crate::time::local_date(at);
    let mut ids: Vec<TaskId> = db
        .tasks
        .values()
        .filter(|task| {
            if task.completed || task.is_trashed() {
                return false;
            }
            if let Some(project_id) = &task.project {
                match db.projects.get(project_id).map(|project| project.status) {
                    Some(ProjectStatus::Active) | None => {}
                    _ => return false,
                }
            }
            db.effective_due(&task.id).is_some_and(|stamp| stamp.local_date() < today)
                || (!due_only
                    && show_planned
                    && db
                        .effective_planned(&task.id)
                        .is_some_and(|stamp| stamp.local_date() < today))
        })
        .map(|task| task.id.clone())
        .collect();
    ids.sort_by_key(|id| {
        db.tasks[id]
            .due
            .or_else(|| db.effective_planned(id))
            .map(|stamp| stamp.at)
    });
    ids
}

/// Everything after the last visible Forecast day. Due, defer and (when
/// shown) inherited planned dates all count.
pub fn forecast_future(
    db: &Db,
    at: DateTime<Utc>,
    after: chrono::NaiveDate,
    show_planned: bool,
    due_only: bool,
) -> Vec<TaskId> {
    let mut ids: Vec<TaskId> = db
        .tasks
        .values()
        .filter(|task| {
            if task.completed || task.is_trashed() {
                return false;
            }
            if let Some(project_id) = &task.project {
                match db.projects.get(project_id).map(|project| project.status) {
                    Some(ProjectStatus::Active) | None => {}
                    _ => return false,
                }
            }
            db.effective_due(&task.id).is_some_and(|stamp| stamp.local_date() > after)
                || (!due_only
                    && (db
                        .effective_defer(&task.id)
                        .is_some_and(|stamp| stamp.local_date() > after)
                        || (show_planned
                            && db
                                .effective_planned(&task.id)
                                .is_some_and(|stamp| stamp.local_date() > after))))
        })
        .map(|task| task.id.clone())
        .collect();
    ids.sort_by_key(|id| {
        db.effective_due(id)
            .or_else(|| db.effective_planned(id))
            .or_else(|| db.effective_defer(id))
            .map(|stamp| stamp.at)
    });
    let _ = at;
    ids
}

/// Projects with their own date on one local day, deduped and ordered by
/// deadline then name. Used by a clicked Forecast day.
pub fn projects_on_date(
    db: &Db,
    date: NaiveDate,
    show_planned: bool,
    due_only: bool,
) -> Vec<ProjectId> {
    let mut ids: Vec<ProjectId> = db
        .projects
        .values()
        .filter(|project| project.status.is_open())
        .filter(|project| {
            project.due.is_some_and(|stamp| stamp.local_date() == date)
                || (!due_only
                    && (project.defer.is_some_and(|stamp| stamp.local_date() == date)
                        || (show_planned
                            && project
                                .planned
                                .is_some_and(|stamp| stamp.local_date() == date))))
        })
        .map(|project| project.id.clone())
        .collect();
    sort_project_ids(db, &mut ids);
    ids
}

/// Projects whose due date, or shown planned date, has slipped before today.
pub fn forecast_past_projects(
    db: &Db,
    at: DateTime<Utc>,
    show_planned: bool,
    due_only: bool,
) -> Vec<ProjectId> {
    let today = crate::time::local_date(at);
    let mut ids: Vec<ProjectId> = db
        .projects
        .values()
        .filter(|project| project.status.is_open())
        .filter(|project| {
            project.due.is_some_and(|stamp| stamp.local_date() < today)
                || (!due_only
                    && show_planned
                    && project
                        .planned
                        .is_some_and(|stamp| stamp.local_date() < today))
        })
        .map(|project| project.id.clone())
        .collect();
    sort_project_ids(db, &mut ids);
    ids
}

/// Projects with any due, defer or shown planned date after `after`.
pub fn forecast_future_projects(
    db: &Db,
    after: NaiveDate,
    show_planned: bool,
    due_only: bool,
) -> Vec<ProjectId> {
    let mut ids: Vec<ProjectId> = db
        .projects
        .values()
        .filter(|project| project.status.is_open())
        .filter(|project| {
            project.due.is_some_and(|stamp| stamp.local_date() > after)
                || (!due_only
                    && (project.defer.is_some_and(|stamp| stamp.local_date() > after)
                        || (show_planned
                            && project
                                .planned
                                .is_some_and(|stamp| stamp.local_date() > after))))
        })
        .map(|project| project.id.clone())
        .collect();
    sort_project_ids(db, &mut ids);
    ids
}

fn sort_project_ids(db: &Db, ids: &mut [ProjectId]) {
    ids.sort_by_cached_key(|id| {
        db.projects.get(id).map(|project| {
            (
                project
                    .due
                    .or(project.planned)
                    .or(project.defer)
                    .map(|stamp| stamp.at),
                project.name.to_lowercase(),
            )
        })
    });
}

fn forecast_groups(
    db: &Db,
    at: DateTime<Utc>,
    show_planned: bool,
    due_only: bool,
) -> Vec<Group> {
    let data = forecast::forecast(db, at, due_only);
    let today = crate::time::local_date(at);
    let tomorrow = today + Duration::days(1);
    let horizon = today + Duration::days(7);
    let month_horizon = today + Duration::days(30);
    let mut groups = Vec::new();
    let mut push = |key: &str,
                    title: &str,
                    position: i64,
                    ids: &[TaskId],
                    projects: &[ProjectId]| {
        if ids.is_empty() && projects.is_empty() {
            return;
        }
        let mut ordered = ids.to_vec();
        // A day's project rows sit above its tasks. Put tasks that belong to
        // one of those projects first, so the nearest task is not mistaken for
        // a child of the project above it.
        if !projects.is_empty() {
            ordered.sort_by_key(|id| {
                !db.tasks
                    .get(id)
                    .and_then(|task| task.project.as_ref())
                    .is_some_and(|project| projects.contains(project))
            });
        }
        groups.push(Group {
            projects: projects.to_vec(),
            key: key.to_string(),
            title: title.to_string(),
            position,
            rows: rows_for(db, &ordered, at),
        });
    };

    let mut project_past = Vec::new();
    let mut project_deferred_today = Vec::new();
    let mut project_today = Vec::new();
    let mut project_planned_today = Vec::new();
    let mut project_tomorrow = Vec::new();
    let mut project_days: BTreeMap<NaiveDate, Vec<ProjectId>> = BTreeMap::new();
    let mut project_later = Vec::new();
    let mut project_next_month = Vec::new();
    for (date, day) in &data.projects {
        if *date < today {
            project_past.extend(day.due.iter().cloned());
            if show_planned {
                project_past.extend(day.planned.iter().cloned());
            }
            continue;
        }
        if *date == today {
            project_today.extend(day.due.iter().cloned());
            project_deferred_today.extend(day.deferred.iter().cloned());
            if show_planned {
                project_planned_today.extend(day.planned.iter().cloned());
            }
        } else {
            let mut ids = day.due.clone();
            ids.extend(day.deferred.iter().cloned());
            if show_planned {
                ids.extend(day.planned.iter().cloned());
            }
            if *date == tomorrow {
                project_tomorrow.extend(ids);
            } else if *date <= horizon {
                project_days.entry(*date).or_default().extend(ids);
            } else if *date <= month_horizon {
                project_later.extend(ids);
            } else {
                project_next_month.extend(ids);
            }
        }
    }
    let project_key = |id: &ProjectId| {
        db.projects.get(id).map(|project| {
            (
                project
                    .due
                    .or(project.planned)
                    .or(project.defer)
                    .map(|stamp| stamp.at),
                project.name.to_lowercase(),
            )
        })
    };
    for ids in [
        &mut project_past,
        &mut project_deferred_today,
        &mut project_today,
        &mut project_planned_today,
        &mut project_tomorrow,
        &mut project_later,
        &mut project_next_month,
    ] {
        dedupe_projects(ids);
        ids.sort_by_key(project_key);
    }
    for ids in project_days.values_mut() {
        dedupe_projects(ids);
        ids.sort_by_key(project_key);
    }
    project_deferred_today.retain(|id| !project_today.contains(id));
    project_planned_today
        .retain(|id| !project_today.contains(id) && !project_deferred_today.contains(id));

    let mut past = data.overdue.clone();
    if show_planned {
        past.extend(data.past_planned.iter().cloned());
        dedupe(&mut past);
    }
    push(
        "forecast:flagged",
        "Flagged",
        -6,
        &data.flagged,
        &[],
    );
    let tag_title = db
        .settings
        .forecast_tag
        .as_ref()
        .map(|tag| db.tag_path(tag))
        .unwrap_or_else(|| "Forecast Tag".to_string());
    push("forecast:tag", &tag_title, -5, &data.tagged, &[]);
    push("forecast:past", "Past", -4, &past, &project_past);
    let mut deferred_today = data.deferred_today.clone();
    deferred_today.retain(|id| !data.today.contains(id));
    push(
        "forecast:deferred",
        "Deferred Today",
        -3,
        &deferred_today,
        &project_deferred_today,
    );
    push("forecast:today", "Today", -2, &data.today, &project_today);
    if show_planned {
        let mut planned_today = data.planned_today.clone();
        planned_today.retain(|id| !data.today.contains(id) && !deferred_today.contains(id));
        push(
            "forecast:planned-today",
            "Planned Today",
            -1,
            &planned_today,
            &project_planned_today,
        );
    }
    push(
        "forecast:tomorrow",
        "Tomorrow",
        0,
        &data.tomorrow,
        &project_tomorrow,
    );
    for (date, day) in &data.days {
        let mut ids = day.due.clone();
        ids.extend(day.deferred.iter().cloned());
        if show_planned {
            ids.extend(day.planned.iter().cloned());
        }
        dedupe(&mut ids);
        let position = (*date - today).num_days();
        let title = forecast::day_label(*date, today);
        let projects = project_days.get(date).map(Vec::as_slice).unwrap_or(&[]);
        push(
            &format!("forecast:{}", date.format("%Y-%m-%d")),
            &title,
            position,
            &ids,
            projects,
        );
    }
    let later: Vec<TaskId> = if show_planned {
        data.later.clone()
    } else {
        data.later
            .iter()
            .filter(|id| db.effective_due(id).is_some())
            .cloned()
            .collect()
    };
    push(
        "forecast:later",
        "Later this month",
        i64::MAX - 1,
        &later,
        &project_later,
    );
    let next_month: Vec<TaskId> = if show_planned {
        data.next_month.clone()
    } else {
        data.next_month
            .iter()
            .filter(|id| db.effective_due(id).is_some())
            .cloned()
            .collect()
    };
    push(
        "forecast:next-month",
        "Later",
        i64::MAX,
        &next_month,
        &project_next_month,
    );
    groups.sort_by(|a, b| a.position.cmp(&b.position));
    groups
}

fn dedupe_projects(ids: &mut Vec<ProjectId>) {
    let mut seen = std::collections::BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
}

/// Drop repeated ids while keeping the first occurrence's order. A task can
/// carry several dates on the same day; the outline should still show it once.
fn dedupe(ids: &mut Vec<TaskId>) {
    let mut seen = std::collections::BTreeSet::new();
    ids.retain(|id| seen.insert(id.clone()));
}

/// Projects that are due for review, soonest first.
pub fn review_projects(db: &Db, at: DateTime<Utc>) -> Vec<ProjectId> {
    let today = crate::time::local_date(at);
    let mut projects: Vec<ProjectId> = db
        .projects
        .values()
        .filter(|project| project.status.is_open() && project.next_review <= today)
        .map(|project| project.id.clone())
        .collect();
    projects.sort_by_key(|id| db.projects[id].next_review);
    projects
}

#[derive(Clone, Debug)]
pub struct NearbyHit {
    pub task: TaskId,
    pub distance_km: f64,
    pub via_tag: TagId,
}

/// Tasks tagged with a located context within the configured radius.
pub fn nearby(db: &Db, at: DateTime<Utc>) -> Vec<NearbyHit> {
    let Some(current) = db.settings.current_location else {
        return Vec::new();
    };
    let radius = db.settings.nearby_radius_km.max(0.1);
    let mut hits: Vec<NearbyHit> = Vec::new();
    for task in db.tasks.values() {
        if task.completed || task.is_trashed() {
            continue;
        }
        if let Some(project_id) = &task.project {
            match db.projects.get(project_id).map(|project| project.status) {
                Some(ProjectStatus::Active) | None => {}
                _ => continue,
            }
        }
        let mut best: Option<NearbyHit> = None;
        for tag_id in &task.tags {
            let Some(tag) = db.tags.get(tag_id) else {
                continue;
            };
            let Some(location) = tag.location else {
                continue;
            };
            let distance = current.distance_km(location);
            if distance > radius {
                continue;
            }
            if best.as_ref().is_none_or(|hit| distance < hit.distance_km) {
                best = Some(NearbyHit {
                    task: task.id.clone(),
                    distance_km: distance,
                    via_tag: tag_id.clone(),
                });
            }
        }
        if let Some(hit) = best {
            hits.push(hit);
        }
    }
    let _ = at;
    hits.sort_by(|a, b| a.distance_km.total_cmp(&b.distance_km));
    hits
}

struct GroupContext<'a> {
    db: &'a Db,
    at: DateTime<Utc>,
    containers: BTreeMap<ContainerId, i64>,
    folders: BTreeMap<FolderId, i64>,
    tags: BTreeMap<TagId, (i64, String)>,
}

impl<'a> GroupContext<'a> {
    fn new(db: &'a Db, at: DateTime<Utc>) -> Self {
        let containers = db
            .flatten_containers()
            .into_iter()
            .enumerate()
            .map(|(index, row)| (row.container, index as i64))
            .collect();
        let folders = db
            .flatten_containers()
            .into_iter()
            .filter_map(|row| match row.container {
                ContainerId::Folder(id) => {
                    let index = db
                        .flatten_containers()
                        .iter()
                        .position(|candidate| {
                            candidate.container == ContainerId::Folder(id.clone())
                        })
                        .unwrap_or(0) as i64;
                    Some((id, index))
                }
                ContainerId::Project(_) => None,
            })
            .collect();
        let tags = db
            .flatten_tags()
            .into_iter()
            .enumerate()
            .map(|(index, row)| (row.tag.clone(), (index as i64, db.tag_path(&row.tag))))
            .collect();
        Self {
            db,
            at,
            containers,
            folders,
            tags,
        }
    }

    fn group_key(&self, row: &TaskRow, group_by: GroupBy) -> (String, String, i64) {
        let db = self.db;
        let Some(task) = db.tasks.get(&row.id) else {
            return ("all".to_string(), "All".to_string(), 0);
        };
        match group_by {
            GroupBy::None => ("all".to_string(), "All".to_string(), 0),
            GroupBy::Project => match row.project.clone() {
                Some(id) => {
                    let position = self
                        .containers
                        .get(&ContainerId::Project(id.clone()))
                        .copied()
                        .unwrap_or(i64::MAX);
                    (format!("project:{id}"), db.project_path(&id), position)
                }
                None => ("inbox".to_string(), "No project".to_string(), -1),
            },
            GroupBy::Folder => {
                let folder = row
                    .project
                    .as_ref()
                    .and_then(|id| db.projects.get(id))
                    .and_then(|project| project.folder.clone());
                match folder {
                    Some(id) => (
                        format!("folder:{id}"),
                        db.folder_path(&id),
                        self.folders.get(&id).copied().unwrap_or(i64::MAX - 1),
                    ),
                    None => (
                        "no-folder".to_string(),
                        "No Folder".to_string(),
                        i64::MAX - 1,
                    ),
                }
            }
            GroupBy::Tag => {
                let candidates: Vec<TagId> =
                    perspective::effective_tags(db, task).into_iter().collect();
                let best = candidates
                    .into_iter()
                    .filter_map(|id| {
                        self.tags
                            .get(&id)
                            .map(|(position, path)| (id, *position, path.clone()))
                    })
                    .min_by_key(|(_, position, _)| *position);
                match best {
                    Some((id, position, path)) => (format!("tag:{id}"), path, position),
                    None => ("untagged".to_string(), "No Tag".to_string(), i64::MAX),
                }
            }
            GroupBy::DueDate => match db.effective_due(&task.id) {
                Some(stamp) => {
                    let today = crate::time::local_date(self.at);
                    let date = stamp.local_date();
                    (
                        format!("due:{}", date.format("%Y-%m-%d")),
                        human_date(&Stamp::date_only(date), self.at),
                        (date - today).num_days(),
                    )
                }
                None => ("due:none".to_string(), "No Due Date".to_string(), i64::MAX),
            },
            GroupBy::DeferDate => match db.effective_defer(&task.id) {
                Some(stamp) => {
                    let today = crate::time::local_date(self.at);
                    let date = stamp.local_date();
                    (
                        format!("defer:{}", date.format("%Y-%m-%d")),
                        human_date(&Stamp::date_only(date), self.at),
                        (date - today).num_days(),
                    )
                }
                None => (
                    "defer:none".to_string(),
                    "No Defer Date".to_string(),
                    i64::MAX,
                ),
            },
            GroupBy::PlannedDate => match db.effective_planned(&task.id) {
                Some(stamp) => {
                    let today = crate::time::local_date(self.at);
                    let date = stamp.local_date();
                    (
                        format!("planned:{}", date.format("%Y-%m-%d")),
                        human_date(&Stamp::date_only(date), self.at),
                        (date - today).num_days(),
                    )
                }
                None => (
                    "planned:none".to_string(),
                    "No Planned Date".to_string(),
                    i64::MAX,
                ),
            },
            GroupBy::CompletedDate => {
                match task.completed_at.or_else(|| task.completed.then_some(task.modified_at)) {
                    Some(at) => {
                        let date = crate::time::local_date(at);
                        (
                            format!("completed:{}", date.format("%Y-%m-%d")),
                            human_date(&Stamp::date_only(date), self.at),
                            (crate::time::local_date(self.at) - date).num_days(),
                        )
                    }
                    None => (
                        "completed:none".to_string(),
                        "Not Completed".to_string(),
                        i64::MAX,
                    ),
                }
            }
            GroupBy::DroppedDate => {
                let dropped = task.trashed_at.or_else(|| {
                    row.project
                        .as_ref()
                        .and_then(|id| db.projects.get(id))
                        .filter(|project| project.status == crate::model::ProjectStatus::Dropped)
                        .and_then(|project| project.dropped_at)
                });
                match dropped {
                    Some(at) => {
                        let date = crate::time::local_date(at);
                        (
                            format!("dropped:{}", date.format("%Y-%m-%d")),
                            human_date(&Stamp::date_only(date), self.at),
                            (crate::time::local_date(self.at) - date).num_days(),
                        )
                    }
                    None => (
                        "dropped:none".to_string(),
                        "Not Dropped".to_string(),
                        i64::MIN,
                    ),
                }
            }
            GroupBy::AddedDate => {
                let date = crate::time::local_date(task.created_at);
                (
                    format!("added:{}", date.format("%Y-%m-%d")),
                    human_date(&Stamp::date_only(date), self.at),
                    (crate::time::local_date(self.at) - date).num_days(),
                )
            }
            GroupBy::ChangedDate => {
                let date = crate::time::local_date(task.modified_at);
                (
                    format!("changed:{}", date.format("%Y-%m-%d")),
                    human_date(&Stamp::date_only(date), self.at),
                    (crate::time::local_date(self.at) - date).num_days(),
                )
            }
            GroupBy::NextReview => {
                match row
                    .project
                    .as_ref()
                    .and_then(|id| db.projects.get(id))
                {
                    Some(project) => (
                        format!("review:{}", project.next_review.format("%Y-%m-%d")),
                        project.next_review.format("%Y-%m-%d").to_string(),
                        (project.next_review - crate::time::local_date(self.at)).num_days(),
                    ),
                    None => (
                        "review:none".to_string(),
                        "No Project".to_string(),
                        i64::MAX,
                    ),
                }
            }
            GroupBy::TagsCombined => {
                let candidates: Vec<TagId> =
                    perspective::effective_tags(db, task).into_iter().collect();
                let mut paths: Vec<(i64, String)> = candidates
                    .into_iter()
                    .filter_map(|id| self.tags.get(&id).cloned())
                    .collect();
                paths.sort_by_key(|(position, _)| *position);
                if paths.is_empty() {
                    ("untagged".to_string(), "No Tag".to_string(), i64::MAX)
                } else {
                    let title = paths
                        .iter()
                        .map(|(_, path)| path.clone())
                        .collect::<Vec<_>>()
                        .join(" · ");
                    let position = paths.first().map(|(position, _)| *position).unwrap_or(0);
                    (format!("tags:{title}"), title, position)
                }
            }
            GroupBy::Flagged => {
                if task.flagged {
                    ("flagged".to_string(), "Flagged".to_string(), 0)
                } else {
                    ("unflagged".to_string(), "Not Flagged".to_string(), 1)
                }
            }
            GroupBy::Estimate => {
                let (position, title) = match task.estimate_min {
                    None => (5, "No Estimate"),
                    Some(minutes) if minutes < 15 => (0, "Under 15m"),
                    Some(minutes) if minutes <= 30 => (1, "15–30m"),
                    Some(minutes) if minutes <= 60 => (2, "30m–1h"),
                    Some(minutes) if minutes <= 120 => (3, "1–2h"),
                    Some(_) => (4, "Over 2h"),
                };
                (format!("estimate:{position}"), title.to_string(), position)
            }
        }
    }
}

fn group_rows(db: &Db, rows: Vec<TaskRow>, group_by: GroupBy, at: DateTime<Utc>) -> Vec<Group> {
    let context = GroupContext::new(db, at);
    let mut groups: Vec<Group> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        let (key, title, position) = context.group_key(&row, group_by);
        if let Some(existing) = index.get(&key).copied() {
            groups[existing].rows.push(row);
        } else {
            index.insert(key.clone(), groups.len());
            groups.push(Group {
                projects: Vec::new(),
                key,
                title,
                position,
                rows: vec![row],
            });
        }
    }
    groups.sort_by(|a, b| {
        a.position
            .cmp(&b.position)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    groups
}

/// Re-group an explicit list of rows. Used by the GUI when the user changes
/// the grouping of a perspective without re-running its filter.
pub fn regroup(db: &Db, rows: Vec<TaskRow>, group_by: GroupBy, at: DateTime<Utc>) -> Vec<Group> {
    group_rows(db, rows, group_by, at)
}

/// Convenience for CLI commands that just need the matching task ids.
pub fn matching_ids(
    db: &Db,
    filter: &FilterNode,
    show_completed: bool,
    at: DateTime<Utc>,
) -> Vec<TaskId> {
    manual_order(db)
        .into_iter()
        .filter(|id| {
            db.tasks.get(id).is_some_and(|task| {
                (show_completed || !task.completed) && perspective::matches(db, task, filter, at)
            })
        })
        .collect()
}
