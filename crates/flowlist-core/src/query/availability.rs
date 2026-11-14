//! Task availability: the rules that decide whether a task can be worked on
//! right now (active project, defer date passed, not blocked by a context or
//! by an earlier task in a sequential list).

use chrono::{DateTime, Utc};

use crate::db::{Db, OrderKey};
use crate::ids::{ProjectId, TagId, TaskId};
use crate::model::{ProjectStatus, TagStatus, Task};
use crate::time::{Stamp, local_date};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    Available,
    Completed,
    ProjectOnHold(ProjectId),
    ProjectDone(ProjectId),
    ProjectDropped(ProjectId),
    Deferred(Stamp),
    BlockedByTag(TagId),
    BlockedBySequence(TaskId),
    ParentUnavailable(TaskId),
    ProjectDeferred(ProjectId, Stamp),
}

impl Availability {
    pub fn is_available(&self) -> bool {
        matches!(self, Availability::Available)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Availability::Completed
                | Availability::ProjectDone(_)
                | Availability::ProjectDropped(_)
        )
    }

    pub fn label(&self) -> &'static str {
        match self {
            Availability::Available => "Available",
            Availability::Completed => "Completed",
            Availability::ProjectOnHold(_) => "Project on hold",
            Availability::ProjectDone(_) => "Project done",
            Availability::ProjectDropped(_) => "Project dropped",
            Availability::Deferred(_) => "Deferred",
            Availability::ProjectDeferred(_, _) => "Deferred by project",
            Availability::BlockedByTag(_) => "Blocked by tag",
            Availability::BlockedBySequence(_) => "Blocked by earlier task",
            Availability::ParentUnavailable(_) => "Blocked by parent",
        }
    }

    pub fn detail(&self, db: &Db) -> Option<String> {
        match self {
            Availability::BlockedByTag(id) => {
                let tag = db.tags.get(id)?;
                Some(format!(
                    "“{}” is {}",
                    tag.name,
                    tag.status.label().to_lowercase()
                ))
            }
            Availability::BlockedBySequence(id) => db
                .tasks
                .get(id)
                .map(|task| format!("Finish “{}” first", task.title)),
            Availability::Deferred(stamp) => {
                Some(format!("Deferred until {}", stamp.to_input_string()))
            }
            Availability::ProjectDeferred(_, stamp) => Some(format!(
                "Project deferred until {}",
                stamp.to_input_string()
            )),
            Availability::ParentUnavailable(id) => db
                .tasks
                .get(id)
                .map(|task| format!("Parent “{}” is not available", task.title)),
            Availability::ProjectOnHold(id) => db
                .projects
                .get(id)
                .map(|project| format!("Project “{}” is on hold", project.name)),
            _ => None,
        }
    }
}

pub fn availability(db: &Db, task_id: &TaskId, at: DateTime<Utc>) -> Availability {
    let Some(task) = db.tasks.get(task_id) else {
        return Availability::Completed;
    };
    availability_inner(db, task, at)
}

fn availability_inner(db: &Db, task: &Task, at: DateTime<Utc>) -> Availability {
    if task.completed {
        return Availability::Completed;
    }

    if let Some(parent_id) = &task.parent {
        let parent = match db.tasks.get(parent_id) {
            Some(parent) => parent,
            None => return Availability::ParentUnavailable(parent_id.clone()),
        };
        if !availability_inner(db, parent, at).is_available() {
            return Availability::ParentUnavailable(parent_id.clone());
        }
    }

    if let Some(project_id) = &task.project {
        let Some(project) = db.projects.get(project_id) else {
            return Availability::ProjectDropped(project_id.clone());
        };
        match project.status {
            ProjectStatus::Active => {}
            ProjectStatus::OnHold => return Availability::ProjectOnHold(project_id.clone()),
            ProjectStatus::Done => return Availability::ProjectDone(project_id.clone()),
            ProjectStatus::Dropped => return Availability::ProjectDropped(project_id.clone()),
        }
        if let Some(defer) = project.defer {
            if defer.at > at {
                return Availability::ProjectDeferred(project_id.clone(), defer);
            }
        }
    }

    if let Some(defer) = task.defer {
        if defer.at > at {
            return Availability::Deferred(defer);
        }
    }

    let mut tags: Vec<&TagId> = task.tags.iter().collect();
    if let Some(project_id) = &task.project {
        if let Some(project) = db.projects.get(project_id) {
            tags.extend(project.tags.iter());
        }
    }
    for tag_id in tags {
        if let Some(tag) = db.tags.get(tag_id) {
            let inherited_blocked = tag
                .parent
                .as_ref()
                .is_some_and(|parent| !db.tag_is_available(parent));
            if tag.status != TagStatus::Available || inherited_blocked {
                return Availability::BlockedByTag(tag_id.clone());
            }
        }
    }

    if let Some(blocker) = sequence_blocker(db, task) {
        return Availability::BlockedBySequence(blocker);
    }

    Availability::Available
}

/// In a sequential list, every earlier incomplete sibling blocks this task.
fn sequence_blocker(db: &Db, task: &Task) -> Option<TaskId> {
    let (key, sequential) = if let Some(parent_id) = &task.parent {
        let parent = db.tasks.get(parent_id)?;
        (OrderKey::Parent(parent_id.clone()), parent.sequential)
    } else if let Some(project_id) = &task.project {
        let project = db.projects.get(project_id)?;
        (
            OrderKey::Project(project_id.clone()),
            project.is_sequential(),
        )
    } else {
        return None;
    };
    if !sequential {
        return None;
    }
    for sibling in db.ordered_tasks(key) {
        if sibling == &task.id {
            break;
        }
        if let Some(sibling) = db.tasks.get(sibling) {
            if !sibling.completed {
                return Some(sibling.id.clone());
            }
        }
    }
    None
}

pub fn is_available(db: &Db, task_id: &TaskId, at: DateTime<Utc>) -> bool {
    availability(db, task_id, at).is_available()
}

pub fn is_due_on(db: &Db, task_id: &TaskId, date: chrono::NaiveDate, at: DateTime<Utc>) -> bool {
    db.tasks
        .get(task_id)
        .and_then(|task| task.due)
        .is_some_and(|due| due.local_date() == date)
        && !db.tasks[task_id].completed
        && local_date(at) == date
}

pub fn due_today(db: &Db, at: DateTime<Utc>) -> Vec<TaskId> {
    let today = local_date(at);
    let mut out: Vec<TaskId> = db
        .tasks
        .values()
        .filter(|task| !task.completed && task.due.is_some_and(|stamp| stamp.local_date() == today))
        .map(|task| task.id.clone())
        .collect();
    out.sort_by_key(|id| db.tasks[id].due.map(|stamp| stamp.at));
    out
}

pub fn overdue(db: &Db, at: DateTime<Utc>) -> Vec<TaskId> {
    let today = local_date(at);
    let mut out: Vec<TaskId> = db
        .tasks
        .values()
        .filter(|task| !task.completed && task.due.is_some_and(|stamp| stamp.local_date() < today))
        .map(|task| task.id.clone())
        .collect();
    out.sort_by_key(|id| db.tasks[id].due.map(|stamp| stamp.at));
    out
}
