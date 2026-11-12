//! The normalized task library: entity maps plus explicit ordering arrays.
//!
//! Ordering is stored in parent arrays (`Project::task_order`,
//! `Task::child_order`, `Folder::child_order`, `Db::root`, ...). That makes
//! drag/drop, indentation and reparenting a matter of splicing arrays, and it
//! gives the JSON file a stable, diff-friendly shape.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::error::{Error, Result};
use crate::ids::{ChecklistId, ContainerId, FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use crate::model::{
    Folder, Geo, Perspective, Project, ProjectStatus, Settings, Tag, TagStatus, Task,
};
use crate::time::{Stamp, now};

pub type TaskMap = BTreeMap<TaskId, Task>;
pub type ProjectMap = BTreeMap<ProjectId, Project>;
pub type FolderMap = BTreeMap<FolderId, Folder>;
pub type TagMap = BTreeMap<TagId, Tag>;
pub type PerspectiveMap = BTreeMap<PerspectiveId, Perspective>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Db {
    #[serde(default)]
    pub tasks: TaskMap,
    #[serde(default)]
    pub projects: ProjectMap,
    #[serde(default)]
    pub folders: FolderMap,
    #[serde(default)]
    pub tags: TagMap,
    #[serde(default)]
    pub perspectives: PerspectiveMap,
    /// Projects and folders at the top level of the library.
    #[serde(default)]
    pub root: Vec<ContainerId>,
    /// Top-level tags.
    #[serde(default)]
    pub tag_roots: Vec<TagId>,
    /// Inbox tasks that are not subtasks.
    #[serde(default)]
    pub inbox_order: Vec<TaskId>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default = "now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "now")]
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

/// Where an ordered list of tasks lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderKey {
    Inbox,
    Project(ProjectId),
    Parent(TaskId),
}

/// Insertion point inside a sibling list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Position {
    Start,
    End,
    Before(TaskId),
    After(TaskId),
    Index(usize),
}

