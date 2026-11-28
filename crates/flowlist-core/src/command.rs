//! Every mutation goes through a serializable `Command`. The store wraps each
//! command in a snapshot so undo/redo covers all operations, including tree
//! moves and bulk changes.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::db::{ContainerPosition, DateConflict, Db, OrderKey, Position};
use crate::error::{Error, Result};
use crate::ids::{ChecklistId, ContainerId, FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use crate::model::{
    ChecklistItem, DefaultNotification, Folder, Geo, IconSpec, Perspective, Priority, Project,
    ProjectKind, ProjectStatus, RepeatMode, RepeatRule, Tag, TagStatus, Task, TaskAlert,
    increment_repeat_title,
};
use crate::time::{Stamp, local_date, now};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewTask {
    /// A client-supplied id, used when a phone captures offline so the task
    /// keeps one identity before it ever reaches this library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<TaskId>,
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
    pub planned: Option<Stamp>,
    pub estimate_min: Option<u32>,
    pub flagged: bool,
    /// Opt this task into full-screen reminders.
    #[serde(default)]
    pub fullscreen_reminder: bool,
    /// Independent reminder time; ignored while `reminder_match_due` is true.
    #[serde(default)]
    pub reminder_at: Option<Stamp>,
    /// Follow the due date and time instead of `reminder_at`.
    #[serde(default = "default_true")]
    pub reminder_match_due: bool,
    #[serde(default)]
    pub alerts: Vec<TaskAlert>,
    #[serde(default)]
    pub removed_notifications: BTreeSet<DefaultNotification>,
    #[serde(default)]
    pub custom_notifications: Vec<Stamp>,
    pub priority: Option<Priority>,
    pub attachments: Vec<String>,
    pub repeat: Option<RepeatRule>,
    pub sequential: bool,
    pub position: Position,
}

impl NewTask {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            notes: String::new(),
            project: None,
            project_path: None,
            parent: None,
            tags: Vec::new(),
            tag_paths: Vec::new(),
            due: None,
            defer: None,
            planned: None,
            estimate_min: None,
            flagged: false,
            fullscreen_reminder: false,
            reminder_at: None,
            reminder_match_due: true,
            alerts: Vec::new(),
            removed_notifications: BTreeSet::new(),
            custom_notifications: Vec::new(),
            priority: None,
            attachments: Vec::new(),
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

/// Deserialize an `Option<Option<T>>` so an explicit JSON `null` clears a
/// field while a missing field leaves it untouched. Serde's plain
/// `Option<Option<T>>` collapses both cases to `None`, which made a phone's
/// "priority: null" patch look like it had never been sent.
fn deserialize_double_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub notes: Option<String>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub project: Option<Option<ProjectId>>,
    /// A `Folder / Project` path. Missing projects (and their folders) are
    /// created in the same undoable step.
    #[serde(deserialize_with = "deserialize_double_option")]
    pub project_path: Option<Option<String>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub parent: Option<Option<TaskId>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub due: Option<Option<Stamp>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub defer: Option<Option<Stamp>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub planned: Option<Option<Stamp>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub estimate_min: Option<Option<u32>>,
    pub flagged: Option<bool>,
    pub fullscreen_reminder: Option<bool>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub reminder_at: Option<Option<Stamp>>,
    pub reminder_match_due: Option<bool>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub priority: Option<Option<Priority>>,
    /// Replace the task's extra relative notification schedules.
    #[serde(default)]
    pub alerts: Option<Vec<TaskAlert>>,
    #[serde(default)]
    pub removed_notifications: Option<BTreeSet<DefaultNotification>>,
    #[serde(default)]
    pub custom_notifications: Option<Vec<Stamp>>,
    pub attachments: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub repeat: Option<Option<RepeatRule>>,
    pub sequential: Option<bool>,
    pub steps_one_off: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub notes: String,
    pub folder: Option<FolderId>,
    pub tags: Vec<TagId>,
    pub due: Option<Stamp>,
    pub defer: Option<Stamp>,
    pub planned: Option<Stamp>,
    pub review_interval_days: Option<u32>,
    pub next_review: Option<NaiveDate>,
    pub sequential: bool,
    pub kind: ProjectKind,
    pub complete_with_last_action: bool,
    pub icon: IconSpec,
    pub attachments: Vec<String>,
    /// Opt the project's tasks into full-screen reminders.
    #[serde(default)]
    pub fullscreen_reminder: bool,
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
            planned: None,
            review_interval_days: None,
            next_review: None,
            sequential: false,
            kind: ProjectKind::Parallel,
            complete_with_last_action: true,
            icon: IconSpec::default(),
            attachments: Vec::new(),
            fullscreen_reminder: false,
            position: ContainerPosition::End,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectPatch {
    pub name: Option<String>,
    pub notes: Option<String>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub folder: Option<Option<FolderId>>,
    pub tags: Option<BTreeSet<TagId>>,
    pub flagged: Option<bool>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub due: Option<Option<Stamp>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub defer: Option<Option<Stamp>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub planned: Option<Option<Stamp>>,
    pub review_interval_days: Option<u32>,
    pub next_review: Option<NaiveDate>,
    pub sequential: Option<bool>,
    pub kind: Option<ProjectKind>,
    pub complete_with_last_action: Option<bool>,
    /// Hide or restore the project in Archive without changing its status.
    pub archived: Option<bool>,
    pub icon: Option<IconSpec>,
    #[serde(default)]
    pub alerts: Option<Vec<TaskAlert>>,
    #[serde(default)]
    pub removed_notifications: Option<BTreeSet<DefaultNotification>>,
    #[serde(default)]
    pub custom_notifications: Option<Vec<Stamp>>,
    pub attachments: Option<Vec<String>>,
    pub fullscreen_reminder: Option<bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TagPatch {
    pub name: Option<String>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub parent: Option<Option<TagId>>,
    pub status: Option<TagStatus>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub location: Option<Option<Geo>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsPatch {
    pub default_review_days: Option<u32>,
    pub show_completed_days: Option<u32>,
    pub week_starts_on: Option<u8>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub default_due_time: Option<Option<chrono::NaiveTime>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub default_defer_time: Option<Option<chrono::NaiveTime>>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub current_location: Option<Option<Geo>>,
    pub nearby_radius_km: Option<f64>,
    pub trash_retention_days: Option<u32>,
    pub changed_days: Option<u32>,
    pub due_soon_days: Option<u32>,
    pub forecast_due_counts: Option<bool>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub forecast_tag: Option<Option<TagId>>,
    pub forecast_flagged: Option<bool>,
    pub default_defer_notification: Option<bool>,
    pub default_planned_notification: Option<bool>,
    pub default_latest_start_notification: Option<bool>,
    pub default_due_notification: Option<bool>,
    #[serde(deserialize_with = "deserialize_double_option")]
    pub default_custom_notification_time: Option<Option<chrono::NaiveTime>>,
    pub auto_archive_closed_projects: Option<bool>,
    pub sidebar: Option<crate::model::SidebarOptions>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerPlacement {
    pub parent: Option<FolderId>,
    pub position: ContainerPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    Replace,
    Merge,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    CreateTask(NewTask),
    UpdateTask {
        id: TaskId,
        patch: TaskPatch,
    },
    /// Apply a task patch and explicitly clamp the task's dates to its new
    /// container. Used only after the user confirms the adjustment.
    UpdateTaskAdjusted {
        id: TaskId,
        patch: TaskPatch,
    },
    /// Apply one patch to several tasks as a single undo step. Tag paths are
    /// resolved (and added tags created) inside the same command.
    BatchUpdateTasks {
        ids: Vec<TaskId>,
        patch: TaskPatch,
        #[serde(default)]
        add_tags: Vec<String>,
        #[serde(default)]
        remove_tags: Vec<String>,
    },
    CompleteTasks {
        ids: Vec<TaskId>,
        completed: bool,
    },
    DeleteTasks {
        ids: Vec<TaskId>,
    },
    /// Move a task and every occurrence in its repeating series to Trash.
    DeleteTaskSeries {
        ids: Vec<TaskId>,
    },
    MoveTasks {
        ids: Vec<TaskId>,
        placement: TaskPlacement,
    },
    /// Move tasks and explicitly clamp conflicting stored dates to the new
    /// container. Used only after the user confirms the adjustment.
    MoveTasksAdjusted {
        ids: Vec<TaskId>,
        placement: TaskPlacement,
    },
    SetTaskTags {
        ids: Vec<TaskId>,
        add: Vec<TagId>,
        remove: Vec<TagId>,
    },
    SetTaskFlag {
        ids: Vec<TaskId>,
        flagged: bool,
    },
    SetTaskPriority {
        ids: Vec<TaskId>,
        priority: Option<Priority>,
        /// Apply the priority to every occurrence in each task's repeating
        /// series, not just the selected occurrence.
        #[serde(default)]
        series: bool,
    },
    /// Move tasks out of Trash without changing their place in the library.
    RestoreTasks {
        ids: Vec<TaskId>,
    },
    /// Permanently remove tasks, including their subtrees.
    PurgeTasks {
        ids: Vec<TaskId>,
    },
    /// Replace a task's child tasks with one-off checklist steps.
    ConvertSubtasksToOneOff {
        id: TaskId,
    },
    /// Turn a task's one-off checklist steps back into child tasks.
    ConvertOneOffToSubtasks {
        id: TaskId,
    },
    /// Switch a task between parallel, sequential and one-off steps. Switching
    /// to one-off converts child tasks into checklist steps; switching back
    /// restores those steps as child tasks. One command keeps the change and
    /// its conversion in a single undo step.
    SetTaskStepMode {
        id: TaskId,
        sequential: bool,
        steps_one_off: bool,
    },
    /// Turn a task (and its subtasks) into a project in the same folder.
    ConvertTaskToProject {
        id: TaskId,
        folder: Option<FolderId>,
    },
    /// Housekeeping: permanently remove trash older than `before`.
    PurgeExpiredTrash {
        before: DateTime<Utc>,
    },
    AddChecklistItem {
        task: TaskId,
        title: String,
        /// Insert before this index; `None` appends.
        position: Option<usize>,
    },
    UpdateChecklistItem {
        task: TaskId,
        item: ChecklistId,
        title: String,
        completed: Option<bool>,
    },
    MoveChecklistItem {
        task: TaskId,
        item: ChecklistId,
        to: usize,
    },
    DeleteChecklistItem {
        task: TaskId,
        item: ChecklistId,
    },
    CreateProject(NewProject),
    UpdateProject {
        id: ProjectId,
        patch: ProjectPatch,
    },
    /// Apply a project patch and explicitly clamp its tasks to the new dates.
    /// Used only after the user confirms the adjustment.
    UpdateProjectAdjusted {
        id: ProjectId,
        patch: ProjectPatch,
    },
    SetProjectStatus {
        id: ProjectId,
        status: ProjectStatus,
    },
    /// Hide or restore projects in Archive without changing their status.
    ArchiveProjects {
        ids: Vec<ProjectId>,
        archived: bool,
    },
    CompleteReview {
        id: ProjectId,
        reviewed_on: NaiveDate,
    },
    MoveProject {
        id: ProjectId,
        placement: ContainerPlacement,
    },
    /// Remove a project and move everything inside it to Trash. Undo brings
    /// the project and its tasks back in one step.
    DeleteProject {
        id: ProjectId,
    },
    CreateFolder {
        name: String,
        parent: Option<FolderId>,
        position: ContainerPosition,
        #[serde(default)]
        fullscreen_reminder: bool,
    },
    RenameFolder {
        id: FolderId,
        name: String,
    },
    MoveFolder {
        id: FolderId,
        placement: ContainerPlacement,
    },
    DeleteFolder {
        id: FolderId,
    },
    /// Set the same status on several projects in one undo step.
    SetProjectStatuses {
        ids: Vec<ProjectId>,
        status: ProjectStatus,
    },
    /// Delete several projects and folders in one undo step. Nested folders
    /// and projects are removed once, and their tasks move to Trash.
    DeleteContainers {
        projects: Vec<ProjectId>,
        folders: Vec<FolderId>,
    },
    CreateTag {
        name: String,
        parent: Option<TagId>,
        status: TagStatus,
        location: Option<Geo>,
        position: Position,
    },
    UpdateTag {
        id: TagId,
        patch: TagPatch,
    },
    DeleteTag {
        id: TagId,
    },
    SavePerspective {
        perspective: Perspective,
    },
    DeletePerspective {
        id: PerspectiveId,
    },
    UpdateSettings(SettingsPatch),
    Import {
        db: Db,
        mode: ImportMode,
    },
}

impl Command {
    pub fn label(&self) -> String {
        self.label_with(None)
    }

    /// A history label that prefers human names over opaque ids. Callers pass
    /// the database state from *before* the command, which is also the state
    /// an Undo restores, so deleted entities still have a readable name.
    pub fn label_with(&self, db: Option<&Db>) -> String {
        let task_name = |id: &TaskId| {
            db.and_then(|db| db.tasks.get(id))
                .map(|task| task.title.clone())
        };
        let project_name = |id: &ProjectId| {
            db.and_then(|db| db.projects.get(id))
                .map(|project| project.name.clone())
        };
        let folder_name = |id: &FolderId| {
            db.and_then(|db| db.folders.get(id))
                .map(|folder| folder.name.clone())
        };
        let tag_name = |id: &TagId| {
            db.and_then(|db| db.tags.get(id))
                .map(|tag| tag.name.clone())
        };
        let perspective_name = |id: &PerspectiveId| {
            db.and_then(|db| db.perspectives.get(id))
                .map(|perspective| perspective.name.clone())
        };

        match self {
            Command::CreateTask(task) => format!("Add task “{}”", task.title),
            Command::UpdateTask { id, .. } => match task_name(id) {
                Some(name) => format!("Edit task “{name}”"),
                None => "Edit task".to_string(),
            },
            Command::CompleteTasks { ids, completed } => {
                let verb = if *completed { "Complete" } else { "Reopen" };
                if ids.len() == 1 {
                    match task_name(&ids[0]) {
                        Some(name) => format!("{verb} task “{name}”"),
                        None => format!("{verb} task"),
                    }
                } else {
                    format!("{verb} {} tasks", ids.len())
                }
            }
            Command::DeleteTasks { ids } => {
                format!("Move {} to Trash", task_count(ids.len()))
            }
            Command::DeleteTaskSeries { ids } => {
                format!("Move {} in the series to Trash", task_count(ids.len()))
            }
            Command::MoveTasks { ids, .. } | Command::MoveTasksAdjusted { ids, .. } => {
                format!("Move {}", task_count(ids.len()))
            }
            Command::UpdateTaskAdjusted { .. } => "Change task".to_string(),
            Command::BatchUpdateTasks { ids, .. } => {
                format!("Edit {}", task_count(ids.len()))
            }
            Command::UpdateProjectAdjusted { .. } => "Change project".to_string(),
            Command::SetTaskTags { ids, .. } => format!("Tag {}", task_count(ids.len())),
            Command::SetTaskFlag { ids, flagged } => {
                format!(
                    "{} {}",
                    if *flagged { "Flag" } else { "Unflag" },
                    task_count(ids.len())
                )
            }
            Command::SetTaskPriority {
                ids,
                priority,
                series,
            } => format!(
                "Set {} to {} priority{}",
                task_count(ids.len()),
                priority.map(|p| p.label()).unwrap_or("no"),
                if *series { " across the series" } else { "" }
            ),
            Command::RestoreTasks { ids } => format!("Restore {}", task_count(ids.len())),
            Command::PurgeTasks { ids } => {
                format!("Permanently delete {}", task_count(ids.len()))
            }
            Command::ConvertSubtasksToOneOff { .. } => {
                "Convert subtasks to one-off steps".to_string()
            }
            Command::ConvertOneOffToSubtasks { .. } => {
                "Restore one-off steps as subtasks".to_string()
            }
            Command::SetTaskStepMode { steps_one_off, .. } => {
                if *steps_one_off {
                    "Switch to one-off steps".to_string()
                } else {
                    "Switch to subtasks".to_string()
                }
            }
            Command::ConvertTaskToProject { .. } => "Convert task to project".to_string(),
            Command::PurgeExpiredTrash { .. } => "Empty expired trash".to_string(),
            Command::AddChecklistItem { .. } => "Add checklist step".to_string(),
            Command::UpdateChecklistItem { .. } => "Update checklist step".to_string(),
            Command::MoveChecklistItem { .. } => "Reorder checklist step".to_string(),
            Command::DeleteChecklistItem { .. } => "Delete checklist step".to_string(),
            Command::CreateProject(project) => format!("Add project “{}”", project.name),
            Command::UpdateProject { id, .. } => match project_name(id) {
                Some(name) => format!("Edit project “{name}”"),
                None => "Edit project".to_string(),
            },
            Command::SetProjectStatus { id, status } => match project_name(id) {
                Some(name) => format!("Set project “{name}” to {}", status.label()),
                None => format!("Set project status to {}", status.label()),
            },
            Command::ArchiveProjects { ids, archived } => format!(
                "{} {}",
                if *archived { "Archive" } else { "Unarchive" },
                item_count(ids.len(), "project", "projects")
            ),
            Command::CompleteReview { id, .. } => match project_name(id) {
                Some(name) => format!("Review project “{name}”"),
                None => "Review project".to_string(),
            },
            Command::MoveProject { id, .. } => match project_name(id) {
                Some(name) => format!("Move project “{name}”"),
                None => "Move project".to_string(),
            },
            Command::DeleteProject { id } => match project_name(id) {
                Some(name) => format!("Delete project “{name}”"),
                None => "Delete project".to_string(),
            },
            Command::CreateFolder { name, .. } => format!("Add folder “{name}”"),
            Command::RenameFolder { id, .. } => match folder_name(id) {
                Some(name) => format!("Rename folder “{name}”"),
                None => "Rename folder".to_string(),
            },
            Command::MoveFolder { id, .. } => match folder_name(id) {
                Some(name) => format!("Move folder “{name}”"),
                None => "Move folder".to_string(),
            },
            Command::DeleteFolder { id } => match folder_name(id) {
                Some(name) => format!("Delete folder “{name}”"),
                None => "Delete folder".to_string(),
            },
            Command::SetProjectStatuses { ids, status } => {
                format!(
                    "Set {} to {}",
                    item_count(ids.len(), "project", "projects"),
                    status.label()
                )
            }
            Command::DeleteContainers { projects, folders } => {
                format!(
                    "Delete {}",
                    item_count(projects.len() + folders.len(), "item", "items")
                )
            }
            Command::CreateTag { name, .. } => format!("Add tag “{name}”"),
            Command::UpdateTag { id, .. } => match tag_name(id) {
                Some(name) => format!("Edit tag “{name}”"),
                None => "Edit tag".to_string(),
            },
            Command::DeleteTag { id } => match tag_name(id) {
                Some(name) => format!("Delete tag “{name}”"),
                None => "Delete tag".to_string(),
            },
            Command::SavePerspective { perspective } => {
                format!("Save perspective “{}”", perspective.name)
            }
            Command::DeletePerspective { id } => match perspective_name(id) {
                Some(name) => format!("Delete perspective “{name}”"),
                None => "Delete perspective".to_string(),
            },
            Command::UpdateSettings(_) => "Change settings".to_string(),
            Command::Import { mode, .. } => match mode {
                ImportMode::Replace => "Import library".to_string(),
                ImportMode::Merge => "Import and merge".to_string(),
            },
        }
    }

    pub fn apply(self, db: &mut Db, at: DateTime<Utc>) -> Result<Outcome> {
        let label = self.label_with(Some(&*db));
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
                    return Err(Error::invalid(
                        "a task cannot have both a project and a parent",
                    ));
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
                if let Some(id) = new_task.id.clone() {
                    if db.tasks.contains_key(&id) {
                        return Err(Error::invalid(format!("task {id} already exists")));
                    }
                    task.id = id;
                }
                task.notes = new_task.notes;
                task.project = project;
                task.parent = parent;
                task.tags = new_task.tags.into_iter().collect();
                task.due = new_task.due;
                task.defer = new_task.defer;
                task.planned = new_task.planned;
                ensure_dates_in_order(task.due, task.defer)?;
                task.estimate_min = new_task.estimate_min;
                task.flagged = new_task.flagged;
                task.fullscreen_reminder = new_task.fullscreen_reminder;
                task.reminder_at = new_task.reminder_at;
                task.reminder_match_due = new_task.reminder_match_due;
                task.alerts = new_task.alerts;
                task.removed_notifications = new_task.removed_notifications;
                task.custom_notifications = new_task.custom_notifications;
                task.priority = new_task.priority;
                task.attachments = new_task.attachments;
                task.repeat = new_task.repeat;
                if task.repeat.is_some() {
                    task.repeat_series = Some(task.id.clone());
                }
                ensure_repeat_has_clock(task.repeat.as_ref(), task.due, task.defer)?;
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
                outcome.message = Some(format!("Task “{title}” added"));
            }
            Command::UpdateTaskAdjusted { id, patch } => {
                Command::UpdateTask {
                    id: id.clone(),
                    patch,
                }
                .apply(db, at)?;
                let adjusted = db.clamp_task_to_container(&id);
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(if adjusted {
                    "Task updated; dates adjusted".to_string()
                } else {
                    "Task updated".to_string()
                });
            }
            Command::BatchUpdateTasks {
                ids,
                patch,
                add_tags,
                remove_tags,
            } => {
                if ids.is_empty() {
                    return Ok(outcome);
                }
                for id in ids.iter() {
                    Command::UpdateTask {
                        id: id.clone(),
                        patch: patch.clone(),
                    }
                    .apply(db, at)?;
                }
                let add: Vec<TagId> = add_tags
                    .iter()
                    .map(|path| path.trim())
                    .filter(|path| !path.is_empty())
                    .map(|path| db.ensure_tag_path(path, true))
                    .collect::<Result<Vec<_>>>()?;
                let remove: Vec<TagId> = remove_tags
                    .iter()
                    .filter_map(|path| db.find_tag_path(path.trim()))
                    .collect();
                if !add.is_empty() || !remove.is_empty() {
                    Command::SetTaskTags {
                        ids: ids.clone(),
                        add,
                        remove,
                    }
                    .apply(db, at)?;
                }
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                outcome.message = Some(format!("Edited {}", task_count(ids.len())));
            }
            Command::UpdateTask { id, patch } => {
                // Resolve or create a project path before borrowing the task so
                // a brand-new project and the assignment share one history step.
                let path_project = match &patch.project_path {
                    Some(Some(path)) if !path.trim().is_empty() => {
                        Some(Some(db.ensure_project_path(path)?))
                    }
                    Some(_) => Some(None),
                    None => None,
                };
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
                    if let Some(planned) = patch.planned {
                        task.planned = planned;
                    }
                    if let Some(estimate) = patch.estimate_min {
                        task.estimate_min = estimate;
                    }
                    if let Some(flagged) = patch.flagged {
                        task.flagged = flagged;
                    }
                    if let Some(fullscreen) = patch.fullscreen_reminder {
                        task.fullscreen_reminder = fullscreen;
                    }
                    if let Some(reminder_at) = patch.reminder_at {
                        task.reminder_at = reminder_at;
                    }
                    if let Some(match_due) = patch.reminder_match_due {
                        task.reminder_match_due = match_due;
                    }
                    if let Some(priority) = patch.priority {
                        task.priority = priority;
                    }
                    if let Some(alerts) = patch.alerts {
                        task.alerts = alerts;
                    }
                    if let Some(removed) = patch.removed_notifications {
                        task.removed_notifications = removed;
                    }
                    if let Some(custom) = patch.custom_notifications {
                        task.custom_notifications = custom;
                    }
                    if let Some(attachments) = patch.attachments {
                        task.attachments = attachments
                            .into_iter()
                            .map(|path| path.trim().to_string())
                            .filter(|path| !path.is_empty())
                            .collect();
                    }
                    if let Some(repeat) = patch.repeat {
                        task.repeat = repeat;
                    }
                    // The first occurrence owns the series id; every spawned
                    // occurrence inherits it so it can be found later.
                    if task.repeat.is_some() && task.repeat_series.is_none() {
                        task.repeat_series = Some(task.id.clone());
                    } else if task.repeat.is_none() {
                        task.repeat_series = None;
                    }
                    if let Some(sequential) = patch.sequential {
                        task.sequential = sequential;
                    }
                    if let Some(one_off) = patch.steps_one_off {
                        task.steps_one_off = one_off;
                    }
                    if let Some(project) = path_project {
                        moved = Some((project, None));
                    } else if let Some(project) = patch.project {
                        moved = Some((project, None));
                    }
                    if let Some(parent) = patch.parent {
                        moved = Some((None, parent));
                    }
                    ensure_repeat_has_clock(task.repeat.as_ref(), task.due, task.defer)?;
                    task.modified_at = at;
                    ensure_dates_in_order(task.due, task.defer)?;
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
                let title = db
                    .tasks
                    .get(&id)
                    .map(|task| task.title.clone())
                    .unwrap_or_default();
                outcome.message = Some(if title.is_empty() {
                    "Task updated".to_string()
                } else {
                    format!("Task “{title}” updated")
                });
            }
            Command::CompleteTasks { ids, completed } => {
                for id in &ids {
                    let Some(task) = db.tasks.get(id) else {
                        continue;
                    };
                    if task.completed == completed {
                        continue;
                    }
                    if let Some(project_id) = db.effective_project(id)
                        && let Some(project) = db.projects.get(&project_id)
                        && project.status == ProjectStatus::OnHold
                    {
                        let message = if completed {
                            "Set it active before completing tasks."
                        } else {
                            "Set it active to edit the task."
                        };
                        return Err(Error::invalid(format!(
                            "Project “{}” is on hold. {message}",
                            project.name
                        )));
                    }
                }
                let mut changed = 0usize;
                let mut first_title: Option<String> = None;
                let mut completed_projects: BTreeSet<ProjectId> = BTreeSet::new();
                for id in ids {
                    let Some(task) = db.tasks.get(&id) else {
                        continue;
                    };
                    if task.completed == completed {
                        continue;
                    }
                    if completed {
                        if let Some(blocker) = crate::query::availability::sequence_blocker(db, &id)
                        {
                            let title = db
                                .tasks
                                .get(&blocker)
                                .map(|task| task.title.clone())
                                .unwrap_or_default();
                            return Err(Error::invalid(format!(
                                "Finish “{title}” first — this is a sequential list"
                            )));
                        }
                    }
                    let repeat = task.repeat.clone();
                    let removed_notifications = task.removed_notifications.clone();
                    let due = task.due;
                    let defer = task.defer;
                    let planned = task.planned;
                    let key = db.order_key_for(&id);
                    let project = task.project.clone();
                    let parent = task.parent.clone();
                    let tags = task.tags.clone();
                    let title = task.title.clone();
                    let series = task
                        .repeat_series
                        .clone()
                        .unwrap_or_else(|| task.id.clone());
                    // Priority belongs to the series unless it was changed on
                    // this occurrence alone. The first occurrence owns it.
                    let series_priority = match db.tasks.get(&series) {
                        Some(root) => root.priority,
                        None => task.priority,
                    };
                    let notes = task.notes.clone();
                    let estimate = task.estimate_min;
                    let flagged = task.flagged;
                    let fullscreen_reminder = task.fullscreen_reminder;
                    let reminder_at = task.reminder_at;
                    let reminder_match_due = task.reminder_match_due;
                    let alerts = task.alerts.clone();
                    let sequential = task.sequential;
                    {
                        let task = db.task_mut(&id)?;
                        task.completed = completed;
                        task.completed_at = completed.then_some(at);
                        if task.repeat.is_some() {
                            task.repeat_series = Some(series.clone());
                        }
                        task.modified_at = at;
                    }
                    changed += 1;
                    if first_title.is_none() {
                        first_title = Some(title.clone());
                    }
                    if completed {
                        if let Some(project) = &project {
                            completed_projects.insert(project.clone());
                        }
                        if let Some(rule) = repeat {
                            if let Some(next) = rule.next_occurrence(due, defer, at) {
                                let mut spawned = Task::new(increment_repeat_title(&title));
                                spawned.repeat_series = Some(series.clone());
                                spawned.notes = notes;
                                spawned.project = project;
                                spawned.parent = parent;
                                spawned.tags = tags;
                                spawned.due = next.due;
                                spawned.defer = next.defer;
                                spawned.planned = match (planned, due, next.due) {
                                    (Some(planned), Some(old_due), Some(new_due)) => {
                                        let days = (new_due.local_date() - old_due.local_date())
                                            .num_days();
                                        let date =
                                            planned.local_date() + chrono::Duration::days(days);
                                        Some(if planned.has_time {
                                            Stamp::on(date, planned.local_time())
                                        } else {
                                            Stamp::date_only(date)
                                        })
                                    }
                                    (planned, _, _) => planned,
                                };
                                spawned.estimate_min = estimate;
                                spawned.flagged = flagged;
                                spawned.fullscreen_reminder = fullscreen_reminder;
                                spawned.reminder_at = reminder_at;
                                spawned.reminder_match_due = reminder_match_due;
                                spawned.alerts = alerts.clone();
                                spawned.removed_notifications = removed_notifications.clone();
                                spawned.priority = series_priority;
                                spawned.sequential = sequential;
                                spawned.checklist = db
                                    .tasks
                                    .get(&id)
                                    .map(|task| {
                                        task.checklist
                                            .iter()
                                            .map(|item| {
                                                let mut copy = item.clone();
                                                copy.completed = false;
                                                copy
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();
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
                let mut auto_done: Vec<ProjectId> = Vec::new();
                if completed {
                    for project_id in completed_projects {
                        let Some(project) = db.projects.get(&project_id) else {
                            continue;
                        };
                        if !project.status.is_open()
                            || !project.complete_with_last_action
                            || project.effective_kind() == ProjectKind::SingleActions
                        {
                            continue;
                        }
                        let (done, total) = db.project_progress(&project_id);
                        if total == 0 || done != total {
                            continue;
                        }
                        let auto_archive = db.settings.auto_archive_closed_projects;
                        if let Some(project) = db.projects.get_mut(&project_id) {
                            project.status = ProjectStatus::Done;
                            project.completed_at = Some(at);
                            project.dropped_at = None;
                            project.archived = Some(auto_archive);
                            project.modified_at = at;
                        }
                        outcome.affected.push(project_id.to_string());
                        auto_done.push(project_id);
                    }
                }
                db.touch();
                let verb = if completed { "Completed" } else { "Reopened" };
                let mut message = if changed == 1 {
                    format!(
                        "{verb} task “{}”",
                        first_title.unwrap_or_else(|| "task".to_string())
                    )
                } else if changed > 1 {
                    format!("{verb} {}", task_count(changed))
                } else {
                    "Nothing to change".to_string()
                };
                if let Some(project_id) = auto_done.first() {
                    let name = db
                        .projects
                        .get(project_id)
                        .map(|project| project.name.clone())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        message.push_str(&format!(" · project “{name}” done"));
                        if db.settings.auto_archive_closed_projects {
                            message.push_str(" and archived");
                        }
                    }
                }
                outcome.message = Some(message);
            }
            Command::DeleteTasks { ids } => {
                let trashed = trash_tasks(db, &ids, at);
                outcome.message = Some(format!("Moved {} to Trash", task_count(trashed.len())));
                db.touch();
            }
            Command::DeleteTaskSeries { ids } => {
                let mut members: Vec<TaskId> = Vec::new();
                for id in &ids {
                    for member in db.series_task_ids(id) {
                        let already_trashed = db
                            .tasks
                            .get(&member)
                            .is_some_and(|task| task.trashed_at.is_some());
                        if !already_trashed && !members.contains(&member) {
                            members.push(member);
                        }
                    }
                }
                let trashed = trash_tasks(db, &members, at);
                outcome.message = Some(format!(
                    "Moved {} in the series to Trash",
                    task_count(trashed.len())
                ));
                db.touch();
            }
            Command::RestoreTasks { ids } => {
                let mut restored: BTreeSet<TaskId> = BTreeSet::new();
                for id in &ids {
                    if db.tasks.contains_key(id) {
                        for descendant in db.task_subtree(id) {
                            restored.insert(descendant);
                        }
                    }
                }
                for id in &restored {
                    if let Some(task) = db.tasks.get_mut(id) {
                        task.trashed_at = None;
                        task.modified_at = at;
                    }
                }
                outcome.message = Some(format!("Restored {}", task_count(restored.len())));
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
            }
            Command::PurgeTasks { ids } => {
                let removed = purge_tasks(db, &ids);
                outcome.message = Some(format!("Permanently deleted {}", task_count(removed)));
                db.touch();
            }
            Command::PurgeExpiredTrash { before } => {
                let ids: Vec<TaskId> = db
                    .tasks
                    .values()
                    .filter(|task| task.trashed_at.is_some_and(|at| at < before))
                    .map(|task| task.id.clone())
                    .collect();
                let removed = purge_tasks(db, &ids);
                outcome.message = Some(if removed == 0 {
                    "No expired tasks to empty".to_string()
                } else {
                    format!("Emptied {} from Trash", task_count(removed))
                });
                db.touch();
            }
            Command::ConvertOneOffToSubtasks { id } => {
                let created = convert_one_off_to_subtasks(db, &id, at)?;
                outcome
                    .created
                    .extend(created.iter().map(|id| id.to_string()));
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(if created.is_empty() {
                    "Switched to subtasks".to_string()
                } else {
                    format!("Restored {} as subtasks", task_count(created.len()))
                });
            }
            Command::SetTaskStepMode {
                id,
                sequential,
                steps_one_off,
            } => {
                let current_one_off = db.task(&id)?.steps_one_off;
                let mut converted = 0usize;
                if steps_one_off && !current_one_off {
                    converted = convert_subtasks_to_one_off(db, &id, at)?;
                } else if !steps_one_off && current_one_off {
                    let created = convert_one_off_to_subtasks(db, &id, at)?;
                    converted = created.len();
                    outcome
                        .created
                        .extend(created.iter().map(|id| id.to_string()));
                }
                {
                    let task = db.task_mut(&id)?;
                    task.sequential = sequential && !steps_one_off;
                    task.steps_one_off = steps_one_off;
                    task.modified_at = at;
                }
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(match (steps_one_off, converted) {
                    (true, 0) => "Switched to one-off steps".to_string(),
                    (true, count) => {
                        format!("Converted {} to one-off steps", task_count(count))
                    }
                    (false, 0) if sequential => "Switched to sequential".to_string(),
                    (false, 0) => "Switched to parallel".to_string(),
                    (false, count) => {
                        format!("Restored {} as subtasks", task_count(count))
                    }
                });
            }
            Command::ConvertSubtasksToOneOff { id } => {
                let count = convert_subtasks_to_one_off(db, &id, at)?;
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(if count == 0 {
                    "Switched to one-off steps".to_string()
                } else {
                    format!("Converted {} to one-off steps", task_count(count))
                });
            }
            Command::ConvertTaskToProject { id, folder } => {
                let task = db.task(&id)?.clone();
                let project_folder = folder.clone().or_else(|| {
                    task.project
                        .as_ref()
                        .and_then(|project| db.projects.get(project))
                        .and_then(|project| project.folder.clone())
                });
                if let Some(folder) = &project_folder {
                    db.folder(folder)?;
                }
                ensure_unique_project_name(db, project_folder.as_ref(), &task.title, None)?;
                let mut project = Project::new(task.title.clone());
                project.notes = task.notes.clone();
                project.folder = project_folder.clone();
                project.tags = task.tags.clone();
                project.due = task.due;
                project.defer = task.defer;
                project.icon = IconSpec::default();
                project.attachments = clean_attachments(task.attachments.clone());
                if task.sequential {
                    project.kind = ProjectKind::Sequential;
                    project.sequential = true;
                }
                project.modified_at = at;
                // The task's top-level subtasks become the project's tasks.
                project.task_order = task.child_order.clone();
                for child in &task.child_order {
                    if let Some(child) = db.tasks.get_mut(child) {
                        child.parent = None;
                        child.project = Some(project.id.clone());
                        child.modified_at = at;
                    }
                }
                let project_id = project.id.clone();
                db.projects.insert(project_id.clone(), project);
                db.insert_container(
                    project_folder,
                    ContainerId::Project(project_id.clone()),
                    ContainerPosition::End,
                );
                // Projects have no checklist, so any remaining items become
                // tasks instead of silently disappearing.
                for item in &task.checklist {
                    let mut step = Task::new(item.title.clone());
                    step.project = Some(project_id.clone());
                    step.completed = item.completed;
                    if item.completed {
                        step.completed_at = Some(at);
                    }
                    step.modified_at = at;
                    let step_id = step.id.clone();
                    db.tasks.insert(step_id.clone(), step);
                    if let Some(project) = db.projects.get_mut(&project_id) {
                        project.task_order.push(step_id);
                    }
                }
                db.detach_task(&id);
                db.tasks.remove(&id);
                outcome.created.push(project_id.to_string());
                let name = db
                    .projects
                    .get(&project_id)
                    .map(|project| project.name.clone())
                    .unwrap_or_default();
                outcome.message = Some(if name.is_empty() {
                    "Converted to a project".to_string()
                } else {
                    format!("Converted to project “{name}”")
                });
                db.touch();
            }
            Command::MoveTasks { ids, placement } => {
                move_tasks(db, &ids, placement)?;
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
                outcome.message = Some(format!("Moved {}", task_count(ids.len())));
            }
            Command::MoveTasksAdjusted { ids, placement } => {
                let conflicts = date_conflicts_for_move(db, &ids, &placement);
                for conflict in &conflicts {
                    let task = db.task_mut(&conflict.task)?;
                    if let Some((_, to)) = conflict.due {
                        task.due = Some(to);
                    }
                    if let Some((_, to)) = conflict.defer {
                        task.defer = Some(to);
                    }
                }
                move_tasks(db, &ids, placement)?;
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
                outcome.message = Some(if conflicts.is_empty() {
                    format!("Moved {}", task_count(ids.len()))
                } else {
                    format!(
                        "Moved {}; adjusted {} date(s)",
                        task_count(ids.len()),
                        conflicts.len()
                    )
                });
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
                outcome.message = Some(format!("Tagged {}", task_count(ids.len())));
            }
            Command::SetTaskFlag { ids, flagged } => {
                for id in &ids {
                    let task = db.task_mut(id)?;
                    task.flagged = flagged;
                    task.modified_at = at;
                }
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                db.touch();
                outcome.message = Some(format!(
                    "{} {}",
                    if flagged { "Flagged" } else { "Unflagged" },
                    task_count(ids.len())
                ));
            }
            Command::SetTaskPriority {
                ids,
                priority,
                series,
            } => {
                let mut targets: Vec<TaskId> = Vec::new();
                for id in &ids {
                    let members = if series {
                        db.series_task_ids(id)
                    } else {
                        vec![id.clone()]
                    };
                    for member in members {
                        if !targets.contains(&member) {
                            targets.push(member);
                        }
                    }
                }
                for id in &targets {
                    let task = db.task_mut(id)?;
                    task.priority = priority;
                    task.modified_at = at;
                }
                outcome
                    .affected
                    .extend(targets.iter().map(|id| id.to_string()));
                db.touch();
                outcome.message = Some(match priority {
                    Some(priority) => format!(
                        "Priority set to {} for {}{}",
                        priority.label().to_lowercase(),
                        task_count(targets.len()),
                        if series { " in the series" } else { "" }
                    ),
                    None => format!(
                        "Priority cleared for {}{}",
                        task_count(targets.len()),
                        if series { " in the series" } else { "" }
                    ),
                });
            }
            Command::AddChecklistItem {
                task: task_id,
                title,
                position,
            } => {
                let title = title.trim();
                if title.is_empty() {
                    return Err(Error::invalid("checklist item cannot be empty"));
                }
                let task = db.task_mut(&task_id)?;
                let item = ChecklistItem::new(title);
                let index = position.unwrap_or(task.checklist.len());
                let index = index.min(task.checklist.len());
                task.checklist.insert(index, item.clone());
                task.modified_at = at;
                outcome.created.push(item.id.to_string());
                outcome.affected.push(task_id.to_string());
                db.touch();
                outcome.message = Some(format!("Checklist step “{title}” added"));
            }
            Command::UpdateChecklistItem {
                task: task_id,
                item,
                title,
                completed,
            } => {
                let title = title.trim();
                if title.is_empty() {
                    return Err(Error::invalid("checklist item cannot be empty"));
                }
                let task = db.task_mut(&task_id)?;
                let Some(item) = task.checklist.iter_mut().find(|entry| entry.id == item) else {
                    return Err(Error::invalid("checklist item not found"));
                };
                item.title = title.to_string();
                if let Some(completed) = completed {
                    item.completed = completed;
                }
                task.modified_at = at;
                outcome.affected.push(task_id.to_string());
                db.touch();
                outcome.message = Some(match completed {
                    Some(true) => format!("Checklist step “{title}” completed"),
                    Some(false) => format!("Checklist step “{title}” reopened"),
                    None => format!("Checklist step “{title}” updated"),
                });
            }
            Command::MoveChecklistItem {
                task: task_id,
                item,
                to,
            } => {
                let task = db.task_mut(&task_id)?;
                let Some(index) = task.checklist.iter().position(|entry| entry.id == item) else {
                    return Err(Error::invalid("checklist item not found"));
                };
                let entry = task.checklist.remove(index);
                let target = to.min(task.checklist.len());
                task.checklist.insert(target, entry);
                task.modified_at = at;
                outcome.affected.push(task_id.to_string());
                db.touch();
                outcome.message = Some("Checklist step reordered".to_string());
            }
            Command::DeleteChecklistItem {
                task: task_id,
                item,
            } => {
                let task = db.task_mut(&task_id)?;
                let before = task.checklist.len();
                task.checklist.retain(|entry| entry.id != item);
                if task.checklist.len() == before {
                    return Err(Error::invalid("checklist item not found"));
                }
                task.modified_at = at;
                outcome.affected.push(task_id.to_string());
                db.touch();
                outcome.message = Some("Checklist step deleted".to_string());
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
                project.planned = new_project.planned;
                ensure_dates_in_order(project.due, project.defer)?;
                project.review_interval_days = new_project
                    .review_interval_days
                    .unwrap_or(db.settings.default_review_days)
                    .max(1);
                project.complete_with_last_action = new_project.complete_with_last_action;
                project.next_review = new_project.next_review.unwrap_or_else(|| {
                    local_date(at) + chrono::Duration::days(project.review_interval_days as i64)
                });
                // `sequential: true` is the older way to ask for a sequential
                // project, and must keep working for callers that never set
                // `kind`.
                project.kind =
                    if new_project.sequential && new_project.kind == ProjectKind::Parallel {
                        ProjectKind::Sequential
                    } else {
                        new_project.kind
                    };
                project.sequential = project.kind == ProjectKind::Sequential;
                project.icon = new_project.icon;
                project.attachments = clean_attachments(new_project.attachments);
                project.fullscreen_reminder = new_project.fullscreen_reminder;
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
                outcome.message = Some(format!("Project “{name}” added"));
            }
            Command::UpdateProjectAdjusted { id, patch } => {
                Command::UpdateProject {
                    id: id.clone(),
                    patch,
                }
                .apply(db, at)?;
                let adjusted = db.clamp_tasks_in_project(&id);
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(if adjusted > 0 {
                    format!("Project updated; adjusted {} task date(s)", adjusted)
                } else {
                    "Project updated".to_string()
                });
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
                    if let Some(flagged) = patch.flagged {
                        project.flagged = flagged;
                    }
                    if let Some(due) = patch.due {
                        project.due = due;
                    }
                    if let Some(defer) = patch.defer {
                        project.defer = defer;
                    }
                    if let Some(planned) = patch.planned {
                        project.planned = planned;
                    }
                    ensure_dates_in_order(project.due, project.defer)?;
                    if let Some(complete) = patch.complete_with_last_action {
                        project.complete_with_last_action = complete;
                    }
                    if let Some(archived) = patch.archived {
                        project.archived = Some(archived);
                    }
                    if let Some(interval) = patch.review_interval_days {
                        project.review_interval_days = interval.max(1);
                        // Changing the cadence moves the next review with it
                        // unless the caller also supplied an explicit date.
                        if patch.next_review.is_none() {
                            let base = project
                                .last_reviewed
                                .unwrap_or_else(|| local_date(at));
                            project.next_review = base
                                .checked_add_signed(chrono::Duration::days(
                                    project.review_interval_days as i64,
                                ))
                                .unwrap_or(base);
                        }
                    }
                    if let Some(review) = patch.next_review {
                        project.next_review = review;
                    }
                    if let Some(icon) = patch.icon {
                        project.icon = icon;
                    }
                    if let Some(attachments) = patch.attachments {
                        project.attachments = clean_attachments(attachments);
                    }
                    if let Some(alerts) = patch.alerts {
                        project.alerts = alerts;
                    }
                    if let Some(removed) = patch.removed_notifications {
                        project.removed_notifications = removed;
                    }
                    if let Some(custom) = patch.custom_notifications {
                        project.custom_notifications = custom;
                    }
                    if let Some(fullscreen) = patch.fullscreen_reminder {
                        project.fullscreen_reminder = fullscreen;
                    }
                    if let Some(kind) = patch.kind {
                        project.kind = kind;
                        project.sequential = kind == ProjectKind::Sequential;
                    } else if let Some(sequential) = patch.sequential {
                        project.sequential = sequential;
                        project.kind = if sequential {
                            ProjectKind::Sequential
                        } else {
                            ProjectKind::Parallel
                        };
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
                let name = db
                    .projects
                    .get(&id)
                    .map(|project| project.name.clone())
                    .unwrap_or_default();
                outcome.message = Some(if name.is_empty() {
                    "Project updated".to_string()
                } else {
                    format!("Project “{name}” updated")
                });
            }
            Command::SetProjectStatus { id, status } => {
                let previous = db.project(&id)?.status;
                let currently_archived = db.project(&id)?.is_archived();
                if previous == status && !(status.is_open() && currently_archived) {
                    return Ok(outcome);
                }
                let auto_archive = db.settings.auto_archive_closed_projects;
                {
                    let project = db.project_mut(&id)?;
                    project.status = status;
                    project.modified_at = at;
                    match status {
                        ProjectStatus::Done => {
                            project.completed_at = Some(at);
                            project.dropped_at = None;
                            project.archived = Some(auto_archive);
                        }
                        ProjectStatus::Dropped => {
                            project.dropped_at = Some(at);
                            project.completed_at = None;
                            project.archived = Some(auto_archive);
                        }
                        ProjectStatus::Active | ProjectStatus::OnHold => {
                            project.completed_at = None;
                            project.dropped_at = None;
                            project.archived = Some(false);
                        }
                    }
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
                let name = db
                    .projects
                    .get(&id)
                    .map(|project| project.name.clone())
                    .unwrap_or_default();
                let archived_note = if status.is_closed() && auto_archive {
                    " and archived"
                } else {
                    ""
                };
                outcome.message = Some(if name.is_empty() {
                    format!(
                        "Project marked {}{archived_note}",
                        status.label().to_lowercase()
                    )
                } else {
                    format!(
                        "Project “{name}” marked {}{archived_note}",
                        status.label().to_lowercase()
                    )
                });
            }
            Command::CompleteReview { id, reviewed_on } => {
                let interval = db.project(&id)?.review_interval_days.max(1) as i64;
                let project = db.project_mut(&id)?;
                project.next_review = reviewed_on + chrono::Duration::days(interval);
                project.last_reviewed = Some(reviewed_on);
                project.modified_at = at;
                db.touch();
                outcome.affected.push(id.to_string());
                let name = db
                    .projects
                    .get(&id)
                    .map(|project| project.name.clone())
                    .unwrap_or_default();
                outcome.message = Some(if name.is_empty() {
                    "Project reviewed".to_string()
                } else {
                    format!("Project “{name}” reviewed")
                });
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
                let name = db
                    .projects
                    .get(&id)
                    .map(|project| project.name.clone())
                    .unwrap_or_default();
                outcome.message = Some(if name.is_empty() {
                    "Project moved".to_string()
                } else {
                    format!("Project “{name}” moved")
                });
            }
            Command::DeleteProject { id } => {
                let name = db.project(&id)?.name.clone();
                let trashed = trash_project(db, &id, at);
                db.touch();
                outcome.message = Some(if trashed == 0 {
                    format!("Project “{name}” deleted")
                } else {
                    format!(
                        "Project “{name}” deleted; {} moved to Trash",
                        task_count(trashed)
                    )
                });
            }
            Command::CreateFolder {
                name,
                parent,
                position,
                fullscreen_reminder,
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
                folder.fullscreen_reminder = fullscreen_reminder;
                let id = folder.id.clone();
                db.folders.insert(id.clone(), folder);
                db.insert_container(parent, ContainerId::Folder(id.clone()), position);
                db.touch();
                outcome.created.push(id.to_string());
                outcome.message = Some(format!("Folder “{name}” added"));
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
                outcome.message = Some(format!("Folder renamed to “{name}”"));
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
                let name = db
                    .folders
                    .get(&id)
                    .map(|folder| folder.name.clone())
                    .unwrap_or_default();
                outcome.message = Some(if name.is_empty() {
                    "Folder moved".to_string()
                } else {
                    format!("Folder “{name}” moved")
                });
            }
            Command::DeleteFolder { id } => {
                db.folder(&id)?;
                // A folder owns everything below it, so deleting one takes its
                // nested folders and their projects with it. Tasks inside those
                // projects move to Trash, the same as deleting a project on its
                // own. The store snapshots before the command runs, so Undo
                // restores the whole tree and its tasks in one step.
                let mut folders = db.folder_descendants(&id);
                folders.push(id.clone());
                let projects: Vec<ProjectId> = db
                    .projects
                    .values()
                    .filter(|project| {
                        project
                            .folder
                            .as_ref()
                            .is_some_and(|folder| folders.contains(folder))
                    })
                    .map(|project| project.id.clone())
                    .collect();
                let mut trashed = 0;
                for project in &projects {
                    trashed += trash_project(db, project, at);
                }
                remove_folder_tree(db, &id);
                db.touch();
                outcome.message = Some(if projects.is_empty() && trashed == 0 {
                    "Folder deleted".to_string()
                } else {
                    format!(
                        "Folder deleted; {} and {} moved to Trash",
                        project_count(projects.len()),
                        task_count(trashed)
                    )
                });
            }
            Command::SetProjectStatuses { ids, status } => {
                for id in ids.iter() {
                    Command::SetProjectStatus {
                        id: id.clone(),
                        status,
                    }
                    .apply(db, at)?;
                }
                outcome.affected.extend(ids.iter().map(|id| id.to_string()));
                outcome.message = Some(format!(
                    "{} marked {}",
                    item_count(ids.len(), "project", "projects"),
                    status.label().to_lowercase()
                ));
            }
            Command::ArchiveProjects { ids, archived } => {
                let mut changed = 0usize;
                for id in ids {
                    let Some(project) = db.projects.get_mut(&id) else {
                        continue;
                    };
                    if project.is_archived() == archived {
                        continue;
                    }
                    project.archived = Some(archived);
                    project.modified_at = at;
                    changed += 1;
                    outcome.affected.push(id.to_string());
                }
                if changed > 0 {
                    db.touch();
                }
                outcome.message = Some(format!(
                    "{} {}",
                    if archived { "Archived" } else { "Unarchived" },
                    item_count(changed, "project", "projects")
                ));
            }
            Command::DeleteContainers { projects, folders } => {
                let selected_folders: Vec<FolderId> = folders
                    .iter()
                    .filter(|id| db.folders.contains_key(*id))
                    .cloned()
                    .collect();
                let mut folder_ids = BTreeSet::new();
                for folder in &selected_folders {
                    folder_ids.insert(folder.clone());
                    folder_ids.extend(db.folder_descendants(folder));
                }
                let mut project_ids: BTreeSet<ProjectId> = projects
                    .iter()
                    .filter(|id| db.projects.contains_key(*id))
                    .cloned()
                    .collect();
                for project in db.projects.values() {
                    if project
                        .folder
                        .as_ref()
                        .is_some_and(|folder| folder_ids.contains(folder))
                    {
                        project_ids.insert(project.id.clone());
                    }
                }
                let mut trashed = 0;
                for project in &project_ids {
                    trashed += trash_project(db, project, at);
                }
                for folder in &selected_folders {
                    if db.folders.contains_key(folder) {
                        remove_folder_tree(db, folder);
                    }
                }
                db.touch();
                outcome
                    .affected
                    .extend(project_ids.iter().map(|id| id.to_string()));
                outcome
                    .affected
                    .extend(selected_folders.iter().map(|id| id.to_string()));
                outcome.message = Some(if trashed == 0 {
                    format!(
                        "{} deleted",
                        item_count(project_ids.len() + selected_folders.len(), "item", "items")
                    )
                } else {
                    format!(
                        "{} deleted; {} moved to Trash",
                        item_count(project_ids.len() + selected_folders.len(), "item", "items"),
                        task_count(trashed)
                    )
                });
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
                outcome.message = Some(format!("Tag “{name}” added"));
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
                    match &old_parent {
                        Some(parent) if Some(parent) != new_parent.as_ref() => {
                            if let Some(old) = db.tags.get_mut(parent) {
                                old.child_order.retain(|child| child != &id);
                            }
                        }
                        None if new_parent.is_some() => {
                            db.tag_roots.retain(|child| child != &id);
                        }
                        _ => {}
                    }
                    db.tag_mut(&id)?.parent = new_parent;
                }
                db.touch();
                outcome.affected.push(id.to_string());
                let name = db
                    .tags
                    .get(&id)
                    .map(|tag| tag.name.clone())
                    .unwrap_or_default();
                outcome.message = Some(if name.is_empty() {
                    "Tag updated".to_string()
                } else {
                    format!("Tag “{name}” updated")
                });
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
                outcome.message = Some(format!("Tag “{}” deleted", tag.name));
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
                let name = perspective.name.clone();
                db.perspectives.insert(id.clone(), perspective);
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(format!("Perspective “{name}” saved"));
            }
            Command::DeletePerspective { id } => {
                let name = db
                    .perspectives
                    .get(&id)
                    .map(|perspective| perspective.name.clone())
                    .unwrap_or_default();
                db.perspectives.remove(&id);
                db.touch();
                outcome.affected.push(id.to_string());
                outcome.message = Some(if name.is_empty() {
                    "Perspective deleted".to_string()
                } else {
                    format!("Perspective “{name}” deleted")
                });
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
                if let Some(value) = patch.trash_retention_days {
                    settings.trash_retention_days = value.min(3650);
                }
                if let Some(value) = patch.changed_days {
                    settings.changed_days = value.clamp(1, 3650);
                }
                if let Some(value) = patch.due_soon_days {
                    settings.due_soon_days = value.min(3650);
                }
                if let Some(value) = patch.forecast_due_counts {
                    settings.forecast_due_counts = value;
                }
                if let Some(tag) = patch.forecast_tag {
                    settings.forecast_tag = tag;
                }
                if let Some(value) = patch.forecast_flagged {
                    settings.forecast_flagged = value;
                }
                if let Some(value) = patch.default_defer_notification {
                    settings.default_defer_notification = value;
                }
                if let Some(value) = patch.default_planned_notification {
                    settings.default_planned_notification = value;
                }
                if let Some(value) = patch.default_latest_start_notification {
                    settings.default_latest_start_notification = value;
                }
                if let Some(value) = patch.default_due_notification {
                    settings.default_due_notification = value;
                }
                if let Some(value) = patch.default_custom_notification_time {
                    settings.default_custom_notification_time = value;
                }
                if let Some(value) = patch.auto_archive_closed_projects {
                    settings.auto_archive_closed_projects = value;
                }
                if let Some(sidebar) = patch.sidebar {
                    settings.sidebar = sidebar;
                }
                db.touch();
                outcome.message = Some("Settings updated".to_string());
            }
            Command::Import { db: imported, mode } => {
                imported.validate().map_err(|error| {
                    Error::Import(format!("imported library is invalid: {error}"))
                })?;
                match mode {
                    ImportMode::Replace => {
                        *db = imported;
                        db.touch();
                        outcome.message = Some("Library imported".to_string());
                    }
                    ImportMode::Merge => {
                        let stats = merge_db(db, imported);
                        outcome.message = Some(format!(
                            "Library merged: {}, {} and {} added",
                            task_count(stats.tasks),
                            project_count(stats.projects),
                            tag_count(stats.tags)
                        ));
                    }
                }
            }
        }
        Ok(outcome)
    }
}

/// Remove a folder and every folder nested inside it, children first so each
/// detach can still read its parent's child list.
fn remove_folder_tree(db: &mut Db, id: &FolderId) {
    let children = db
        .folders
        .get(id)
        .map(|folder| folder.child_order.clone())
        .unwrap_or_default();
    for child in children {
        if let ContainerId::Folder(child) = child {
            remove_folder_tree(db, &child);
        }
    }
    db.detach_container(&ContainerId::Folder(id.clone()));
    db.folders.remove(id);
}

/// Hourly and minute repeats advance the clock, so the date they schedule from
/// must carry a time. A date-only task is refused rather than promoted to a
/// guessed time.
fn ensure_repeat_has_clock(
    rule: Option<&RepeatRule>,
    due: Option<Stamp>,
    defer: Option<Stamp>,
) -> Result<()> {
    let Some(rule) = rule.filter(|rule| rule.uses_sub_day()) else {
        return Ok(());
    };
    let anchor = match rule.mode {
        RepeatMode::DueAgain => due.or(defer),
        RepeatMode::DeferAnother => defer.or(due),
    };
    if anchor.is_some_and(|stamp| !stamp.has_time) {
        return Err(Error::invalid("Add a time first"));
    }
    Ok(())
}

/// Move the given tasks and every task below them to Trash. Both plain
/// A task cannot become available after it is already due. Comparing the
/// local calendar dates keeps same-day defer/due pairs valid even when one side
/// is all-day and the other carries a time.
fn ensure_dates_in_order(due: Option<Stamp>, defer: Option<Stamp>) -> Result<()> {
    if let (Some(due), Some(defer)) = (due, defer) {
        if defer.local_date() > due.local_date() {
            return Err(Error::invalid(
                "Defer date cannot be later than the due date",
            ));
        }
    }
    Ok(())
}

/// deletion and whole-series deletion use this so their snapshot is identical.
fn trash_tasks(db: &mut Db, ids: &[TaskId], at: DateTime<Utc>) -> BTreeSet<TaskId> {
    let mut trashed: BTreeSet<TaskId> = BTreeSet::new();
    for id in ids {
        if db.tasks.contains_key(id) {
            for descendant in db.task_subtree(id) {
                trashed.insert(descendant);
            }
        }
    }
    for id in &trashed {
        if let Some(task) = db.tasks.get_mut(id) {
            task.trashed_at = Some(at);
            task.modified_at = at;
        }
    }
    trashed
}

/// Move a project's whole task tree to Trash and drop the project itself.
/// Returns how many tasks were trashed. Deleting one project and the folder
/// cascade share this so both keep the trashed roots reachable from the Inbox.
fn trash_project(db: &mut Db, id: &ProjectId, at: DateTime<Utc>) -> usize {
    let roots: Vec<TaskId> = db
        .tasks
        .values()
        .filter(|task| task.project.as_ref() == Some(id) && task.parent.is_none())
        .map(|task| task.id.clone())
        .collect();
    let mut trashed: BTreeSet<TaskId> = BTreeSet::new();
    for root in &roots {
        for descendant in db.task_subtree(root) {
            trashed.insert(descendant);
        }
    }
    for task_id in &trashed {
        if let Some(task) = db.tasks.get_mut(task_id) {
            task.trashed_at = Some(at);
            task.modified_at = at;
        }
    }
    // The project's ordering list disappears with it. Keep the trashed roots
    // addressable from the Inbox so validation and an Undo can both find them.
    for root in roots {
        if let Some(task) = db.tasks.get_mut(&root) {
            task.project = None;
        }
        if !db.inbox_order.contains(&root) {
            db.inbox_order.push(root);
        }
    }
    db.detach_container(&ContainerId::Project(id.clone()));
    db.projects.remove(id);
    trashed.len()
}

/// Attachment paths are user input: trim them and drop empty lines so a blank
/// row in the inspector never becomes an empty path in the library.
fn clean_attachments(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

/// Stored task dates that applying this patch would have to change.
pub fn date_conflicts_for_task_patch(db: &Db, id: &TaskId, patch: &TaskPatch) -> Vec<DateConflict> {
    let Some(task) = db.tasks.get(id) else {
        return Vec::new();
    };
    let due = patch.due.unwrap_or(task.due);
    let defer = patch.defer.unwrap_or(task.defer);
    let parent = patch.parent.clone().unwrap_or(task.parent.clone());
    let project = match &patch.project_path {
        Some(Some(path)) => db
            .find_project_by_path(path)
            .or_else(|| db.find_project_by_name(path)),
        Some(None) => None,
        None => patch.project.clone().unwrap_or(task.project.clone()),
    };
    let container = if let Some(parent) = &parent {
        db.tasks
            .get(parent)
            .map(|parent| (parent.title.clone(), true))
    } else if let Some(project) = &project {
        Some((db.project_path(project), false))
    } else {
        None
    };
    let (container_due, container_defer) = if let Some(parent) = parent {
        (db.effective_due(&parent), db.effective_defer(&parent))
    } else if let Some(project) = project {
        db.projects
            .get(&project)
            .map(|project| (project.due, project.defer))
            .unwrap_or((None, None))
    } else {
        (None, None)
    };
    let due_change = match (due, container_due) {
        (Some(from), Some(to)) if from.local_date() > to.local_date() => Some((from, to)),
        _ => None,
    };
    let defer_change = match (defer, container_defer) {
        (Some(from), Some(to)) if from.local_date() < to.local_date() => Some((from, to)),
        _ => None,
    };
    if due_change.is_none() && defer_change.is_none() {
        Vec::new()
    } else {
        vec![DateConflict {
            task: id.clone(),
            title: task.title.clone(),
            container,
            due: due_change,
            defer: defer_change,
        }]
    }
}

/// Tasks that a project-date patch would have to clamp.
pub fn date_conflicts_for_project_patch(
    db: &Db,
    id: &ProjectId,
    patch: &ProjectPatch,
) -> Vec<DateConflict> {
    let Some(project) = db.projects.get(id) else {
        return Vec::new();
    };
    let container = Some((db.project_path(id), false));
    let project_due = patch.due.unwrap_or(project.due);
    let project_defer = patch.defer.unwrap_or(project.defer);
    let mut out = Vec::new();
    for task in db.tasks.values() {
        if task.project.as_ref() != Some(id) || task.completed || task.is_trashed() {
            continue;
        }
        let mut container_due = project_due;
        let mut container_defer = project_defer;
        let mut parent = task.parent.clone();
        let mut guard = 0;
        while let Some(parent_id) = parent {
            guard += 1;
            if guard > 64 {
                break;
            }
            let Some(parent_task) = db.tasks.get(&parent_id) else {
                break;
            };
            if let Some(due) = parent_task.due {
                container_due = match container_due {
                    Some(current) if current.at <= due.at => Some(current),
                    _ => Some(due),
                };
            }
            if let Some(defer) = parent_task.defer {
                container_defer = match container_defer {
                    Some(current) if current.at >= defer.at => Some(current),
                    _ => Some(defer),
                };
            }
            parent = parent_task.parent.clone();
        }
        let due = match (task.due, container_due) {
            (Some(from), Some(to)) if from.local_date() > to.local_date() => Some((from, to)),
            _ => None,
        };
        let defer = match (task.defer, container_defer) {
            (Some(from), Some(to)) if from.local_date() < to.local_date() => Some((from, to)),
            _ => None,
        };
        if due.is_some() || defer.is_some() {
            out.push(DateConflict {
                task: task.id.clone(),
                title: task.title.clone(),
                container: container.clone(),
                due,
                defer,
            });
        }
    }
    out
}

/// Stored task dates that the requested move would have to change.
pub fn date_conflicts_for_move(
    db: &Db,
    ids: &[TaskId],
    placement: &TaskPlacement,
) -> Vec<DateConflict> {
    let (container_due, container_defer, container) = match placement {
        TaskPlacement::Inbox(_) => (None, None, None),
        TaskPlacement::Project { project, .. } => {
            let (due, defer) = db
                .projects
                .get(project)
                .map(|project| (project.due, project.defer))
                .unwrap_or((None, None));
            (due, defer, Some((db.project_path(project), false)))
        }
        TaskPlacement::Parent { parent, .. } => (
            db.effective_due(parent),
            db.effective_defer(parent),
            db.tasks
                .get(parent)
                .map(|parent| (parent.title.clone(), true)),
        ),
    };
    let mut out = Vec::new();
    for id in ids {
        let Some(task) = db.tasks.get(id) else {
            continue;
        };
        if task.completed || task.is_trashed() {
            continue;
        }
        let due = match (task.due, container_due) {
            (Some(from), Some(to)) if from.local_date() > to.local_date() => Some((from, to)),
            _ => None,
        };
        let defer = match (task.defer, container_defer) {
            (Some(from), Some(to)) if from.local_date() < to.local_date() => Some((from, to)),
            _ => None,
        };
        if due.is_some() || defer.is_some() {
            out.push(DateConflict {
                task: id.clone(),
                title: task.title.clone(),
                container: container.clone(),
                due,
                defer,
            });
        }
    }
    out
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
        if unique
            .iter()
            .any(|other| other != &id && ancestors.contains(other))
        {
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

fn tag_depth(tags: &BTreeMap<TagId, Tag>, id: &TagId) -> usize {
    let mut depth = 0;
    let mut current = tags.get(id).and_then(|tag| tag.parent.as_ref());
    while let Some(parent) = current {
        depth += 1;
        current = tags.get(parent).and_then(|tag| tag.parent.as_ref());
    }
    depth
}

fn tag_path_slash(tags: &BTreeMap<TagId, Tag>, id: &TagId) -> String {
    let mut parts = Vec::new();
    let mut current = Some(id.clone());
    while let Some(id) = current {
        let Some(tag) = tags.get(&id) else {
            break;
        };
        parts.push(tag.name.clone());
        current = tag.parent.clone();
    }
    parts.reverse();
    parts.join("/")
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

    // Reconcile tags by path: merging a library twice must not create a second
    // "Errands" just because the ids differ. New tags inherit the mapped id of
    // their parent so tag trees stay intact.
    let mut tag_map: BTreeMap<TagId, TagId> = BTreeMap::new();
    let mut imported_tags: Vec<&Tag> = imported.tags.values().collect();
    imported_tags.sort_by_key(|tag| tag_depth(&imported.tags, &tag.id));
    for tag in imported_tags {
        if db.tags.contains_key(&tag.id) {
            tag_map.insert(tag.id.clone(), tag.id.clone());
            continue;
        }
        let path = tag_path_slash(&imported.tags, &tag.id);
        if let Some(existing) = db.find_tag_path(&path) {
            tag_map.insert(tag.id.clone(), existing);
            continue;
        }
        let mut copy = tag.clone();
        copy.parent = tag
            .parent
            .as_ref()
            .and_then(|parent| tag_map.get(parent).cloned());
        let new_id = copy.id.clone();
        match copy.parent.clone() {
            Some(parent) => {
                if let Some(entity) = db.tags.get_mut(&parent) {
                    if !entity.child_order.contains(&new_id) {
                        entity.child_order.push(new_id.clone());
                    }
                }
            }
            None => {
                if !db.tag_roots.contains(&new_id) {
                    db.tag_roots.push(new_id.clone());
                }
            }
        }
        db.tags.insert(new_id.clone(), copy);
        tag_map.insert(tag.id.clone(), new_id);
        stats.tags += 1;
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
            let mut copy = task.clone();
            copy.tags = copy
                .tags
                .iter()
                .filter_map(|tag| tag_map.get(tag).cloned())
                .collect();
            db.tasks.insert(id.clone(), copy);
            stats.tasks += 1;
        }
    }

    // Merge ordering arrays by appending unknown ids.
    for container in imported.root {
        if !db.root.contains(&container) {
            db.root.push(container);
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
        let Some(mapped) = tag_map.get(id) else {
            continue;
        };
        if let Some(target) = db.tags.get_mut(mapped) {
            for child in &tag.child_order {
                if let Some(child) = tag_map.get(child) {
                    if !target.child_order.contains(child) {
                        target.child_order.push(child.clone());
                    }
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

/// Replace a task's child tasks with one-off checklist steps. Returns the
/// number of steps created. Nested children are flattened, so task-only
/// details such as notes, dates, tags and priority are dropped.
fn convert_subtasks_to_one_off(db: &mut Db, id: &TaskId, at: DateTime<Utc>) -> Result<usize> {
    db.task(id)?;
    let descendants = db.task_subtree(id);
    let items: Vec<(String, bool)> = descendants
        .iter()
        .skip(1)
        .filter_map(|child_id| {
            db.tasks
                .get(child_id)
                .filter(|task| !task.is_trashed())
                .map(|task| (task.title.clone(), task.completed))
        })
        .collect();
    {
        let task = db.task_mut(id)?;
        for (title, completed) in &items {
            let mut item = ChecklistItem::new(title.clone());
            item.completed = *completed;
            task.checklist.push(item);
        }
        task.steps_one_off = true;
        task.sequential = false;
        task.modified_at = at;
    }
    let children: Vec<TaskId> = descendants.into_iter().skip(1).collect();
    if !children.is_empty() {
        purge_tasks(db, &children);
    }
    Ok(items.len())
}

/// Turn a task's one-off checklist steps back into child tasks. Returns the
/// ids of the created subtasks.
fn convert_one_off_to_subtasks(db: &mut Db, id: &TaskId, at: DateTime<Utc>) -> Result<Vec<TaskId>> {
    let parent = db.task(id)?.clone();
    let mut created = Vec::new();
    for item in parent.checklist.clone() {
        let mut child = Task::new(item.title);
        child.parent = Some(id.clone());
        child.project = parent.project.clone();
        child.completed = item.completed;
        child.completed_at = item.completed.then_some(at);
        child.modified_at = at;
        let child_id = child.id.clone();
        db.tasks.insert(child_id.clone(), child);
        db.insert_task_at(
            OrderKey::Parent(id.clone()),
            child_id.clone(),
            Position::End,
        );
        created.push(child_id);
    }
    let task = db.task_mut(id)?;
    task.checklist.clear();
    task.steps_one_off = false;
    task.modified_at = at;
    Ok(created)
}

/// Remove tasks and their subtrees from every map and order list.
fn purge_tasks(db: &mut Db, ids: &[TaskId]) -> usize {
    let mut removed: BTreeSet<TaskId> = BTreeSet::new();
    for id in ids {
        if db.tasks.contains_key(id) {
            for descendant in db.task_subtree(id) {
                removed.insert(descendant);
            }
        }
    }
    for id in &removed {
        db.detach_task(id);
        db.tasks.remove(id);
    }
    removed.len()
}

fn item_count(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

fn task_count(count: usize) -> String {
    match count {
        1 => "1 task".to_string(),
        n => format!("{n} tasks"),
    }
}

fn project_count(count: usize) -> String {
    match count {
        1 => "1 project".to_string(),
        n => format!("{n} projects"),
    }
}

fn tag_count(count: usize) -> String {
    match count {
        1 => "1 tag".to_string(),
        n => format!("{n} tags"),
    }
}

fn default_true() -> bool {
    true
}
