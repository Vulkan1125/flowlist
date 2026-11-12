//! Query engine shared by the GUI, CLI and MCP server.
//!
//! A query produces `Group`s of `TaskRow`s. Rows keep their outline depth so
//! the GUI can indent subtasks, and carry a computed `Availability` so every
//! client agrees on what "available" means.

pub mod availability;
pub mod forecast;
pub mod perspective;
pub mod sort;

use chrono::{DateTime, Duration, Utc};
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
        if !show_completed && task.completed {
            continue;
        }
        if !perspective::matches(db, task, filter, at) {
            continue;
        }
        rows.push(row_for(db, &id, at));
    }
    sort::sort_rows(db, &mut rows, sort);
    group_rows(db, rows, group_by, at)
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

/// The eight built-in perspectives.
pub fn view(db: &Db, which: BuiltinPerspective, at: DateTime<Utc>) -> Vec<Group> {
    match which {
        BuiltinPerspective::Inbox => {
            let rows = db
                .inbox_outline(false)
                .into_iter()
                .map(|row| row_for(db, &row.task, at))
                .collect();
            vec![Group {
                key: "inbox".to_string(),
                title: "Inbox".to_string(),
                position: 0,
                rows,
            }]
        }
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
                    key: format!("project:{id}"),
                    title: db.project_path(&id),
                    position: index as i64,
                    rows,
                });
            }
            groups
        }
        BuiltinPerspective::Forecast => forecast_groups(db, at),
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
                    .filter(|id| {
                        db.tasks
                            .get(id)
                            .is_some_and(|task| !task.completed)
                    })
                    .collect();
                if ids.is_empty() {
                    continue;
                }
                groups.push(Group {
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
                        && task
                            .completed_at
                            .is_some_and(|completed| completed >= cutoff)
                })
                .map(|task| row_for(db, &task.id, at))
                .collect();
            sort::sort_rows(db, &mut rows, SortBy::CompletedAt);
            vec![Group {
                key: "completed".to_string(),
                title: "Completed".to_string(),
                position: 0,
                rows,
            }]
        }
        BuiltinPerspective::Nearby => {
            let hits = nearby(db, at);
            let ids: Vec<TaskId> = hits.into_iter().map(|hit| hit.task).collect();
            vec![Group {
                key: "nearby".to_string(),
                title: "Nearby".to_string(),
                position: 0,
                rows: rows_for(db, &ids, at),
            }]
        }
    }
}

fn forecast_groups(db: &Db, at: DateTime<Utc>) -> Vec<Group> {
    let data = forecast::forecast(db, at);
    let today = crate::time::local_date(at);
    let mut groups = Vec::new();
    let mut push = |key: &str, title: &str, position: i64, ids: &[TaskId]| {
        if ids.is_empty() {
            return;
        }
        groups.push(Group {
            key: key.to_string(),
            title: title.to_string(),
            position,
            rows: rows_for(db, ids, at),
        });
    };

    push("forecast:overdue", "Overdue", -3, &data.overdue);
    push(
        "forecast:deferred",
        "Deferred Today",
        -2,
        &data.deferred_today,
    );
    push("forecast:today", "Today", -1, &data.today);
    push("forecast:tomorrow", "Tomorrow", 0, &data.tomorrow);
    for (date, day) in &data.days {
        let mut ids = day.due.clone();
        ids.extend(day.deferred.iter().cloned());
        let position = (*date - today).num_days();
        let title = forecast::day_label(*date, today);
        push(
            &format!("forecast:{}", date.format("%Y-%m-%d")),
            &title,
            position,
            &ids,
        );
    }
    push("forecast:later", "Later", i64::MAX - 1, &data.later);
    groups.sort_by(|a, b| a.position.cmp(&b.position));
    groups
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
        if task.completed {
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
            if best
                .as_ref()
                .is_none_or(|hit| distance < hit.distance_km)
            {
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
                        .position(|candidate| candidate.container == ContainerId::Folder(id.clone()))
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
                    (
                        format!("project:{id}"),
                        db.project_path(&id),
                        position,
                    )
                }
                None => ("inbox".to_string(), "Inbox".to_string(), -1),
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
                let mut candidates: Vec<TagId> = task.tags.iter().cloned().collect();
                if let Some(project_id) = &task.project {
                    if let Some(project) = db.projects.get(project_id) {
                        candidates.extend(project.tags.iter().cloned());
                    }
                }
                let best = candidates
                    .into_iter()
                    .filter_map(|id| self.tags.get(&id).map(|(position, path)| (id, *position, path.clone())))
                    .min_by_key(|(_, position, _)| *position);
                match best {
                    Some((id, position, path)) => (
                        format!("tag:{id}"),
                        path,
                        position,
                    ),
                    None => ("untagged".to_string(), "No Tag".to_string(), i64::MAX),
                }
            }
            GroupBy::DueDate => match task.due {
                Some(stamp) => {
                    let today = crate::time::local_date(self.at);
                    let date = stamp.local_date();
                    (
                        format!("due:{}", date.format("%Y-%m-%d")),
                        human_date(&Stamp::date_only(date), self.at),
                        (date - today).num_days(),
                    )
                }
                None => (
                    "due:none".to_string(),
                    "No Due Date".to_string(),
                    i64::MAX,
                ),
            },
            GroupBy::DeferDate => match task.defer {
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

fn group_rows(
    db: &Db,
    rows: Vec<TaskRow>,
    group_by: GroupBy,
    at: DateTime<Utc>,
) -> Vec<Group> {
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