/// Insertion point inside a folder's container list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerPosition {
    Start,
    End,
    Before(ContainerId),
    After(ContainerId),
    Index(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerRow {
    pub container: ContainerId,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectTaskRow {
    pub task: TaskId,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagRow {
    pub tag: TagId,
    pub depth: u32,
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

impl Db {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            projects: BTreeMap::new(),
            folders: BTreeMap::new(),
            tags: BTreeMap::new(),
            perspectives: BTreeMap::new(),
            root: Vec::new(),
            tag_roots: Vec::new(),
            inbox_order: Vec::new(),
            settings: Settings::default(),
            created_at: now(),
            modified_at: now(),
        }
    }

    pub fn touch(&mut self) {
        self.modified_at = now();
    }

    // ---------------------------------------------------------------- lookups

    pub fn task(&self, id: &TaskId) -> Result<&Task> {
        self.tasks
            .get(id)
            .ok_or_else(|| Error::not_found(TaskId::label(), id.to_string()))
    }

    pub fn task_mut(&mut self, id: &TaskId) -> Result<&mut Task> {
        self.tasks
            .get_mut(id)
            .ok_or_else(|| Error::not_found(TaskId::label(), id.to_string()))
    }

    pub fn project(&self, id: &ProjectId) -> Result<&Project> {
        self.projects
            .get(id)
            .ok_or_else(|| Error::not_found(ProjectId::label(), id.to_string()))
    }

    pub fn project_mut(&mut self, id: &ProjectId) -> Result<&mut Project> {
        self.projects
            .get_mut(id)
            .ok_or_else(|| Error::not_found(ProjectId::label(), id.to_string()))
    }

    pub fn folder(&self, id: &FolderId) -> Result<&Folder> {
        self.folders
            .get(id)
            .ok_or_else(|| Error::not_found(FolderId::label(), id.to_string()))
    }

    pub fn folder_mut(&mut self, id: &FolderId) -> Result<&mut Folder> {
        self.folders
            .get_mut(id)
            .ok_or_else(|| Error::not_found(FolderId::label(), id.to_string()))
    }

    pub fn tag(&self, id: &TagId) -> Result<&Tag> {
        self.tags
            .get(id)
            .ok_or_else(|| Error::not_found(TagId::label(), id.to_string()))
    }

    pub fn tag_mut(&mut self, id: &TagId) -> Result<&mut Tag> {
        self.tags
            .get_mut(id)
            .ok_or_else(|| Error::not_found(TagId::label(), id.to_string()))
    }

    pub fn perspective(&self, id: &PerspectiveId) -> Result<&Perspective> {
        self.perspectives
            .get(id)
            .ok_or_else(|| Error::not_found(PerspectiveId::label(), id.to_string()))
    }

    fn resolve_ids<'a>(
        prefix: &str,
        kind: &'static str,
        ids: impl Iterator<Item = &'a str>,
    ) -> Result<String> {
        let prefix = prefix.trim();
        if prefix.is_empty() {
            return Err(Error::invalid(format!("empty {kind} id")));
        }
        let matches: Vec<&str> = ids
            .filter(|id| id.starts_with(prefix) || id.ends_with(prefix))
            .collect();
        match matches.as_slice() {
            [] => Err(Error::not_found(kind, prefix.to_string())),
            [single] => Ok((*single).to_string()),
            many => Err(Error::Ambiguous {
                prefix: prefix.to_string(),
                matches: many.len(),
            }),
        }
    }

    pub fn resolve_task(&self, prefix: &str) -> Result<TaskId> {
        Self::resolve_ids(
            prefix,
            TaskId::label(),
            self.tasks.keys().map(|id| id.as_str()),
        )
        .map(TaskId::from)
    }

    pub fn resolve_project(&self, prefix: &str) -> Result<ProjectId> {
        Self::resolve_ids(
            prefix,
            ProjectId::label(),
            self.projects.keys().map(|id| id.as_str()),
        )
        .map(ProjectId::from)
    }

    pub fn resolve_folder(&self, prefix: &str) -> Result<FolderId> {
        Self::resolve_ids(
            prefix,
            FolderId::label(),
            self.folders.keys().map(|id| id.as_str()),
        )
        .map(FolderId::from)
    }

    pub fn resolve_tag(&self, prefix: &str) -> Result<TagId> {
        Self::resolve_ids(
            prefix,
            TagId::label(),
            self.tags.keys().map(|id| id.as_str()),
        )
        .map(TagId::from)
    }

    pub fn resolve_perspective(&self, prefix: &str) -> Result<PerspectiveId> {
        Self::resolve_ids(
            prefix,
            PerspectiveId::label(),
            self.perspectives.keys().map(|id| id.as_str()),
        )
        .map(PerspectiveId::from)
    }

    pub fn resolve_container(&self, prefix: &str) -> Result<ContainerId> {
        let projects: Vec<ContainerId> = self
            .projects
            .keys()
            .filter(|id| id.matches_spec(prefix))
            .map(|id| ContainerId::Project((*id).clone()))
            .collect();
        let folders: Vec<ContainerId> = self
            .folders
            .keys()
            .filter(|id| id.matches_spec(prefix))
            .map(|id| ContainerId::Folder((*id).clone()))
            .collect();
        match projects.len() + folders.len() {
            0 => Err(Error::not_found("container", prefix.to_string())),
            1 => Ok(projects.into_iter().chain(folders).next().unwrap()),
            count => Err(Error::Ambiguous {
                prefix: prefix.to_string(),
                matches: count,
            }),
        }
    }

    // ------------------------------------------------------------- task order

    pub fn order_key_for(&self, task_id: &TaskId) -> Option<OrderKey> {
        let task = self.tasks.get(task_id)?;
        if let Some(parent) = &task.parent {
            Some(OrderKey::Parent(parent.clone()))
        } else if let Some(project) = &task.project {
            Some(OrderKey::Project(project.clone()))
        } else {
            Some(OrderKey::Inbox)
        }
    }

    pub fn ordered_tasks(&self, key: OrderKey) -> &[TaskId] {
        match key {
            OrderKey::Inbox => &self.inbox_order,
            OrderKey::Project(id) => self
                .projects
                .get(&id)
                .map(|project| project.task_order.as_slice())
                .unwrap_or(&[]),
            OrderKey::Parent(id) => self
                .tasks
                .get(&id)
                .map(|task| task.child_order.as_slice())
                .unwrap_or(&[]),
        }
    }

    fn ordered_tasks_mut(&mut self, key: OrderKey) -> Option<&mut Vec<TaskId>> {
        match key {
            OrderKey::Inbox => Some(&mut self.inbox_order),
            OrderKey::Project(id) => self.projects.get_mut(&id).map(|p| &mut p.task_order),
            OrderKey::Parent(id) => self.tasks.get_mut(&id).map(|t| &mut t.child_order),
        }
    }

    pub fn detach_task(&mut self, task_id: &TaskId) {
        if let Some(key) = self.order_key_for(task_id) {
            if let Some(list) = self.ordered_tasks_mut(key) {
                list.retain(|id| id != task_id);
            }
        }
    }

    pub fn insert_task_at(&mut self, key: OrderKey, task_id: TaskId, position: Position) {
        let target = match &position {
            Position::Before(id) => self
                .ordered_tasks(key.clone())
                .iter()
                .position(|candidate| candidate == id)
                .unwrap_or(usize::MAX),
            Position::After(id) => self
                .ordered_tasks(key.clone())
                .iter()
                .position(|candidate| candidate == id)
                .map(|index| index + 1)
                .unwrap_or(usize::MAX),
            Position::Start => 0,
            Position::End => usize::MAX,
            Position::Index(index) => *index,
        };
        if let Some(list) = self.ordered_tasks_mut(key) {
            let index = target.min(list.len());
            list.insert(index, task_id);
        }
    }

    /// Every task id in the subtree rooted at `task_id`, in pre-order.
    pub fn task_subtree(&self, task_id: &TaskId) -> Vec<TaskId> {
        let mut out = Vec::new();
        self.collect_subtree(task_id, &mut out);
        out
    }

    fn collect_subtree(&self, task_id: &TaskId, out: &mut Vec<TaskId>) {
        out.push(task_id.clone());
        if let Some(task) = self.tasks.get(task_id) {
            for child in &task.child_order {
                self.collect_subtree(child, out);
            }
        }
    }

    pub fn task_ancestors(&self, task_id: &TaskId) -> Vec<TaskId> {
        let mut out = Vec::new();
        let mut current = self.tasks.get(task_id).and_then(|task| task.parent.clone());
        while let Some(id) = current {
            out.push(id.clone());
            current = self.tasks.get(&id).and_then(|task| task.parent.clone());
        }
        out
    }

    pub fn effective_project(&self, task_id: &TaskId) -> Option<ProjectId> {
        let mut current = self.tasks.get(task_id).and_then(|task| task.parent.clone());
        while let Some(id) = current {
            if let Some(task) = self.tasks.get(&id) {
                if let Some(project) = &task.project {
                    return Some(project.clone());
                }
                current = task.parent.clone();
            } else {
                break;
            }
        }
        self.tasks
            .get(task_id)
            .and_then(|task| task.project.clone())
    }

    pub fn task_depth(&self, task_id: &TaskId) -> u32 {
        self.task_ancestors(task_id).len() as u32
    }

    /// Flattened project outline, skipping completed subtrees when asked.
    pub fn project_outline(
        &self,
        project: &ProjectId,
        include_completed: bool,
    ) -> Vec<ProjectTaskRow> {
        let mut out = Vec::new();
        let Some(project) = self.projects.get(project) else {
            return out;
        };
        for task_id in &project.task_order {
            self.collect_outline(task_id, 0, include_completed, &mut out);
        }
        out
    }

    fn collect_outline(
        &self,
        task_id: &TaskId,
        depth: u32,
        include_completed: bool,
        out: &mut Vec<ProjectTaskRow>,
    ) {
        let Some(task) = self.tasks.get(task_id) else {
            return;
        };
        if task.completed && !include_completed {
            return;
        }
        out.push(ProjectTaskRow {
            task: task_id.clone(),
            depth,
        });
        for child in &task.child_order {
            self.collect_outline(child, depth + 1, include_completed, out);
        }
    }

    pub fn inbox_outline(&self, include_completed: bool) -> Vec<ProjectTaskRow> {
        let mut out = Vec::new();
        for task_id in &self.inbox_order {
            self.collect_outline(task_id, 0, include_completed, &mut out);
        }
        out
    }

    pub fn tasks_in_project(&self, project: &ProjectId, include_completed: bool) -> Vec<TaskId> {
        self.project_outline(project, include_completed)
            .into_iter()
            .map(|row| row.task)
            .collect()
    }

    pub fn project_progress(&self, project: &ProjectId) -> (usize, usize) {
        let all = self.tasks_in_project(project, true);
        let done = all
            .iter()
            .filter(|id| self.tasks.get(*id).is_some_and(|task| task.completed))
            .count();
        (done, all.len())
    }

    // ------------------------------------------------------------- containers

    pub fn container_children(&self, parent: Option<FolderId>) -> &[ContainerId] {
        match parent {
            None => &self.root,
            Some(folder) => self
                .folders
                .get(&folder)
                .map(|folder| folder.child_order.as_slice())
                .unwrap_or(&[]),
        }
    }

    fn container_children_mut(
        &mut self,
        parent: Option<FolderId>,
    ) -> Option<&mut Vec<ContainerId>> {
        match parent {
            None => Some(&mut self.root),
            Some(folder) => self.folders.get_mut(&folder).map(|f| &mut f.child_order),
        }
    }

    pub fn detach_container(&mut self, container: &ContainerId) {
        let parent = match container {
            ContainerId::Folder(id) => self
                .folders
                .get(id)
                .and_then(|folder| folder.parent.clone()),
            ContainerId::Project(id) => self
                .projects
                .get(id)
                .and_then(|project| project.folder.clone()),
        };
        if let Some(list) = self.container_children_mut(parent) {
            list.retain(|item| item != container);
        }
        match container {
            ContainerId::Folder(id) => {
                if let Some(folder) = self.folders.get_mut(id) {
                    folder.parent = None;
                }
            }
            ContainerId::Project(id) => {
                if let Some(project) = self.projects.get_mut(id) {
                    project.folder = None;
                }
            }
        }
    }

    pub fn insert_container(
        &mut self,
        parent: Option<FolderId>,
        container: ContainerId,
        position: ContainerPosition,
    ) {
        let target = match &position {
            ContainerPosition::Start => 0,
            ContainerPosition::End => usize::MAX,
            ContainerPosition::Index(index) => *index,
            ContainerPosition::Before(id) => self
                .container_children(parent.clone())
                .iter()
                .position(|item| item == id)
                .unwrap_or(usize::MAX),
            ContainerPosition::After(id) => self
                .container_children(parent.clone())
                .iter()
                .position(|item| item == id)
                .map(|index| index + 1)
                .unwrap_or(usize::MAX),
        };
        match &container {
            ContainerId::Folder(id) => {
                if let Some(folder) = self.folders.get_mut(id) {
                    folder.parent = parent.clone();
                }
            }
            ContainerId::Project(id) => {
                if let Some(project) = self.projects.get_mut(id) {
                    project.folder = parent.clone();
                }
            }
        }
        if let Some(list) = self.container_children_mut(parent) {
            let index = target.min(list.len());
            list.insert(index, container);
        }
    }

    pub fn flatten_containers(&self) -> Vec<ContainerRow> {
        let mut out = Vec::new();
        for container in &self.root {
            self.collect_container(container, 0, &mut out);
        }
        out
    }

    fn collect_container(&self, container: &ContainerId, depth: u32, out: &mut Vec<ContainerRow>) {
        out.push(ContainerRow {
            container: container.clone(),
            depth,
        });
        if let ContainerId::Folder(folder_id) = container {
            if let Some(folder) = self.folders.get(folder_id) {
                for child in &folder.child_order {
                    self.collect_container(child, depth + 1, out);
                }
            }
        }
    }

    pub fn folder_descendants(&self, folder: &FolderId) -> Vec<FolderId> {
        let mut out = Vec::new();
        if let Some(folder) = self.folders.get(folder) {
            for child in &folder.child_order {
                if let ContainerId::Folder(id) = child {
                    out.push(id.clone());
                    out.extend(self.folder_descendants(id));
                }
            }
        }
        out
    }

    pub fn project_is_in_folder(
        &self,
        project: &ProjectId,
        folder: &FolderId,
        include_children: bool,
    ) -> bool {
        let Some(project) = self.projects.get(project) else {
            return false;
        };
        let Some(parent) = project.folder.clone() else {
            return false;
        };
        if &parent == folder {
            return true;
        }
        if include_children {
            return self.folder_descendants(folder).contains(&parent);
        }
        false
    }

    pub fn folder_path(&self, folder: &FolderId) -> String {
        let mut parts = Vec::new();
        let mut current = Some(folder.clone());
        while let Some(id) = current {
            if let Some(folder) = self.folders.get(&id) {
                parts.push(folder.name.clone());
                current = folder.parent.clone();
            } else {
                break;
            }
        }
        parts.reverse();
        parts.join(" / ")
    }

    pub fn project_path(&self, project: &ProjectId) -> String {
        let Some(project) = self.projects.get(project) else {
            return String::new();
        };
        match &project.folder {
            Some(folder) => format!("{} / {}", self.folder_path(folder), project.name),
            None => project.name.clone(),
        }
    }

    // ------------------------------------------------------------------- tags

    pub fn tag_roots(&self) -> &[TagId] {
        &self.tag_roots
    }

    pub fn tag_children(&self, tag: &TagId) -> &[TagId] {
        self.tags
            .get(tag)
            .map(|tag| tag.child_order.as_slice())
            .unwrap_or(&[])
    }

    pub fn tag_descendants(&self, tag: &TagId) -> Vec<TagId> {
        let mut out = Vec::new();
        if let Some(tag) = self.tags.get(tag) {
            for child in &tag.child_order {
                out.push(child.clone());
                out.extend(self.tag_descendants(child));
            }
        }
        out
    }

    pub fn tag_ancestors(&self, tag: &TagId) -> Vec<TagId> {
        let mut out = Vec::new();
        let mut current = self.tags.get(tag).and_then(|tag| tag.parent.clone());
        while let Some(id) = current {
            out.push(id.clone());
            current = self.tags.get(&id).and_then(|tag| tag.parent.clone());
        }
        out
    }

    pub fn tag_path(&self, tag: &TagId) -> String {
        let mut parts = Vec::new();
        let mut current = Some(tag.clone());
        while let Some(id) = current {
            if let Some(tag) = self.tags.get(&id) {
                parts.push(tag.name.clone());
                current = tag.parent.clone();
            } else {
                break;
            }
        }
        parts.reverse();
        parts.join(" / ")
    }

    pub fn flatten_tags(&self) -> Vec<TagRow> {
        let mut out = Vec::new();
        for tag in &self.tag_roots {
            self.collect_tag(tag, 0, &mut out);
        }
        out
    }

    fn collect_tag(&self, tag: &TagId, depth: u32, out: &mut Vec<TagRow>) {
        out.push(TagRow {
            tag: tag.clone(),
            depth,
        });
        if let Some(tag) = self.tags.get(tag) {
            for child in &tag.child_order {
                self.collect_tag(child, depth + 1, out);
            }
        }
    }

    /// Tag availability including inherited blocked/on-hold state.
    pub fn tag_is_available(&self, tag: &TagId) -> bool {
        let Some(tag) = self.tags.get(tag) else {
            return true;
        };
        if tag.status != TagStatus::Available {
            return false;
        }
        tag.parent
            .as_ref()
            .map(|parent| self.tag_is_available(parent))
            .unwrap_or(true)
    }

    pub fn tag_usage_count(&self, tag: &TagId, include_children: bool) -> usize {
        let mut ids = BTreeSet::from([tag.clone()]);
        if include_children {
            ids.extend(self.tag_descendants(tag));
        }
        self.tasks
            .values()
            .filter(|task| !task.completed && task.tags.iter().any(|t| ids.contains(t)))
            .count()
    }

    pub fn ensure_tag_path(&mut self, path: &str, create: bool) -> Result<TagId> {
        let parts: Vec<&str> = path
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return Err(Error::invalid("empty tag name"));
        }
        let mut parent: Option<TagId> = None;
        for (index, part) in parts.iter().enumerate() {
            let siblings = match &parent {
                Some(id) => self.tag_children(id).to_vec(),
                None => self.tag_roots.clone(),
            };
            let found = siblings.into_iter().find(|id| {
                self.tags
                    .get(id)
                    .is_some_and(|tag| tag.name.eq_ignore_ascii_case(part))
            });
            match found {
                Some(id) => parent = Some(id),
                None if create => {
                    let mut tag = Tag::new(*part);
                    tag.parent = parent.clone();
                    let id = tag.id.clone();
                    self.tags.insert(id.clone(), tag);
                    match &parent {
                        Some(parent_id) => {
                            if let Some(parent_tag) = self.tags.get_mut(parent_id) {
                                parent_tag.child_order.push(id.clone());
                            }
                        }
                        None => self.tag_roots.push(id.clone()),
                    }
                    parent = Some(id);
                }
                None => {
                    return Err(Error::not_found("tag", parts[..=index].join(" / ")));
                }
            }
        }
        parent.ok_or_else(|| Error::invalid("empty tag path"))
    }

    pub fn find_tag_path(&self, path: &str) -> Option<TagId> {
        let mut parent: Option<TagId> = None;
        let parts: Vec<&str> = path
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        for part in parts {
            let siblings = match &parent {
                Some(id) => self.tag_children(id).to_vec(),
                None => self.tag_roots.clone(),
            };
            let found = siblings.into_iter().find(|id| {
                self.tags
                    .get(id)
                    .is_some_and(|tag| tag.name.eq_ignore_ascii_case(part))
            })?;
            parent = Some(found);
        }
        parent
    }

    // -------------------------------------------------------------- projects

    pub fn find_project_by_name(&self, name: &str) -> Option<ProjectId> {
        let target = name.trim();
        let mut matches = self
            .projects
            .values()
            .filter(|project| project.name.eq_ignore_ascii_case(target))
            .map(|project| project.id.clone());
        let first = matches.next()?;
        match matches.next() {
            Some(_) => None,
            None => Some(first),
        }
    }

    pub fn find_project_by_path(&self, path: &str) -> Option<ProjectId> {
        let path = path.trim();
        if let Some((folder_path, name)) = path.rsplit_once('/') {
            let folder_id = self.find_folder_path(folder_path)?;
            return self
                .projects
                .values()
                .find(|project| {
                    project.folder.as_ref() == Some(&folder_id)
                        && project.name.eq_ignore_ascii_case(name.trim())
                })
                .map(|project| project.id.clone());
        }
        self.find_project_by_name(path)
    }

    pub fn find_folder_path(&self, path: &str) -> Option<FolderId> {
        let mut parent: Option<FolderId> = None;
        let parts: Vec<&str> = path
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        for part in parts {
            let siblings = self.container_children(parent.clone()).to_vec();
            let found = siblings.into_iter().find_map(|container| match container {
                ContainerId::Folder(id)
                    if self
                        .folders
                        .get(&id)
                        .is_some_and(|folder| folder.name.eq_ignore_ascii_case(part)) =>
                {
                    Some(id)
                }
                _ => None,
            })?;
            parent = Some(found);
        }
        parent
    }

    pub fn ensure_folder_path(&mut self, path: &str) -> Result<FolderId> {
        let mut parent: Option<FolderId> = None;
        let parts: Vec<&str> = path
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return Err(Error::invalid("empty folder name"));
        }
        for part in parts {
            let found = self
                .container_children(parent.clone())
                .iter()
                .find_map(|c| {
                    if let ContainerId::Folder(id) = c {
                        self.folders
                            .get(id)
                            .filter(|folder| folder.name.eq_ignore_ascii_case(part))
                            .map(|_| id.clone())
                    } else {
                        None
                    }
                });
            match found {
                Some(id) => parent = Some(id),
                None => {
                    let mut folder = Folder::new(part);
                    folder.parent = parent.clone();
                    let id = folder.id.clone();
                    self.folders.insert(id.clone(), folder);
                    self.insert_container(
                        parent.clone(),
                        ContainerId::Folder(id.clone()),
                        ContainerPosition::End,
                    );
                    parent = Some(id);
                }
            }
        }
        parent.ok_or_else(|| Error::invalid("empty folder path"))
    }

    pub fn ensure_project_path(&mut self, path: &str) -> Result<ProjectId> {
        if let Some(id) = self.find_project_by_path(path) {
            return Ok(id);
        }
        let (folder, name) = match path.rsplit_once('/') {
            Some((folder_path, name)) => (
                Some(self.ensure_folder_path(folder_path)?),
                name.trim().to_string(),
            ),
            None => (None, path.trim().to_string()),
        };
        if name.is_empty() {
            return Err(Error::invalid("empty project name"));
        }
        let mut project = Project::new(name);
        project.review_interval_days = self.settings.default_review_days.max(1);
        project.next_review = crate::time::local_date(now())
            + chrono::Duration::days(project.review_interval_days as i64);
        project.folder = folder.clone();
        let id = project.id.clone();
        self.projects.insert(id.clone(), project);
        self.insert_container(
            folder,
            ContainerId::Project(id.clone()),
            ContainerPosition::End,
        );
        Ok(id)
    }

    pub fn project_count(&self, folder: &FolderId) -> usize {
        let mut count = 0;
        for child in self.container_children(Some(folder.clone())) {
            match child {
                ContainerId::Project(_) => count += 1,
                ContainerId::Folder(id) => count += self.project_count(id),
            }
        }
        count
    }

    // -------------------------------------------------------------- utilities

    pub fn tasks_tagged(&self, tag: &TagId, include_children: bool) -> Vec<TaskId> {
        let mut tags = BTreeSet::from([tag.clone()]);
        if include_children {
            tags.extend(self.tag_descendants(tag));
        }
        let mut out: Vec<TaskId> = self
            .tasks
            .values()
            .filter(|task| task.tags.iter().any(|t| tags.contains(t)))
            .map(|task| task.id.clone())
            .collect();
        out.sort_by_key(|id| {
            let task = &self.tasks[id];
            (
                task.completed,
                task.due.map(|s| s.at),
                task.title.to_lowercase(),
            )
        });
        out
    }

    pub fn all_open_tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values().filter(|task| !task.completed)
    }

    pub fn is_review_due(&self, project: &ProjectId, today: NaiveDate) -> bool {
        self.projects
            .get(project)
            .is_some_and(|project| project.is_due_for_review(today))
    }

    pub fn add_tag_location(&mut self, tag: &TagId, location: Option<Geo>) -> Result<()> {
        self.tag_mut(tag)?.location = location;
        self.touch();
        Ok(())
    }

    // ------------------------------------------------------------ validation

    /// Structural validation. The store refuses to save an invalid database,
    /// so every mutation is checked before it reaches disk.
    pub fn validate(&self) -> Result<()> {
        let mut problems: Vec<String> = Vec::new();

        for (id, task) in &self.tasks {
            if id != &task.id {
                problems.push(format!("task map key {id} does not match id {}", task.id));
            }
            if task.completed && task.completed_at.is_none() {
                problems.push(format!("completed task {id} has no completion date"));
            }
            let mut checklist_ids: HashSet<ChecklistId> = HashSet::new();
            for item in &task.checklist {
                if item.title.trim().is_empty() {
                    problems.push(format!("task {id} has an empty checklist item"));
                }
                if !checklist_ids.insert(item.id.clone()) {
                    problems.push(format!(
                        "task {id} has a duplicate checklist item {}",
                        item.id
                    ));
                }
            }
        }

        for (id, project) in &self.projects {
            if id != &project.id {
                problems.push(format!(
                    "project map key {id} does not match id {}",
                    project.id
                ));
            }
            if project.review_interval_days == 0 {
                problems.push(format!("project {id} has a zero review interval"));
            }
        }
        for (id, folder) in &self.folders {
            if id != &folder.id {
                problems.push(format!(
                    "folder map key {id} does not match id {}",
                    folder.id
                ));
            }
        }
        for (id, tag) in &self.tags {
            if id != &tag.id {
                problems.push(format!("tag map key {id} does not match id {}", tag.id));
            }
        }
        for (id, perspective) in &self.perspectives {
            if id != &perspective.id {
                problems.push(format!(
                    "perspective map key {id} does not match id {}",
                    perspective.id
                ));
            }
        }

        // Task order arrays.
        let mut ordered: HashSet<TaskId> = HashSet::new();
        for task in self.tasks.values() {
            for child in &task.child_order {
                if !ordered.insert(child.clone()) {
                    problems.push(format!("task {child} appears more than once in task order"));
                }
                match self.tasks.get(child) {
                    None => problems.push(format!("missing child task {child}")),
                    Some(child_task) => {
                        if child_task.parent.as_ref() != Some(&task.id) {
                            problems.push(format!(
                                "task {child} is ordered under {} but has a different parent",
                                task.id
                            ));
                        }
                        if child_task.project != task.project {
                            problems
                                .push(format!("task {child} project differs from parent project"));
                        }
                    }
                }
            }
        }
        for project in self.projects.values() {
            for task_id in &project.task_order {
                if !ordered.insert(task_id.clone()) {
                    problems.push(format!(
                        "task {task_id} appears more than once in task order"
                    ));
                }
                match self.tasks.get(task_id) {
                    None => problems.push(format!("missing project task {task_id}")),
                    Some(task) => {
                        if task.parent.is_some() {
                            problems
                                .push(format!("project task {task_id} unexpectedly has a parent"));
                        }
                        if task.project.as_ref() != Some(&project.id) {
                            problems.push(format!(
                                "project task {task_id} has the wrong project reference"
                            ));
                        }
                    }
                }
            }
        }
        for task_id in &self.inbox_order {
            if !ordered.insert(task_id.clone()) {
                problems.push(format!(
                    "task {task_id} appears more than once in task order"
                ));
            }
            match self.tasks.get(task_id) {
                None => problems.push(format!("missing inbox task {task_id}")),
                Some(task) => {
                    if task.parent.is_some() || task.project.is_some() {
                        problems.push(format!("inbox task {task_id} has a parent or project"));
                    }
                }
            }
        }
        for task_id in self.tasks.keys() {
            if !ordered.contains(task_id) {
                problems.push(format!("task {task_id} is orphaned (not in any order)"));
            }
        }

        // Folder/container tree.
        let mut containers: HashSet<ContainerId> = HashSet::new();
        for container in &self.root {
            if !containers.insert(container.clone()) {
                problems.push(format!("container {container} appears twice in root"));
            }
            match container {
                ContainerId::Folder(id) => match self.folders.get(id) {
                    None => problems.push(format!("missing root folder {id}")),
                    Some(folder) if folder.parent.is_some() => {
                        problems.push(format!("root folder {id} has a parent"))
                    }
                    _ => {}
                },
                ContainerId::Project(id) => match self.projects.get(id) {
                    None => problems.push(format!("missing root project {id}")),
                    Some(project) if project.folder.is_some() => {
                        problems.push(format!("root project {id} has a folder"))
                    }
                    _ => {}
                },
            }
        }
        for folder in self.folders.values() {
            for child in &folder.child_order {
                if !containers.insert(child.clone()) {
                    problems.push(format!("container {child} appears twice"));
                }
                match child {
                    ContainerId::Folder(id) => match self.folders.get(id) {
                        None => problems.push(format!("missing folder {id}")),
                        Some(child_folder) if child_folder.parent.as_ref() != Some(&folder.id) => {
                            problems.push(format!("folder {id} parent mismatch"))
                        }
                        _ => {}
                    },
                    ContainerId::Project(id) => match self.projects.get(id) {
                        None => problems.push(format!("missing project {id}")),
                        Some(project) if project.folder.as_ref() != Some(&folder.id) => {
                            problems.push(format!("project {id} folder mismatch"))
                        }
                        _ => {}
                    },
                }
            }
        }
        for id in self.folders.keys() {
            if !containers.contains(&ContainerId::Folder(id.clone())) {
                problems.push(format!("folder {id} is orphaned"));
            }
        }
        for id in self.projects.keys() {
            if !containers.contains(&ContainerId::Project(id.clone())) {
                problems.push(format!("project {id} is orphaned"));
            }
        }

        // Tag tree.
        let mut tags_seen: HashSet<TagId> = HashSet::new();
        for id in &self.tag_roots {
            if !tags_seen.insert(id.clone()) {
                problems.push(format!("tag {id} appears twice in roots"));
            }
            match self.tags.get(id) {
                None => problems.push(format!("missing root tag {id}")),
                Some(tag) if tag.parent.is_some() => {
                    problems.push(format!("root tag {id} has a parent"))
                }
                _ => {}
            }
        }
        for tag in self.tags.values() {
            for child in &tag.child_order {
                if !tags_seen.insert(child.clone()) {
                    problems.push(format!("tag {child} appears twice"));
                }
                match self.tags.get(child) {
                    None => problems.push(format!("missing tag {child}")),
                    Some(child_tag) if child_tag.parent.as_ref() != Some(&tag.id) => {
                        problems.push(format!("tag {child} parent mismatch"))
                    }
                    _ => {}
                }
            }
        }
        for id in self.tags.keys() {
            if !tags_seen.contains(id) {
                problems.push(format!("tag {id} is orphaned"));
            }
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(Error::invalid(problems.join("; ")))
        }
    }

    /// Remove dangling references and re-index ordering arrays. Returns a
    /// human-readable list of repairs. Used by `flowlist doctor --fix`.
    pub fn repair(&mut self) -> Vec<String> {
        let mut repaired = Vec::new();

        let known_tasks: HashSet<TaskId> = self.tasks.keys().cloned().collect();
        let known_projects: HashSet<ProjectId> = self.projects.keys().cloned().collect();
        let known_folders: HashSet<FolderId> = self.folders.keys().cloned().collect();

        for task in self.tasks.values_mut() {
            let before = task.child_order.len();
            task.child_order
                .retain(|id| known_tasks.contains(id) && id != &task.id);
            if task.child_order.len() != before {
                repaired.push(format!("removed dangling children from task {}", task.id));
            }
            if let Some(project) = &task.project {
                if !known_projects.contains(project) {
                    task.project = None;
                    repaired.push(format!("cleared missing project on task {}", task.id));
                }
            }
        }

        let mut seen: HashSet<TaskId> = HashSet::new();
        let mut dedupe = |list: &mut Vec<TaskId>, seen: &mut HashSet<TaskId>, label: &str| {
            let before = list.len();
            list.retain(|id| known_tasks.contains(id) && seen.insert(id.clone()));
            if list.len() != before {
                repaired.push(format!("repaired order list {label}"));
            }
        };
        let mut inbox = std::mem::take(&mut self.inbox_order);
        dedupe(&mut inbox, &mut seen, "inbox");
        self.inbox_order = inbox;
        let project_ids: Vec<ProjectId> = self.projects.keys().cloned().collect();
        for project_id in project_ids {
            let mut list = self
                .projects
                .get(&project_id)
                .map(|project| project.task_order.clone())
                .unwrap_or_default();
            dedupe(&mut list, &mut seen, &project_id.to_string());
            if let Some(project) = self.projects.get_mut(&project_id) {
                project.task_order = list;
            }
        }
        let task_ids: Vec<TaskId> = self.tasks.keys().cloned().collect();
        for task_id in task_ids {
            let mut list = self
                .tasks
                .get(&task_id)
                .map(|task| task.child_order.clone())
                .unwrap_or_default();
            dedupe(&mut list, &mut seen, &task_id.to_string());
            if let Some(task) = self.tasks.get_mut(&task_id) {
                task.child_order = list;
            }
        }
        for task_id in known_tasks {
            if !seen.contains(&task_id) {
                if let Some(task) = self.tasks.get(&task_id) {
                    if task.parent.is_none() && task.project.is_none() {
                        self.inbox_order.push(task_id.clone());
                        repaired.push(format!("re-added orphaned inbox task {task_id}"));
                    }
                }
            }
        }

        for container_list_owner in ["root"] {
            let _ = container_list_owner;
        }
        let mut root = std::mem::take(&mut self.root);
        root.retain(|container| match container {
            ContainerId::Folder(id) => known_folders.contains(id),
            ContainerId::Project(id) => known_projects.contains(id),
        });
        self.root = root;

        for folder in self.folders.values_mut() {
            folder.child_order.retain(|container| match container {
                ContainerId::Folder(id) => known_folders.contains(id),
                ContainerId::Project(id) => known_projects.contains(id),
            });
        }
        for project in self.projects.values_mut() {
            if let Some(folder) = &project.folder {
                if !known_folders.contains(folder) {
                    project.folder = None;
                    repaired.push(format!("moved project {} to root", project.id));
                }
            }
        }

        self.touch();
        repaired
    }

    /// Convenience for tests and sample data.
    pub fn add_task_to_inbox(&mut self, mut task: Task) -> TaskId {
        let id = task.id.clone();
        task.project = None;
        task.parent = None;
        self.inbox_order.push(id.clone());
        self.tasks.insert(id.clone(), task);
        self.touch();
        id
    }

    pub fn add_project(&mut self, mut project: Project) -> ProjectId {
        let id = project.id.clone();
        project.folder = None;
        self.projects.insert(id.clone(), project);
        self.root.push(ContainerId::Project(id.clone()));
        self.touch();
        id
    }

    pub fn add_folder(&mut self, mut folder: Folder) -> FolderId {
        let id = folder.id.clone();
        folder.parent = None;
        self.folders.insert(id.clone(), folder);
        self.root.push(ContainerId::Folder(id.clone()));
        self.touch();
        id
    }

    pub fn set_task_due(&mut self, id: &TaskId, due: Option<Stamp>) -> Result<()> {
        self.task_mut(id)?.due = due;
        self.touch();
        Ok(())
    }

    pub fn project_status(&self, id: &ProjectId) -> Option<ProjectStatus> {
        self.projects.get(id).map(|project| project.status)
    }

    pub fn tag_status(&self, id: &TagId) -> Option<TagStatus> {
        self.tags.get(id).map(|tag| tag.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_db_is_valid() {
        Db::new().validate().unwrap();
    }

    #[test]
    fn adding_tasks_keeps_invariants() {
        let mut db = Db::new();
        let a = db.add_task_to_inbox(Task::new("one"));
        let b = db.add_task_to_inbox(Task::new("two"));
        db.validate().unwrap();
        assert_eq!(db.inbox_order, vec![a, b]);
    }

    #[test]
    fn project_paths_are_created() {
        let mut db = Db::new();
        let project = db.ensure_project_path("Work/Website").unwrap();
        assert_eq!(db.project_path(&project), "Work / Website");
        assert_eq!(
            db.find_project_by_path("Work/Website"),
            Some(project.clone())
        );
        db.validate().unwrap();
    }

    #[test]
    fn tag_paths_are_hierarchical() {
        let mut db = Db::new();
        let child = db.ensure_tag_path("Errands/Phone", true).unwrap();
        assert_eq!(db.tag_path(&child), "Errands / Phone");
        assert_eq!(db.ensure_tag_path("errands/phone", false).unwrap(), child);
        db.validate().unwrap();
    }
}
