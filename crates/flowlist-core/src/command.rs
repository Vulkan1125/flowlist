//! Every mutation goes through a serializable `Command`. The store wraps each
//! command in a snapshot so undo/redo covers all operations, including tree
//! moves and bulk changes.

use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeSet;

use crate::db::{
    ContainerPosition, Db, OrderKey, Position,
};
use crate::error::{Error, Result};
use crate::ids::{ContainerId, FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use crate::model::{
    Folder, Geo, Perspective, Project, ProjectStatus, RepeatRule, Tag, TagStatus, Task,
};
use crate::time::{Stamp, local_date, now};

#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub label: String,
    pub created: Vec<String>,
    pub affected: Vec<String>,
    pub message: Option<String>,
}

impl Outcome {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewTask {
    pub title: String,
    pub notes: String,
    pub project: Option<ProjectId>,
    /// A `Folder/Project` path resolved (and created) while applying. Used by
    /// quick entry so filing a task is a single undoable command.
    pub project_path: Option<String>,
    pub parent: Option<TaskId>,
    pub tags: Vec<TagId>,
    /// `Parent/Child` tag paths resolved (and created) while applying.
    pub tag_paths: Vec<String>,
    pub due: Option<Stamp>,
    pub defer: Option<Stamp>,
    pub estimate_min: Option<u32>,
    pub flagged: bool,
    pub repeat: Option<RepeatRule>,
    pub sequential: bool,
    pub position: Position,
}

impl NewTask {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            notes: String::new(),
            project: None,
            project_path: None,
            parent: None,
            tags: Vec::new(),
            tag_paths: Vec::new(),
            due: None,
            defer: None,
            estimate_min: None,
            flagged: false,
            repeat: None,
            sequential: false,
            position: Position::End,
        }
    }

    pub fn in_project(title: impl Into<String>, project: ProjectId) -> Self {
        Self {
            project: Some(project),
            ..Self::new(title)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub project: Option<Option<ProjectId>>,
    pub parent: Option<Option<TaskId>>,
    pub due: Option<Option<Stamp>>,
    pub defer: Option<Option<Stamp>>,
    pub estimate_min: Option<Option<u32>>,
    pub flagged: Option<bool>,
    pub repeat: Option<Option<RepeatRule>>,
    pub sequential: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct NewProject {
    pub name: String,
    pub notes: String,
    pub folder: Option<FolderId>,
    pub tags: Vec<TagId>,
    pub due: Option<Stamp>,
    pub defer: Option<Stamp>,
    pub review_interval_days: Option<u32>,
    pub next_review: Option<NaiveDate>,
    pub sequential: bool,
    pub position: ContainerPosition,
}

impl NewProject {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            notes: String::new(),
            folder: None,
            tags: Vec::new(),
            due: None,
            defer: None,
            review_interval_days: None,
            next_review: None,
            sequential: false,
            position: ContainerPosition::End,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub notes: Option<String>,
    pub folder: Option<Option<FolderId>>,
    pub tags: Option<BTreeSet<TagId>>,
    pub due: Option<Option<Stamp>>,
    pub defer: Option<Option<Stamp>>,
    pub review_interval_days: Option<u32>,
    pub next_review: Option<NaiveDate>,
    pub sequential: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct TagPatch {
    pub name: Option<String>,
    pub parent: Option<Option<TagId>>,
    pub status: Option<TagStatus>,
    pub location: Option<Option<Geo>>,
}

#[derive(Clone, Debug, Default)]
pub struct SettingsPatch {
    pub default_review_days: Option<u32>,
    pub show_completed_days: Option<u32>,
    pub week_starts_on: Option<u8>,
    pub default_due_time: Option<Option<chrono::NaiveTime>>,
    pub default_defer_time: Option<Option<chrono::NaiveTime>>,
    pub current_location: Option<Option<Geo>>,
    pub nearby_radius_km: Option<f64>,
}

#[derive(Clone, Debug)]
pub enum TaskPlacement {
    Inbox(Position),
    Project {
        project: ProjectId,
        position: Position,
    },
    Parent {
        parent: TaskId,
        position: Position,
    },
}

#[derive(Clone, Debug)]
pub struct ContainerPlacement {
    pub parent: Option<FolderId>,
    pub position: ContainerPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportMode {
    Replace,
    Merge,
}

#[derive(Clone, Debug)]
pub enum Command {
    CreateTask(NewTask),
    UpdateTask { id: TaskId, patch: TaskPatch },
    CompleteTasks { ids: Vec<TaskId>, completed: bool },
    DeleteTasks { ids: Vec<TaskId> },
    MoveTasks { ids: Vec<TaskId>, placement: TaskPlacement },
    SetTaskTags { ids: Vec<TaskId>, add: Vec<TagId>, remove: Vec<TagId> },
    SetTaskFlag { ids: Vec<TaskId>, flagged: bool },
    CreateProject(NewProject),
    UpdateProject { id: ProjectId, patch: ProjectPatch },
    SetProjectStatus { id: ProjectId, status: ProjectStatus },
    CompleteReview { id: ProjectId, reviewed_on: NaiveDate },
    MoveProject { id: ProjectId, placement: ContainerPlacement },
    CreateFolder { name: String, parent: Option<FolderId>, position: ContainerPosition },
    RenameFolder { id: FolderId, name: String },
    MoveFolder { id: FolderId, placement: ContainerPlacement },
    DeleteFolder { id: FolderId },
    CreateTag {
        name: String,
        parent: Option<TagId>,
        status: TagStatus,
        location: Option<Geo>,
        position: Position,
    },
    UpdateTag { id: TagId, patch: TagPatch },
    DeleteTag { id: TagId },
    SavePerspective { perspective: Perspective },
    DeletePerspective { id: PerspectiveId },
    UpdateSettings(SettingsPatch),
    Import { db: Db, mode: ImportMode },
}

impl Command {
    pub fn label(&self) -> String {
        match self {
            Command::CreateTask(task) => format!("Add task “{}”", task.title),
            Command::UpdateTask { id, .. } => format!("Edit task {id}"),
            Command::CompleteTasks { ids, completed } => {
                let verb = if *completed { "Complete" } else { "Uncomplete" };
                if ids.len() == 1 {
                    format!("{verb} task")
                } else {
                    format!("{verb} {} tasks", ids.len())
                }
            }
            Command::DeleteTasks { ids } => format!("Delete {} task(s)", ids.len()),
            Command::MoveTasks { ids, .. } => format!("Move {} task(s)", ids.len()),
            Command::SetTaskTags { ids, .. } => format!("Tag {} task(s)", ids.len()),
            Command::SetTaskFlag { ids, flagged } => {
                format!("{} {} task(s)", if *flagged { "Flag" } else { "Unflag" }, ids.len())
            }
            Command::CreateProject(project) => format!("Add project “{}”", project.name),
            Command::UpdateProject { id, .. } => format!("Edit project {id}"),
            Command::SetProjectStatus { id, status } => {
                format!("Set project {id} to {}", status.label())
            }
            Command::CompleteReview { id, .. } => format!("Review project {id}"),
            Command::MoveProject { id, .. } => format!("Move project {id}"),
            Command::CreateFolder { name, .. } => format!("Add folder “{name}”"),
            Command::RenameFolder { id, .. } => format!("Rename folder {id}"),
            Command::MoveFolder { id, .. } => format!("Move folder {id}"),
            Command::DeleteFolder { id } => format!("Delete folder {id}"),
            Command::CreateTag { name, .. } => format!("Add tag “{name}”"),
            Command::UpdateTag { id, .. } => format!("Edit tag {id}"),
            Command::DeleteTag { id } => format!("Delete tag {id}"),
            Command::SavePerspective { perspective } => {
                format!("Save perspective “{}”", perspective.name)
            }
            Command::DeletePerspective { id } => format!("Delete perspective {id}"),
            Command::UpdateSettings(_) => "Update settings".to_string(),
            Command::Import { mode, .. } => match mode {
                ImportMode::Replace => "Import library".to_string(),
                ImportMode::Merge => "Merge import".to_string(),
            },
        }
    }

    pub fn apply(self, db: &mut Db, at: DateTime<Utc>) -> Result<Outcome> {
        let label = self.label();
        let mut outcome = Outcome::new(label);
        match self {
            Command::CreateTask(mut new_task) => {
                let title = new_task.title.trim();
                if title.is_empty() {
                    return Err(Error::invalid("task title cannot be empty"));
                }
                if let Some(path) = new_task.project_path.clone() {
                    new_task.project = Some(db.ensure_project_path(&path)?);
                }
                if !new_task.tag_paths.is_empty() {
                    for path in new_task.tag_paths.clone() {
                        let tag = db.ensure_tag_path(&path, true)?;
                        if !new_task.tags.contains(&tag) {
                            new_task.tags.push(tag);
                        }
                    }
                }
                if new_task.project.is_some() && new_task.parent.is_some() {
                    return Err(Error::invalid("a task cannot have both a project and a parent"));
                }
                let (project, parent) = match (&new_task.parent, &new_task.project) {
                    (Some(parent), _) => {
                        let parent_task = db.task(parent)?;
                        (parent_task.project.clone(), Some(parent.clone()))
                    }
                    (None, Some(project)) => {
                        db.project(project)?;
                        (Some(project.clone()), None)
                    }
                    (None, None) => (None, None),
                };
                let mut task = Task::new(title);
                task.notes = new_task.notes;
                task.project = project;
                task.parent = parent;
                task.tags = new_task.tags.into_iter().collect();
                task.due = new_task.due;
                task.defer = new_task.defer;
                task.estimate_min = new_task.estimate_min;
                task.flagged = new_task.flagged;
                task.repeat = new_task.repeat;
                task.sequential = new_task.sequential;
                task.modified_at = at;
                let id = task.id.clone();
                db.tasks.insert(id.clone(), task);
                let key = match &db.tasks[&id].parent {
                    Some(parent) => OrderKey::Parent(parent.clone()),
                    None => match &db.tasks[&id].project {
                        Some(project) => OrderKey::Project(project.clone()),
                        None => OrderKey::Inbox,
                    },
                };
                db.insert_task_at(key, id.clone(), new_task.position);
                db.touch();
                outcome.created.push(id.to_string());
                outcome.message = Some(format!("Added “{title}”"));
            }
            Command::UpdateTask { id, patch } => {
                let mut moved: Option<(Option<ProjectId>, Option<TaskId>)> = None;
                {
                    let task = db.task_mut(&id)?;
                    if let Some(title) = patch.title {
                        let title = title.trim();
                        if title.is_empty() {
                            return Err(Error::invalid("task title cannot be empty"));
                        }
                        task.title = title.to_string();
                    }
                    if let Some(notes) = patch.notes {
                        task.notes = notes;
                    }
                    if let Some(due) = patch.due {
                        task.due = due;
                    }
                    if let Some(defer) = patch.defer {
                        task.defer = defer;
                    }
                    if let Some(estimate) = patch.estimate_min {
                        task.estimate_min = estimate;
                    }
                    if let Some(flagged) = patch.flagged {
                        task.flagged = flagged;
                    }
                    if let Some(repeat) = patch.repeat {
                        task.repeat = repeat;
                    }
                    if let Some(sequential) = patch.sequential {
                        task.sequential = sequential;
                    }
                    if let Some(project) = patch.project {
                        moved = Some((project, None));
                    }
                    if let Some(parent) = patch.parent {
                        moved = Some((None, parent));
                    }
                    task.modified_at = at;
                }
                if let Some((project, parent)) = moved {
                    if parent.is_some() && project.is_some() {
                        return Err(Error::invalid("choose either a project or a parent task"));
                    }
                    let placement = if let Some(parent) = parent {
                        TaskPlacement::Parent {
                            parent,
                            position: Position::End,
                        }
                    } else if let Some(project) = project {
                        TaskPlacement::Project {
                            project,
                            position: Position::End,
                        }
                    } else {
                        TaskPlacement::Inbox(Position::End)
                    };
                    move_tasks(db, &[id.clone()], placement)?;
                }
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::CompleteTasks { ids, completed } => {
                for id in ids {
                    let Some(task) = db.tasks.get(&id) else {
                        continue;
                    };
                    if task.completed == completed {
                        continue;
                    }
                    let repeat = task.repeat.clone();
                    let due = task.due;
                    let defer = task.defer;
                    let key = db.order_key_for(&id);
                    let project = task.project.clone();
                    let parent = task.parent.clone();
                    let tags = task.tags.clone();
                    let title = task.title.clone();
                    let notes = task.notes.clone();
                    let estimate = task.estimate_min;
                    let flagged = task.flagged;
                    let sequential = task.sequential;
                    {
                        let task = db.task_mut(&id)?;
                        task.completed = completed;
                        task.completed_at = completed.then_some(at);
                        task.modified_at = at;
                    }
                    if completed {
                        if let Some(rule) = repeat {
                            if let Some(next) = rule.next_occurrence(due, defer, at) {
                                let mut spawned = Task::new(title);
                                spawned.notes = notes;
                                spawned.project = project;
                                spawned.parent = parent;
                                spawned.tags = tags;
                                spawned.due = next.due;
                                spawned.defer = next.defer;
                                spawned.estimate_min = estimate;
                                spawned.flagged = flagged;
                                spawned.sequential = sequential;
                                let mut next_rule = rule;
                                next_rule.completed_count = next.completed_count;
                                spawned.repeat = Some(next_rule);
                                let spawned_id = spawned.id.clone();
                                db.tasks.insert(spawned_id.clone(), spawned);
                                if let Some(key) = key {
                                    db.insert_task_at(
                                        key,
                                        spawned_id.clone(),
                                        Position::After(id.clone()),
                                    );
                                }
                                outcome.created.push(spawned_id.to_string());
                            }
                        }
                    }
                    outcome.affected.push(id.to_string());
                }
                db.touch();
            }
            Command::DeleteTasks { ids } => {
                let mut removed: BTreeSet<TaskId> = BTreeSet::new();
                for id in ids {
                    if db.tasks.contains_key(&id) {
                        for descendant in db.task_subtree(&id) {
                            removed.insert(descendant);
                        }
                    }
                }
                for id in &removed {
                    db.detach_task(id);
                    db.tasks.remove(id);
                }
                outcome.message = Some(format!("Deleted {} task(s)", removed.len()));
                db.touch();
            }
            Command::MoveTasks { ids, placement } => {
                move_tasks(db, &ids, placement)?;
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
            }
            Command::SetTaskTags { ids, add, remove } => {
                for id in &ids {
                    let task = db.task_mut(id)?;
                    for tag in &add {
                        task.tags.insert(tag.clone());
                    }
                    for tag in &remove {
                        task.tags.remove(tag);
                    }
                    task.modified_at = at;
                }
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
            }
            Command::SetTaskFlag { ids, flagged } => {
                for id in &ids {
                    let task = db.task_mut(id)?;
                    task.flagged = flagged;
                    task.modified_at = at;
                }
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
            }
            Command::CreateProject(new_project) => {
                let name = new_project.name.trim();
                if name.is_empty() {
                    return Err(Error::invalid("project name cannot be empty"));
                }
                if let Some(folder) = &new_project.folder {
                    db.folder(folder)?;
                }
                ensure_unique_project_name(db, new_project.folder.as_ref(), name, None)?;
                let mut project = Project::new(name);
                project.notes = new_project.notes;
                project.folder = new_project.folder.clone();
                project.tags = new_project.tags.into_iter().collect();
                project.due = new_project.due;
                project.defer = new_project.defer;
                project.review_interval_days = new_project
                    .review_interval_days
                    .unwrap_or(db.settings.default_review_days)
                    .max(1);
                project.next_review = new_project.next_review.unwrap_or_else(|| {
                    local_date(at) + chrono::Duration::days(project.review_interval_days as i64)
                });
                project.sequential = new_project.sequential;
                project.modified_at = at;
                let id = project.id.clone();
                db.projects.insert(id.clone(), project);
                db.insert_container(
                    new_project.folder,
                    ContainerId::Project(id.clone()),
                    new_project.position,
                );
                db.touch();
                outcome.created.push(id.to_string());
            }
            Command::UpdateProject { id, patch } => {
                if let Some(name) = &patch.name {
                    let name = name.trim();
                    if name.is_empty() {
                        return Err(Error::invalid("project name cannot be empty"));
                    }
                    let folder = db.project(&id)?.folder.clone();
                    ensure_unique_project_name(db, folder.as_ref(), name, Some(&id))?;
                }
                let folder_change = patch.folder.clone();
                {
                    let project = db.project_mut(&id)?;
                    if let Some(name) = patch.name {
                        project.name = name.trim().to_string();
                    }
                    if let Some(notes) = patch.notes {
                        project.notes = notes;
                    }
                    if let Some(tags) = patch.tags {
                        project.tags = tags;
                    }
                    if let Some(due) = patch.due {
                        project.due = due;
                    }
                    if let Some(defer) = patch.defer {
                        project.defer = defer;
                    }
                    if let Some(interval) = patch.review_interval_days {
                        project.review_interval_days = interval.max(1);
                    }
                    if let Some(review) = patch.next_review {
                        project.next_review = review;
                    }
                    if let Some(sequential) = patch.sequential {
                        project.sequential = sequential;
                    }
                    project.modified_at = at;
                }
                if let Some(folder) = folder_change {
                    if let Some(folder) = &folder {
                        db.folder(folder)?;
                    }
                    db.detach_container(&ContainerId::Project(id.clone()));
                    db.insert_container(
                        folder,
                        ContainerId::Project(id.clone()),
                        ContainerPosition::End,
                    );
                }
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::SetProjectStatus { id, status } => {
                let previous = db.project(&id)?.status;
                if previous == status {
                    return Ok(outcome);
                }
                {
                    let project = db.project_mut(&id)?;
                    project.status = status;
                    project.modified_at = at;
                }
                if status == ProjectStatus::Done {
                    let tasks = db.tasks_in_project(&id, true);
                    for task_id in tasks {
                        if let Some(task) = db.tasks.get_mut(&task_id) {
                            if !task.completed {
                                task.completed = true;
                                task.completed_at = Some(at);
                                task.modified_at = at;
                            }
                        }
                    }
                }
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(format!("Project marked {}", status.label()));
            }
            Command::CompleteReview { id, reviewed_on } => {
                let interval = db.project(&id)?.review_interval_days.max(1) as i64;
                let project = db.project_mut(&id)?;
                project.next_review = reviewed_on + chrono::Duration::days(interval);
                project.modified_at = at;
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::MoveProject { id, placement } => {
                let project = db.project(&id)?;
                if let Some(parent) = &placement.parent {
                    if project.folder.as_ref() == Some(parent) {
                        // Already there; still honour ordering.
                    }
                    db.folder(parent)?;
                }
                db.detach_container(&ContainerId::Project(id.clone()));
                db.insert_container(
                    placement.parent,
                    ContainerId::Project(id.clone()),
                    placement.position,
                );
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::CreateFolder {
                name,
                parent,
                position,
            } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(Error::invalid("folder name cannot be empty"));
                }
                if let Some(parent) = &parent {
                    db.folder(parent)?;
                }
                ensure_unique_folder_name(db, parent.as_ref(), name, None)?;
                let mut folder = Folder::new(name);
                folder.parent = parent.clone();
                let id = folder.id.clone();
                db.folders.insert(id.clone(), folder);
                db.insert_container(parent, ContainerId::Folder(id.clone()), position);
                db.touch();
                outcome.created.push(id.to_string());
            }
            Command::RenameFolder { id, name } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(Error::invalid("folder name cannot be empty"));
                }
                let parent = db.folder(&id)?.parent.clone();
                ensure_unique_folder_name(db, parent.as_ref(), name, Some(&id))?;
                db.folder_mut(&id)?.name = name.to_string();
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::MoveFolder { id, placement } => {
                if let Some(parent) = &placement.parent {
                    if parent == &id {
                        return Err(Error::invalid("a folder cannot contain itself"));
                    }
                    if db.folder_descendants(&id).contains(parent) {
                        return Err(Error::invalid("cannot move a folder into its descendant"));
                    }
                    db.folder(parent)?;
                }
                db.detach_container(&ContainerId::Folder(id.clone()));
                db.insert_container(
                    placement.parent,
                    ContainerId::Folder(id.clone()),
                    placement.position,
                );
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::DeleteFolder { id } => {
                let folder = db.folder(&id)?.clone();
                let parent = folder.parent.clone();
                for child in folder.child_order.clone() {
                    db.detach_container(&child);
                    db.insert_container(
                        parent.clone(),
                        child,
                        ContainerPosition::End,
                    );
                }
                db.detach_container(&ContainerId::Folder(id.clone()));
                db.folders.remove(&id);
                db.touch();
                outcome.message = Some("Folder deleted; its projects were moved up".to_string());
            }
            Command::CreateTag {
                name,
                parent,
                status,
                location,
                position,
            } => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(Error::invalid("tag name cannot be empty"));
                }
                if let Some(parent) = &parent {
                    db.tag(parent)?;
                }
                ensure_unique_tag_name(db, parent.as_ref(), name, None)?;
                let mut tag = Tag::new(name);
                tag.parent = parent.clone();
                tag.status = status;
                tag.location = location;
                let id = tag.id.clone();
                db.tags.insert(id.clone(), tag);
                match &parent {
                    Some(parent) => {
                        if let Some(parent_tag) = db.tags.get_mut(parent) {
                            let index = match position {
                                Position::Start => 0,
                                Position::End => usize::MAX,
                                Position::Index(index) => index,
                                _ => usize::MAX,
                            };
                            let index = index.min(parent_tag.child_order.len());
                            parent_tag.child_order.insert(index, id.clone());
                        }
                    }
                    None => {
                        let index = match position {
                            Position::Start => 0,
                            Position::End => usize::MAX,
                            Position::Index(index) => index,
                            _ => usize::MAX,
                        };
                        let index = index.min(db.tag_roots.len());
                        db.tag_roots.insert(index, id.clone());
                    }
                }
                db.touch();
                outcome.created.push(id.to_string());
            }
            Command::UpdateTag { id, patch } => {
                let old_parent = db.tag(&id)?.parent.clone();
                if let Some(name) = &patch.name {
                    let name = name.trim();
                    if name.is_empty() {
                        return Err(Error::invalid("tag name cannot be empty"));
                    }
                    let parent = patch.parent.clone().unwrap_or(old_parent.clone());
                    ensure_unique_tag_name(db, parent.as_ref(), name, Some(&id))?;
                }
                if let Some(Some(parent)) = &patch.parent {
                    if parent == &id {
                        return Err(Error::invalid("a tag cannot contain itself"));
                    }
                    if db.tag_descendants(&id).contains(parent) {
                        return Err(Error::invalid("cannot move a tag into its descendant"));
                    }
                    db.tag(parent)?;
                }
                {
                    let tag = db.tag_mut(&id)?;
                    if let Some(name) = patch.name {
                        tag.name = name.trim().to_string();
                    }
                    if let Some(status) = patch.status {
                        tag.status = status;
                    }
                    if let Some(location) = patch.location {
                        tag.location = location;
                    }
                    tag.modified_at = at;
                }
                if let Some(new_parent) = patch.parent {
                    let other_children = match &new_parent {
                        Some(parent) => db
                            .tags
                            .get(parent)
                            .map(|tag| tag.child_order.clone())
                            .unwrap_or_default(),
                        None => db.tag_roots.clone(),
                    };
                    let mut list = other_children;
                    list.retain(|child| child != &id);
                    list.push(id.clone());
                    match new_parent.clone() {
                        Some(parent) => {
                            if let Some(parent_tag) = db.tags.get_mut(&parent) {
                                parent_tag.child_order = list;
                            }
                        }
                        None => db.tag_roots = list,
                    }
                    if let Some(parent) = old_parent {
                        if Some(&parent) != new_parent.as_ref() {
                            if let Some(old) = db.tags.get_mut(&parent) {
                                old.child_order.retain(|child| child != &id);
                            }
                        }
                    }
                    db.tag_mut(&id)?.parent = new_parent;
                }
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::DeleteTag { id } => {
                let tag = db.tag(&id)?.clone();
                let parent = tag.parent.clone();
                for child in tag.child_order.clone() {
                    let index = match &parent {
                        Some(parent) => db
                            .tags
                            .get(parent)
                            .map(|tag| tag.child_order.len())
                            .unwrap_or(0),
                        None => db.tag_roots.len(),
                    };
                    match &parent {
                        Some(parent_id) => {
                            if let Some(parent_tag) = db.tags.get_mut(parent_id) {
                                parent_tag.child_order.insert(index, child.clone());
                            }
                        }
                        None => db.tag_roots.insert(index, child.clone()),
                    }
                    db.tags.get_mut(&child).unwrap().parent = parent.clone();
                }
                if let Some(parent) = &parent {
                    if let Some(parent_tag) = db.tags.get_mut(parent) {
                        parent_tag.child_order.retain(|child| child != &id);
                    }
                } else {
                    db.tag_roots.retain(|child| child != &id);
                }
                for task in db.tasks.values_mut() {
                    task.tags.remove(&id);
                }
                for project in db.projects.values_mut() {
                    project.tags.remove(&id);
                }
                db.tags.remove(&id);
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::SavePerspective { mut perspective } => {
                let name = perspective.name.trim();
                if name.is_empty() {
                    return Err(Error::invalid("perspective name cannot be empty"));
                }
                perspective.name = name.to_string();
                if perspective.id.as_str().is_empty() {
                    perspective.id = PerspectiveId::new();
                }
                if !db.perspectives.contains_key(&perspective.id) {
                    let next = db
                        .perspectives
                        .values()
                        .map(|item| item.position)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    perspective.position = next;
                }
                let id = perspective.id.clone();
                db.perspectives.insert(id.clone(), perspective);
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::DeletePerspective { id } => {
                db.perspectives.remove(&id);
                db.touch();
                outcome.affected.push(id.to_string());
            }
            Command::UpdateSettings(patch) => {
                let settings = &mut db.settings;
                if let Some(value) = patch.default_review_days {
                    settings.default_review_days = value.max(1);
                }
                if let Some(value) = patch.show_completed_days {
                    settings.show_completed_days = value.min(3650);
                }
                if let Some(value) = patch.week_starts_on {
                    settings.week_starts_on = value % 7;
                }
                if let Some(value) = patch.default_due_time {
                    settings.default_due_time = value;
                }
                if let Some(value) = patch.default_defer_time {
                    settings.default_defer_time = value;
                }
                if let Some(value) = patch.current_location {
                    settings.current_location = value;
                }
                if let Some(value) = patch.nearby_radius_km {
                    settings.nearby_radius_km = value.max(0.1);
                }
                db.touch();
            }
            Command::Import { db: imported, mode } => {
                imported.validate().map_err(|error| {
                    Error::Import(format!("imported library is invalid: {error}"))
                })?;
                match mode {
                    ImportMode::Replace => {
                        *db = imported;
                        db.touch();
                        outcome.message = Some("Library replaced from import".to_string());
                    }
                    ImportMode::Merge => {
                        let stats = merge_db(db, imported);
                        outcome.message = Some(format!(
                            "Merged import: {} new tasks, {} new projects, {} new tags",
                            stats.tasks, stats.projects, stats.tags
                        ));
                    }
                }
            }
        }
        Ok(outcome)
    }
}

fn move_tasks(db: &mut Db, ids: &[TaskId], placement: TaskPlacement) -> Result<()> {
    let mut unique: Vec<TaskId> = Vec::new();
    for id in ids {
        db.task(id)?;
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    // Prevent parent/child pairs from being moved twice.
    for id in unique.clone() {
        let ancestors = db.task_ancestors(&id);
        if unique.iter().any(|other| other != &id && ancestors.contains(other)) {
            unique.retain(|candidate| candidate != &id);
        }
    }

    let (project, parent, position) = match placement {
        TaskPlacement::Inbox(position) => (None, None, position),
        TaskPlacement::Project { project, position } => {
            db.project(&project)?;
            (Some(project), None, position)
        }
        TaskPlacement::Parent { parent, position } => {
            db.task(&parent)?;
            for id in &unique {
                if db.task_subtree(id).contains(&parent) {
                    return Err(Error::invalid("cannot move a task inside itself"));
                }
            }
            let inherited = db.task(&parent)?.project.clone();
            (inherited, Some(parent), position)
        }
    };

    for id in &unique {
        db.detach_task(id);
    }

    for id in &unique {
        {
            let task = db.task_mut(id)?;
            task.project = project.clone();
            task.parent = parent.clone();
            task.modified_at = now();
        }
        propagate_project(db, id, project.as_ref());
        let key = if let Some(parent) = &parent {
            OrderKey::Parent(parent.clone())
        } else if let Some(project) = &project {
            OrderKey::Project(project.clone())
        } else {
            OrderKey::Inbox
        };
        db.insert_task_at(key, id.clone(), position.clone());
    }
    Ok(())
}

fn propagate_project(db: &mut Db, id: &TaskId, project: Option<&ProjectId>) {
    let descendant_ids = db.task_subtree(id);
    for descendant in descendant_ids {
        if let Some(task) = db.tasks.get_mut(&descendant) {
            task.project = project.cloned();
        }
    }
}

fn ensure_unique_project_name(
    db: &Db,
    folder: Option<&FolderId>,
    name: &str,
    except: Option<&ProjectId>,
) -> Result<()> {
    let duplicate = db.projects.values().any(|project| {
        Some(&project.id) != except
            && project.folder.as_ref() == folder
            && project.name.eq_ignore_ascii_case(name)
    });
    if duplicate {
        Err(Error::invalid(format!(
            "a project named “{name}” already exists here"
        )))
    } else {
        Ok(())
    }
}

fn ensure_unique_folder_name(
    db: &Db,
    parent: Option<&FolderId>,
    name: &str,
    except: Option<&FolderId>,
) -> Result<()> {
    let duplicate = db.folders.values().any(|folder| {
        Some(&folder.id) != except
            && folder.parent.as_ref() == parent
            && folder.name.eq_ignore_ascii_case(name)
    });
    if duplicate {
        Err(Error::invalid(format!(
            "a folder named “{name}” already exists here"
        )))
    } else {
        Ok(())
    }
}

fn ensure_unique_tag_name(
    db: &Db,
    parent: Option<&TagId>,
    name: &str,
    except: Option<&TagId>,
) -> Result<()> {
    let duplicate = db.tags.values().any(|tag| {
        Some(&tag.id) != except
            && tag.parent.as_ref() == parent
            && tag.name.eq_ignore_ascii_case(name)
    });
    if duplicate {
        Err(Error::invalid(format!(
            "a tag named “{name}” already exists here"
        )))
    } else {
        Ok(())
    }
}

#[derive(Default, Debug)]
struct MergeStats {
    tasks: usize,
    projects: usize,
    tags: usize,
    folders: usize,
}

fn merge_db(db: &mut Db, imported: Db) -> MergeStats {
    let mut stats = MergeStats::default();
    let mut ordered_tasks: BTreeSet<TaskId> = BTreeSet::new();
    for task in db.tasks.values() {
        ordered_tasks.insert(task.id.clone());
        for child in &task.child_order {
            ordered_tasks.insert(child.clone());
        }
    }
    for project in db.projects.values() {
        for task in &project.task_order {
            ordered_tasks.insert(task.clone());
        }
    }
    for task in &db.inbox_order {
        ordered_tasks.insert(task.clone());
    }

    for (id, tag) in &imported.tags {
        if !db.tags.contains_key(id) {
            db.tags.insert(id.clone(), tag.clone());
            stats.tags += 1;
        }
    }
    for (id, folder) in &imported.folders {
        if !db.folders.contains_key(id) {
            db.folders.insert(id.clone(), folder.clone());
            stats.folders += 1;
        }
    }
    for (id, project) in &imported.projects {
        if !db.projects.contains_key(id) {
            db.projects.insert(id.clone(), project.clone());
            stats.projects += 1;
        }
    }
    for (id, task) in &imported.tasks {
        if !db.tasks.contains_key(id) {
            db.tasks.insert(id.clone(), task.clone());
            stats.tasks += 1;
        }
    }

    // Merge ordering arrays by appending unknown ids.
    for container in imported.root {
        if !db.root.contains(&container) {
            db.root.push(container);
        }
    }
    for id in imported.tag_roots {
        if !db.tag_roots.contains(&id) {
            db.tag_roots.push(id);
        }
    }
    for (id, folder) in &imported.folders {
        if let Some(target) = db.folders.get_mut(id) {
            for child in &folder.child_order {
                if !target.child_order.contains(child) {
                    target.child_order.push(child.clone());
                }
            }
        }
    }
    for (id, project) in &imported.projects {
        if let Some(target) = db.projects.get_mut(id) {
            for task in &project.task_order {
                if !target.task_order.contains(task) {
                    target.task_order.push(task.clone());
                }
            }
        }
    }
    for (id, tag) in &imported.tags {
        if let Some(target) = db.tags.get_mut(id) {
            for child in &tag.child_order {
                if !target.child_order.contains(child) {
                    target.child_order.push(child.clone());
                }
            }
        }
    }
    for (id, task) in &imported.tasks {
        if let Some(target) = db.tasks.get_mut(id) {
            for child in &task.child_order {
                if !target.child_order.contains(child) {
                    target.child_order.push(child.clone());
                }
            }
        }
    }
    for task in imported.inbox_order {
        if db.tasks.contains_key(&task) && !db.inbox_order.contains(&task) {
            db.inbox_order.push(task);
        }
    }
    // Any imported task that is not in an order list lands in the Inbox.
    for id in db.tasks.keys() {
        if !ordered_tasks.contains(id)
            && !db.inbox_order.contains(id)
            && !db.projects.values().any(|p| p.task_order.contains(id))
            && !db.tasks.values().any(|t| t.child_order.contains(id))
        {
            db.inbox_order.push(id.clone());
        }
    }
    db.touch();
    stats
}
