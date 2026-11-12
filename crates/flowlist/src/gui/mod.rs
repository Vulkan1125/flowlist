//! The Flowlist window: state, commands, keyboard routing and layout.
//!
//! The GUI shares its entire domain layer with the CLI and MCP server. Every
//! edit goes through `Store::execute`, so undo, validation and cross-process
//! locking behave exactly the same everywhere.

mod content;
mod drag;
mod inspector;
mod modals;
mod session;
mod sidebar;
mod text;
mod theme;
mod vim;
mod widgets;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use anyhow::Result;
use gpui::{
    App, Application, Bounds, ClipboardItem, Context, FocusHandle, Focusable, IntoElement,
    KeyDownEvent, Keystroke, MouseButton, Render, ScrollHandle, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, size,
};

use flowlist_core::command::{
    Command, NewProject, NewTask, Outcome, ProjectPatch, SettingsPatch, TagPatch, TaskPatch,
    TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, Db, OrderKey, Position};
use flowlist_core::ids::{ChecklistId, ContainerId, FolderId, ProjectId, TagId, TaskId};
use flowlist_core::model::{
    BuiltinPerspective, Condition, FilterNode, GroupBy, Perspective, SortBy, TagStatus,
};
use flowlist_core::parse::{parse_duration, parse_quick, parse_repeat, parse_stamp};
use flowlist_core::paths::DataPaths;
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::store::Store;
use flowlist_core::time::{format_duration, local_date, now};
use flowlist_core::{export, taskpaper};

use drag::DragTask;

pub use session::{Session, View};
use theme::{Mode, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusArea {
    Content,
    Sidebar,
    Capture,
    Search,
    Field,
    TagInput,
    Checklist,
    ChecklistList,
    Modal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditField {
    Title,
    Notes,
    Project,
    Defer,
    Due,
    Estimate,
    Repeat,
}

impl EditField {
    fn label(self) -> &'static str {
        match self {
            EditField::Title => "Title",
            EditField::Notes => "Notes",
            EditField::Project => "Project",
            EditField::Defer => "Defer",
            EditField::Due => "Due",
            EditField::Estimate => "Estimate",
            EditField::Repeat => "Repeat",
        }
    }

    fn next(self) -> Self {
        match self {
            EditField::Title => EditField::Notes,
            EditField::Notes => EditField::Project,
            EditField::Project => EditField::Defer,
            EditField::Defer => EditField::Due,
            EditField::Due => EditField::Estimate,
            EditField::Estimate => EditField::Repeat,
            EditField::Repeat => EditField::Title,
        }
    }

    fn previous(self) -> Self {
        match self {
            EditField::Title => EditField::Repeat,
            EditField::Notes => EditField::Title,
            EditField::Project => EditField::Notes,
            EditField::Defer => EditField::Project,
            EditField::Due => EditField::Defer,
            EditField::Estimate => EditField::Due,
            EditField::Repeat => EditField::Estimate,
        }
    }
}

enum BufferKey {
    Ignored,
    Changed,
    Submit,
    Cancel,
    Tab,
}

pub struct FlowlistApp {
    store: Store,
    theme: Theme,
    view: View,
    selected: Vec<TaskId>,
    anchor: Option<TaskId>,
    expanded: BTreeSet<String>,
    collapsed_groups: BTreeSet<String>,
    show_inspector: bool,
    show_completed: bool,
    vim_mode: bool,
    vim: vim::VimState,
    sidebar_cursor: Option<sidebar::SidebarTarget>,
    checklist_cursor: Option<usize>,
    focus: FocusArea,
    focus_handle: FocusHandle,
    capture: text::TextBuffer,
    search: text::TextBuffer,
    field: text::TextBuffer,
    tag_buffer: text::TextBuffer,
    checklist_buffer: text::TextBuffer,
    checklist_editing: Option<ChecklistId>,
    editing: Option<EditField>,
    field_error: Option<String>,
    modal: Option<modals::Modal>,
    modal_focus: usize,
    toast: Option<String>,
    sort: Option<SortBy>,
    group: Option<GroupBy>,
    scroll: ScrollHandle,
    sidebar_scroll: ScrollHandle,
    last_sync: Instant,
}

impl FlowlistApp {
    fn new(store: Store, cx: &mut Context<Self>) -> Self {
        let session: Session = store.read_session().unwrap_or_default();
        let mut view = session.view(store.db());
        if !view_exists(store.db(), &view) {
            view = View::Builtin(BuiltinPerspective::Inbox);
        }
        let selected = session
            .selected_tasks(store.db())
            .into_iter()
            .filter(|id| store.db().tasks.contains_key(id))
            .collect();
        let theme = Theme::new(session.theme_mode().unwrap_or(Mode::Light));
        let sort = session.sort.as_deref().and_then(sort_from_slug);
        let group = session.group.as_deref().and_then(group_from_slug);
        let show_completed = session.show_completed.unwrap_or(false);
        let show_inspector = session.inspector.unwrap_or(true);
        let vim_mode = session.vim_mode.unwrap_or(true);
        let focus_handle = cx.focus_handle();
        let toast = store_warning(&store);

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = this.update(cx, |this, cx| {
                    if this.store.reload_if_changed().unwrap_or(false) {
                        this.prune_selection();
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            store,
            theme,
            view,
            selected,
            anchor: None,
            expanded: BTreeSet::new(),
            collapsed_groups: BTreeSet::new(),
            show_inspector,
            show_completed,
            vim_mode,
            vim: vim::VimState::default(),
            sidebar_cursor: None,
            checklist_cursor: None,
            focus: FocusArea::Content,
            focus_handle,
            capture: text::TextBuffer::empty(),
            search: text::TextBuffer::empty(),
            field: text::TextBuffer::empty(),
            tag_buffer: text::TextBuffer::empty(),
            checklist_buffer: text::TextBuffer::empty(),
            checklist_editing: None,
            editing: None,
            field_error: None,
            modal: None,
            modal_focus: 0,
            toast,
            sort,
            group,
            scroll: ScrollHandle::new(),
            sidebar_scroll: ScrollHandle::new(),
            last_sync: Instant::now(),
        }
    }

    // ------------------------------------------------------------- session

    fn save_session(&self) {
        let session = Session {
            view: Some(self.view.key()),
            selected: self.selected.iter().map(|id| id.to_string()).collect(),
            inspector: Some(self.show_inspector),
            theme: Some(self.theme.mode().slug().to_string()),
            window: None,
            sort: self.sort.map(|sort| sort_slug(sort).to_string()),
            group: self.group.map(|group| group_slug(group).to_string()),
            show_completed: Some(self.show_completed),
            vim_mode: Some(self.vim_mode),
        };
        let _ = self.store.write_session(&session);
    }

    fn prune_selection(&mut self) {
        let db = self.store.db();
        self.selected.retain(|id| db.tasks.contains_key(id));
        if let Some(anchor) = &self.anchor {
            if !db.tasks.contains_key(anchor) {
                self.anchor = self.selected.first().cloned();
            }
        }
    }

    // -------------------------------------------------------------- views

    fn current_groups(&self) -> Vec<Group> {
        let db = self.store.db();
        let at = now();
        let mut groups = match &self.view {
            View::Builtin(BuiltinPerspective::Projects) => {
                query::view(db, BuiltinPerspective::Projects, at)
            }
            View::Builtin(BuiltinPerspective::Review) => Vec::new(),
            View::Builtin(BuiltinPerspective::Nearby) => Vec::new(),
            View::Builtin(perspective) => query::view(db, *perspective, at),
            View::Project(id) => self.project_groups(id),
            View::Folder(id) => db
                .flatten_containers()
                .into_iter()
                .filter_map(|row| match row.container {
                    ContainerId::Project(project) => Some(project),
                    ContainerId::Folder(_) => None,
                })
                .filter(|project| project_in_folder(db, project, id))
                .flat_map(|project| self.project_groups(&project))
                .collect::<Vec<_>>(),
            View::Tag(id) => {
                let ids: Vec<TaskId> = db
                    .tasks_tagged(id, true)
                    .into_iter()
                    .filter(|task_id| {
                        db.tasks
                            .get(task_id)
                            .is_some_and(|task| self.show_completed || !task.completed)
                    })
                    .collect();
                let rows = query::rows_for(db, &ids, at);
                vec![Group {
                    key: format!("tag:{id}"),
                    title: db.tag_path(id),
                    position: 0,
                    rows,
                }]
            }
            View::Custom(id) => match db.perspectives.get(id) {
                Some(perspective) => query::custom(db, perspective, at),
                None => Vec::new(),
            },
        };

        if let Some(sort) = self.sort {
            for group in &mut groups {
                query::sort::sort_rows(db, &mut group.rows, sort);
            }
        }

        if let Some(group_by) = self.group {
            let rows: Vec<TaskRow> = groups.into_iter().flat_map(|group| group.rows).collect();
            groups = query::regroup(db, rows, group_by, at);
        }

        if !self.search.is_empty() {
            groups = self.filter_groups(groups);
        }
        groups
    }

    fn project_groups(&self, id: &ProjectId) -> Vec<Group> {
        let db = self.store.db();
        let at = now();
        let rows: Vec<TaskRow> = db
            .project_outline(id, self.show_completed)
            .into_iter()
            .map(|row| {
                let mut task_row = query::row_for(db, &row.task, at);
                task_row.depth = row.depth;
                task_row
            })
            .collect();
        let title = db
            .projects
            .get(id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project".to_string());
        vec![Group {
            key: format!("project:{id}"),
            title,
            position: 0,
            rows,
        }]
    }

    fn filter_groups(&self, groups: Vec<Group>) -> Vec<Group> {
        let db = self.store.db();
        let needle = self.search.text().trim().to_lowercase();
        let mut out = Vec::new();
        for mut group in groups {
            group.rows.retain(|row| {
                let Some(task) = db.tasks.get(&row.id) else {
                    return false;
                };
                let project = task
                    .project
                    .as_ref()
                    .map(|id| db.project_path(id))
                    .unwrap_or_default();
                let tags = task
                    .tags
                    .iter()
                    .filter_map(|id| db.tags.get(id))
                    .map(|tag| tag.name.clone())
                    .collect::<Vec<_>>()
                    .join(" ");
                task.title.to_lowercase().contains(&needle)
                    || task.notes.to_lowercase().contains(&needle)
                    || project.to_lowercase().contains(&needle)
                    || tags.to_lowercase().contains(&needle)
            });
            if !group.rows.is_empty() {
                out.push(group);
            }
        }
        out
    }

    fn visible_ids(&self) -> Vec<TaskId> {
        self.current_groups()
            .into_iter()
            .flat_map(|group| group.rows.into_iter().map(|row| row.id))
            .collect()
    }

    fn open_view(&mut self, view: View, cx: &mut Context<Self>) {
        self.view = view;
        self.selected.clear();
        self.anchor = None;
        self.search.clear();
        self.focus = FocusArea::Content;
        self.save_session();
        cx.notify();
    }

    fn view_title(&self) -> String {
        self.view.title(self.store.db())
    }

    fn view_subtitle(&self) -> &'static str {
        self.view.subtitle()
    }

    // ----------------------------------------------------------- selection

    fn selected_one(&self) -> Option<TaskId> {
        self.selected.first().cloned()
    }

    fn select_only(&mut self, id: TaskId) {
        self.selected = vec![id.clone()];
        self.anchor = Some(id);
    }

    fn toggle_selection(&mut self, id: TaskId) {
        if let Some(index) = self.selected.iter().position(|candidate| candidate == &id) {
            self.selected.remove(index);
        } else {
            self.selected.push(id.clone());
            self.anchor = Some(id);
        }
    }

    fn select_range_to(&mut self, id: TaskId) {
        let ids = self.visible_ids();
        let Some(anchor) = self.anchor.clone() else {
            self.select_only(id);
            return;
        };
        let Some(start) = ids.iter().position(|candidate| candidate == &anchor) else {
            self.select_only(id);
            return;
        };
        let Some(end) = ids.iter().position(|candidate| candidate == &id) else {
            self.select_only(id);
            return;
        };
        let (low, high) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        self.selected = ids[low..=high].to_vec();
        self.save_session();
    }

    fn move_selection(&mut self, delta: isize, extend: bool) {
        let ids = self.visible_ids();
        if ids.is_empty() {
            self.selected.clear();
            return;
        }
        let current = self
            .selected
            .first()
            .and_then(|id| ids.iter().position(|candidate| candidate == id));
        let next = match current {
            Some(index) => {
                let target = index as isize + delta;
                target.clamp(0, ids.len() as isize - 1) as usize
            }
            None => {
                if delta < 0 {
                    ids.len() - 1
                } else {
                    0
                }
            }
        };
        if extend {
            let anchor = self.anchor.clone().unwrap_or_else(|| ids[next].clone());
            self.anchor = Some(anchor);
            self.selected = vec![ids[next].clone()];
            self.select_range_to(ids[next].clone());
        } else {
            self.select_only(ids[next].clone());
        }
        self.save_session();
    }

    // ------------------------------------------------------------ commands

    fn execute(&mut self, command: Command) -> Option<Outcome> {
        match self.store.execute(command) {
            Ok(outcome) => {
                self.toast = outcome
                    .message
                    .clone()
                    .or_else(|| Some(outcome.label.clone()));
                self.prune_selection();
                self.save_session();
                Some(outcome)
            }
            Err(error) => {
                self.toast = Some(format!("Could not save: {error}"));
                None
            }
        }
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        match self.store.undo() {
            Ok(outcome) => {
                self.toast = Some(outcome.label);
                self.prune_selection();
                self.save_session();
            }
            Err(error) => self.toast = Some(error.to_string()),
        }
        cx.notify();
    }

    fn redo(&mut self, cx: &mut Context<Self>) {
        match self.store.redo() {
            Ok(outcome) => {
                self.toast = Some(outcome.label);
                self.prune_selection();
                self.save_session();
            }
            Err(error) => self.toast = Some(error.to_string()),
        }
        cx.notify();
    }

    fn complete_selected(&mut self, completed: bool, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        let ids = self.selected.clone();
        self.execute(Command::CompleteTasks { ids, completed });
        self.save_session();
        cx.notify();
    }

    fn toggle_complete(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let completed = !self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.completed);
        self.execute(Command::CompleteTasks {
            ids: vec![id],
            completed,
        });
        cx.notify();
    }

    fn flag_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        let db = self.store.db();
        let flag = !self
            .selected
            .iter()
            .all(|id| db.tasks.get(id).is_some_and(|task| task.flagged));
        self.execute(Command::SetTaskFlag {
            ids: self.selected.clone(),
            flagged: flag,
        });
        cx.notify();
    }

    fn toggle_flag(&mut self, id: TaskId, cx: &mut Context<Self>) {
        let flagged = !self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.flagged);
        self.execute(Command::SetTaskFlag {
            ids: vec![id],
            flagged,
        });
        cx.notify();
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        let ids = self.selected.clone();
        let count = ids.len();
        self.execute(Command::DeleteTasks { ids });
        self.selected.clear();
        self.anchor = None;
        self.toast = Some(format!("Deleted {count} task(s) · ⌘Z to undo"));
        self.save_session();
        cx.notify();
    }

    fn indent_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(key) = db.order_key_for(&id) else {
            cx.notify();
            return;
        };
        let siblings = db.ordered_tasks(key).to_vec();
        let Some(index) = siblings.iter().position(|candidate| candidate == &id) else {
            cx.notify();
            return;
        };
        if index == 0 {
            self.toast = Some("Nothing above to nest under".to_string());
            cx.notify();
            return;
        }
        let parent = siblings[index - 1].clone();
        self.execute(Command::MoveTasks {
            ids: vec![id],
            placement: TaskPlacement::Parent {
                parent,
                position: Position::End,
            },
        });
        cx.notify();
    }

    fn outdent_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(task) = db.tasks.get(&id) else {
            cx.notify();
            return;
        };
        let Some(parent_id) = task.parent.clone() else {
            self.toast = Some("Task is already at the top level".to_string());
            cx.notify();
            return;
        };
        let Some(parent) = db.tasks.get(&parent_id) else {
            cx.notify();
            return;
        };
        let placement = match &parent.parent {
            Some(grandparent) => TaskPlacement::Parent {
                parent: grandparent.clone(),
                position: Position::After(parent_id.clone()),
            },
            None => match &parent.project {
                Some(project) => TaskPlacement::Project {
                    project: project.clone(),
                    position: Position::After(parent_id.clone()),
                },
                None => TaskPlacement::Inbox(Position::After(parent_id.clone())),
            },
        };
        self.execute(Command::MoveTasks {
            ids: vec![id],
            placement,
        });
        cx.notify();
    }

    fn move_selected(&mut self, up: bool, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(key) = db.order_key_for(&id) else {
            cx.notify();
            return;
        };
        let siblings = db.ordered_tasks(key.clone()).to_vec();
        let Some(index) = siblings.iter().position(|candidate| candidate == &id) else {
            cx.notify();
            return;
        };
        let position = if up {
            if index == 0 {
                cx.notify();
                return;
            }
            Position::Before(siblings[index - 1].clone())
        } else {
            if index + 1 >= siblings.len() {
                cx.notify();
                return;
            }
            Position::After(siblings[index + 1].clone())
        };
        let placement = match key {
            OrderKey::Inbox => TaskPlacement::Inbox(position),
            OrderKey::Project(project) => TaskPlacement::Project { project, position },
            OrderKey::Parent(parent) => TaskPlacement::Parent { parent, position },
        };
        self.execute(Command::MoveTasks {
            ids: vec![id],
            placement,
        });
        cx.notify();
    }

    // ------------------------------------------------------------- capture

    fn commit_capture(&mut self, cx: &mut Context<Self>) {
        let text = self.capture.take();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        }
        let at = now();
        match parse_quick(trimmed, at) {
            Ok(entry) => {
                let mut new_task = entry.into_new_task();
                match &self.view {
                    View::Project(id) => {
                        new_task.project = Some(id.clone());
                        new_task.project_path = None;
                    }
                    View::Tag(id) => {
                        if !new_task.tags.contains(id) {
                            new_task.tags.push(id.clone());
                        }
                    }
                    _ => {}
                }
                self.execute(Command::CreateTask(new_task));
            }
            Err(error) => {
                self.toast = Some(format!("Could not understand that: {error}"));
            }
        }
        self.focus = FocusArea::Capture;
        cx.notify();
    }

    // ------------------------------------------------------------ inspector

    fn begin_checklist_edit(&mut self, item: ChecklistId, title: String, cx: &mut Context<Self>) {
        self.editing = None;
        self.checklist_editing = Some(item);
        self.checklist_buffer.set(title);
        self.focus = FocusArea::Checklist;
        cx.notify();
    }

    fn commit_checklist(&mut self, cx: &mut Context<Self>) {
        let Some(task_id) = self.selected_one() else {
            self.checklist_editing = None;
            self.checklist_buffer.clear();
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let title = self.checklist_buffer.text().trim().to_string();
        if title.is_empty() {
            if self.checklist_editing.is_some() {
                self.toast = Some("A checklist item needs a title".to_string());
            } else {
                self.focus = FocusArea::Content;
            }
            cx.notify();
            return;
        }
        match self.checklist_editing.take() {
            Some(item) => {
                self.execute(Command::UpdateChecklistItem {
                    task: task_id,
                    item,
                    title,
                    completed: None,
                });
                self.checklist_buffer.clear();
                self.focus = FocusArea::Content;
            }
            None => {
                self.execute(Command::AddChecklistItem {
                    task: task_id,
                    title,
                    position: None,
                });
                self.checklist_buffer.clear();
            }
        }
        cx.notify();
    }

    fn toggle_checklist_item(&mut self, task: TaskId, item: ChecklistId, cx: &mut Context<Self>) {
        let db = self.store.db();
        let Some(entity) = db
            .tasks
            .get(&task)
            .and_then(|task| task.checklist_item(&item))
        else {
            return;
        };
        let title = entity.title.clone();
        let completed = !entity.completed;
        self.execute(Command::UpdateChecklistItem {
            task,
            item,
            title,
            completed: Some(completed),
        });
        cx.notify();
    }

    fn move_checklist_item(
        &mut self,
        task: TaskId,
        item: ChecklistId,
        to: usize,
        cx: &mut Context<Self>,
    ) {
        self.execute(Command::MoveChecklistItem { task, item, to });
        cx.notify();
    }

    fn delete_checklist_item(&mut self, task: TaskId, item: ChecklistId, cx: &mut Context<Self>) {
        if self.checklist_editing.as_ref() == Some(&item) {
            self.checklist_editing = None;
            self.checklist_buffer.clear();
            self.focus = FocusArea::Content;
        }
        self.execute(Command::DeleteChecklistItem { task, item });
        cx.notify();
    }

    fn begin_edit(&mut self, field: EditField, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        };
        let db = self.store.db();
        let Some(task) = db.tasks.get(&id) else {
            cx.notify();
            return;
        };
        let value = match field {
            EditField::Title => task.title.clone(),
            EditField::Notes => task.notes.clone(),
            EditField::Project => task
                .project
                .as_ref()
                .map(|project| db.project_path(project))
                .unwrap_or_default(),
            EditField::Defer => task
                .defer
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Due => task
                .due
                .as_ref()
                .map(|stamp| stamp.to_input_string())
                .unwrap_or_default(),
            EditField::Estimate => task.estimate_min.map(format_duration).unwrap_or_default(),
            EditField::Repeat => task
                .repeat
                .as_ref()
                .map(|rule| rule.description())
                .unwrap_or_default(),
        };
        self.field.set(value);
        self.field_error = None;
        self.editing = Some(field);
        self.focus = FocusArea::Field;
        cx.notify();
    }

    fn commit_field(&mut self, cx: &mut Context<Self>) {
        let Some(field) = self.editing else {
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let Some(id) = self.selected_one() else {
            self.editing = None;
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let raw = self.field.text().to_string();
        let value = raw.trim().to_string();
        let mut patch = TaskPatch::default();
        match field {
            EditField::Title => {
                if value.is_empty() {
                    self.field_error = Some("A task needs a title".to_string());
                    cx.notify();
                    return;
                }
                patch.title = Some(value);
            }
            EditField::Notes => {
                patch.notes = Some(raw);
            }
            EditField::Project => {
                if value.is_empty() {
                    patch.project = Some(None);
                } else {
                    let db = self.store.db();
                    let project = db
                        .find_project_by_path(&value)
                        .or_else(|| db.find_project_by_name(&value));
                    match project {
                        Some(project) => patch.project = Some(Some(project)),
                        None => {
                            self.field_error = Some(format!("No project named “{value}”"));
                            cx.notify();
                            return;
                        }
                    }
                }
            }
            EditField::Defer | EditField::Due => {
                let stamp = if value.is_empty() {
                    None
                } else {
                    match parse_stamp(&value, now()) {
                        Ok(stamp) => Some(stamp),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return;
                        }
                    }
                };
                if field == EditField::Defer {
                    patch.defer = Some(stamp);
                } else {
                    patch.due = Some(stamp);
                }
            }
            EditField::Estimate => {
                let estimate = if value.is_empty() {
                    None
                } else {
                    match parse_duration(&value) {
                        Ok(minutes) => Some(minutes),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return;
                        }
                    }
                };
                patch.estimate_min = Some(estimate);
            }
            EditField::Repeat => {
                let repeat = if value.is_empty() {
                    None
                } else {
                    match parse_repeat(&value, now()) {
                        Ok(rule) => Some(rule),
                        Err(error) => {
                            self.field_error = Some(error.to_string());
                            cx.notify();
                            return;
                        }
                    }
                };
                patch.repeat = Some(repeat);
            }
        }
        self.execute(Command::UpdateTask { id, patch });
        self.editing = None;
        self.field_error = None;
        self.focus = FocusArea::Content;
        cx.notify();
    }

    fn add_tag_from_buffer(&mut self, cx: &mut Context<Self>) {
        let name = self.tag_buffer.take().trim().to_string();
        if name.is_empty() {
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        }
        let Some(id) = self.selected_one() else {
            self.toast = Some("Select a task first".to_string());
            self.focus = FocusArea::Content;
            cx.notify();
            return;
        };
        let existing = self.store.db().find_tag_path(&name);
        let tag = match existing {
            Some(tag) => tag,
            None => {
                let outcome = self.execute(Command::CreateTag {
                    name: name.clone(),
                    parent: None,
                    status: TagStatus::Available,
                    location: None,
                    position: Position::End,
                });
                match outcome.and_then(|outcome| outcome.created.first().cloned()) {
                    Some(created) => TagId::from_string(created),
                    None => {
                        self.focus = FocusArea::Content;
                        cx.notify();
                        return;
                    }
                }
            }
        };
        self.execute(Command::SetTaskTags {
            ids: vec![id],
            add: vec![tag],
            remove: Vec::new(),
        });
        self.focus = FocusArea::TagInput;
        cx.notify();
    }

    // ------------------------------------------------------------ mutations

    fn toggle_show_completed(&mut self, cx: &mut Context<Self>) {
        self.show_completed = !self.show_completed;
        self.save_session();
        cx.notify();
    }

    fn cycle_sort(&mut self, cx: &mut Context<Self>) {
        let order = [
            None,
            Some(SortBy::Due),
            Some(SortBy::Defer),
            Some(SortBy::Name),
            Some(SortBy::Flagged),
            Some(SortBy::Estimate),
            Some(SortBy::Created),
            Some(SortBy::Manual),
        ];
        let index = order
            .iter()
            .position(|candidate| *candidate == self.sort)
            .unwrap_or(0);
        self.sort = order[(index + 1) % order.len()];
        self.save_session();
        cx.notify();
    }

    fn cycle_group(&mut self, cx: &mut Context<Self>) {
        let order = [
            None,
            Some(GroupBy::Project),
            Some(GroupBy::Tag),
            Some(GroupBy::Folder),
            Some(GroupBy::DueDate),
            Some(GroupBy::DeferDate),
            Some(GroupBy::Flagged),
            Some(GroupBy::Estimate),
        ];
        let index = order
            .iter()
            .position(|candidate| *candidate == self.group)
            .unwrap_or(0);
        self.group = order[(index + 1) % order.len()];
        self.save_session();
        cx.notify();
    }

    fn toggle_expanded(&mut self, key: String, cx: &mut Context<Self>) {
        self.toggle_open(key);
        cx.notify();
    }

    fn toggle_group_collapsed(&mut self, key: String, cx: &mut Context<Self>) {
        self.toggle_open(key);
        cx.notify();
    }

    fn toggle_open(&mut self, key: String) {
        if self.is_open(&key, true) {
            self.expanded.remove(&key);
            self.collapsed_groups.insert(key);
        } else {
            self.collapsed_groups.remove(&key);
            self.expanded.insert(key);
        }
    }

    fn is_open(&self, key: &str, default: bool) -> bool {
        if self.expanded.contains(key) {
            true
        } else if self.collapsed_groups.contains(key) {
            false
        } else {
            default
        }
    }

    fn drop_on_project(&mut self, project: ProjectId, payload: &DragTask, cx: &mut Context<Self>) {
        let ids = self.drag_ids(payload);
        if payload.tag {
            return;
        }
        self.execute(Command::MoveTasks {
            ids,
            placement: TaskPlacement::Project {
                project,
                position: Position::End,
            },
        });
        cx.notify();
    }

    fn drop_on_task(&mut self, target: TaskId, payload: &DragTask, cx: &mut Context<Self>) {
        if payload.tag || payload.id == target {
            return;
        }
        let db = self.store.db();
        if db.task_subtree(&payload.id).contains(&target) {
            self.toast = Some("A task cannot be nested inside itself".to_string());
            cx.notify();
            return;
        }
        let ids = self.drag_ids(payload);
        self.execute(Command::MoveTasks {
            ids,
            placement: TaskPlacement::Parent {
                parent: target,
                position: Position::End,
            },
        });
        cx.notify();
    }

    fn drop_on_tag(&mut self, tag: TagId, payload: &DragTask, cx: &mut Context<Self>) {
        let ids = self.drag_ids(payload);
        self.execute(Command::SetTaskTags {
            ids,
            add: vec![tag],
            remove: Vec::new(),
        });
        cx.notify();
    }

    fn drag_ids(&self, payload: &DragTask) -> Vec<TaskId> {
        if self.selected.contains(&payload.id) {
            self.selected.clone()
        } else {
            vec![payload.id.clone()]
        }
    }

    fn drag_payload(&self, id: &TaskId) -> Option<DragTask> {
        let task = self.store.db().tasks.get(id)?;
        Some(DragTask::task(id.clone(), task.title.clone()))
    }

    // ------------------------------------------------------------ clipboard

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let db = self.store.db();
        let lines: Vec<String> = self
            .selected
            .iter()
            .filter_map(|id| db.tasks.get(id))
            .map(|task| {
                let project = task
                    .project
                    .as_ref()
                    .map(|project| db.project_path(project))
                    .unwrap_or_else(|| "Inbox".to_string());
                format!("{}\t{}", task.title, project)
            })
            .collect();
        if lines.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(lines.join("\n")));
        self.toast = Some(format!("Copied {} task(s)", lines.len()));
        cx.notify();
    }

    // -------------------------------------------------------------- modals

    fn open_modal(&mut self, modal: modals::Modal, cx: &mut Context<Self>) {
        self.modal = Some(modal);
        self.modal_focus = 0;
        self.focus = FocusArea::Modal;
        cx.notify();
    }

    fn entity_for_view(&self) -> Option<modals::EntityRef> {
        match &self.view {
            View::Project(id) => Some(modals::EntityRef::Project(id.clone())),
            View::Folder(id) => Some(modals::EntityRef::Folder(id.clone())),
            View::Tag(id) => Some(modals::EntityRef::Tag(id.clone())),
            View::Custom(id) => Some(modals::EntityRef::Perspective(id.clone())),
            View::Builtin(_) => None,
        }
    }

    fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        self.modal_focus = 0;
        self.focus = FocusArea::Content;
        cx.notify();
    }

    fn commit_modal(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        let result = self.apply_modal(&modal);
        match result {
            Ok(()) => {
                self.modal = None;
                self.modal_focus = 0;
                self.focus = FocusArea::Content;
            }
            Err(message) => {
                self.modal = Some(modal);
                self.toast = Some(message);
            }
        }
        cx.notify();
    }

    fn apply_modal(&mut self, modal: &modals::Modal) -> std::result::Result<(), String> {
        match modal {
            modals::Modal::Rename { entity, name } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A name cannot be empty".to_string());
                }
                match entity {
                    modals::EntityRef::Project(id) => {
                        self.execute(Command::UpdateProject {
                            id: id.clone(),
                            patch: ProjectPatch {
                                name: Some(name),
                                ..Default::default()
                            },
                        });
                    }
                    modals::EntityRef::Folder(id) => {
                        self.execute(Command::RenameFolder {
                            id: id.clone(),
                            name,
                        });
                    }
                    modals::EntityRef::Tag(id) => {
                        self.execute(Command::UpdateTag {
                            id: id.clone(),
                            patch: TagPatch {
                                name: Some(name),
                                ..Default::default()
                            },
                        });
                    }
                    modals::EntityRef::Perspective(id) => {
                        let Some(mut perspective) = self.store.db().perspectives.get(id).cloned()
                        else {
                            return Err("That perspective no longer exists".to_string());
                        };
                        perspective.name = name;
                        self.execute(Command::SavePerspective { perspective });
                    }
                }
            }
            modals::Modal::NewProject { name, notes } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A project needs a name".to_string());
                }
                let db = self.store.db();
                let folder = match &self.view {
                    View::Folder(id) => Some(id.clone()),
                    View::Project(id) => db
                        .projects
                        .get(id)
                        .and_then(|project| project.folder.clone()),
                    _ => None,
                };
                let mut new_project = NewProject::new(name);
                new_project.notes = notes.text().to_string();
                new_project.folder = folder;
                new_project.position = ContainerPosition::End;
                if let Some(outcome) = self.execute(Command::CreateProject(new_project)) {
                    if let Some(created) = outcome.created.first() {
                        if let Ok(id) = self.store.db().resolve_project(created) {
                            self.view = View::Project(id);
                            self.selected.clear();
                            self.anchor = None;
                        }
                    }
                }
            }
            modals::Modal::NewFolder { name } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A folder needs a name".to_string());
                }
                let parent = match &self.view {
                    View::Folder(id) => Some(id.clone()),
                    _ => None,
                };
                self.execute(Command::CreateFolder {
                    name,
                    parent,
                    position: ContainerPosition::End,
                });
            }
            modals::Modal::NewTag { name } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A tag needs a name".to_string());
                }
                let parent = match &self.view {
                    View::Tag(id) => Some(id.clone()),
                    _ => None,
                };
                self.execute(Command::CreateTag {
                    name,
                    parent,
                    status: TagStatus::Available,
                    location: None,
                    position: Position::End,
                });
            }
            modals::Modal::NewPerspective {
                name,
                group,
                sort,
                flagged_only,
            } => {
                let name = name.text().trim().to_string();
                if name.is_empty() {
                    return Err("A perspective needs a name".to_string());
                }
                let mut perspective = Perspective::new(name);
                perspective.group_by = *group;
                perspective.sort = *sort;
                if *flagged_only {
                    perspective.filter =
                        FilterNode::all_conditions(vec![Condition::Flagged { value: true }]);
                }
                self.execute(Command::SavePerspective { perspective });
            }
            modals::Modal::Import { text, merge } => {
                let data = text.text().to_string();
                if data.trim().is_empty() {
                    return Err("Paste a library or TaskPaper text first".to_string());
                }
                let db = if data.trim_start().starts_with('{') {
                    export::from_json(&data).map_err(|error| error.to_string())?
                } else {
                    taskpaper::import(&data).map_err(|error| error.to_string())?
                };
                let mode = if *merge {
                    flowlist_core::command::ImportMode::Merge
                } else {
                    flowlist_core::command::ImportMode::Replace
                };
                self.execute(Command::Import { db, mode });
            }
            modals::Modal::Settings {
                review_days,
                completed_days,
                radius,
                vim_mode,
            } => {
                let review = review_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Review days must be a number".to_string())?;
                let completed = completed_days
                    .text()
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| "Completed days must be a number".to_string())?;
                let radius = radius
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Radius must be a number".to_string())?;
                self.execute(Command::UpdateSettings(SettingsPatch {
                    default_review_days: Some(review),
                    show_completed_days: Some(completed),
                    nearby_radius_km: Some(radius),
                    ..Default::default()
                }));
                self.vim_mode = *vim_mode;
                self.save_session();
            }
            modals::Modal::Location { lat, lon } => {
                let lat = lat
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Latitude must be a number".to_string())?;
                let lon = lon
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Longitude must be a number".to_string())?;
                self.execute(Command::UpdateSettings(SettingsPatch {
                    current_location: Some(Some(flowlist_core::model::Geo { lat, lon })),
                    ..Default::default()
                }));
            }
            modals::Modal::TagLocation { tag, lat, lon } => {
                let lat = lat
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Latitude must be a number".to_string())?;
                let lon = lon
                    .text()
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "Longitude must be a number".to_string())?;
                self.execute(Command::UpdateTag {
                    id: tag.clone(),
                    patch: TagPatch {
                        location: Some(Some(flowlist_core::model::Geo { lat, lon })),
                        ..Default::default()
                    },
                });
            }
            modals::Modal::Help => {}
        }
        Ok(())
    }

    // --------------------------------------------------------------- keys

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        let mods = stroke.modifiers;

        if self.focus == FocusArea::Modal {
            if mods.platform {
                if stroke.key == "w" || stroke.key == "q" {
                    self.close_modal(cx);
                    return;
                }
            }
            let paste = if mods.platform && stroke.key == "v" {
                cx.read_from_clipboard().and_then(|item| item.text())
            } else {
                None
            };
            if mods.platform && stroke.key == "a" {
                if let Some(buffer) = self.active_buffer_mut() {
                    buffer.select_all();
                }
                cx.notify();
                return;
            }
            if mods.platform && stroke.key == "c" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                return;
            }
            if mods.platform && stroke.key == "x" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    buffer.delete_selection();
                }
                cx.notify();
                return;
            }
            if let Some(buffer) = self.active_buffer_mut() {
                match handle_buffer_key(buffer, stroke, paste.as_deref()) {
                    BufferKey::Changed => {
                        cx.notify();
                        return;
                    }
                    BufferKey::Submit => {
                        self.commit_modal(cx);
                        return;
                    }
                    BufferKey::Cancel => {
                        self.close_modal(cx);
                        return;
                    }
                    BufferKey::Tab => {
                        let count = self
                            .modal
                            .as_ref()
                            .map(|modal| modal.field_count())
                            .unwrap_or(0);
                        if count > 0 {
                            self.modal_focus = if mods.shift {
                                (self.modal_focus + count - 1) % count
                            } else {
                                (self.modal_focus + 1) % count
                            };
                        }
                        cx.notify();
                        return;
                    }
                    BufferKey::Ignored => {}
                }
            }
            return;
        }

        if matches!(
            self.focus,
            FocusArea::Capture
                | FocusArea::Search
                | FocusArea::Field
                | FocusArea::TagInput
                | FocusArea::Checklist
        ) {
            let paste = if mods.platform && stroke.key == "v" {
                cx.read_from_clipboard().and_then(|item| item.text())
            } else {
                None
            };
            if mods.platform && stroke.key == "a" {
                if let Some(buffer) = self.active_buffer_mut() {
                    buffer.select_all();
                }
                cx.notify();
                return;
            }
            if mods.platform && stroke.key == "c" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                return;
            }
            if mods.platform && stroke.key == "x" {
                if let Some(buffer) = self.active_buffer_mut() {
                    let text = buffer.selected_text();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    buffer.delete_selection();
                }
                cx.notify();
                return;
            }
            if let Some(buffer) = self.active_buffer_mut() {
                match handle_buffer_key(buffer, stroke, paste.as_deref()) {
                    BufferKey::Changed => {
                        cx.notify();
                        return;
                    }
                    BufferKey::Submit => {
                        self.submit_focus(cx);
                        return;
                    }
                    BufferKey::Cancel => {
                        self.cancel_focus(cx);
                        return;
                    }
                    BufferKey::Tab => {
                        self.tab_focus(mods.shift, cx);
                        return;
                    }
                    BufferKey::Ignored => {}
                }
            }
        }

        if self.vim_mode && self.handle_vim_key(stroke, cx) {
            window.focus(&self.focus_handle);
            return;
        }

        if mods.platform {
            match stroke.key.as_str() {
                "z" => {
                    if mods.shift {
                        self.redo(cx);
                    } else {
                        self.undo(cx);
                    }
                }
                "n" => {
                    if mods.shift {
                        self.open_modal(modals::Modal::new_project(self), cx);
                    } else if self.view.shows_capture() {
                        self.focus = FocusArea::Capture;
                        self.capture.clear();
                    } else {
                        self.focus = FocusArea::Search;
                        self.search.select_all();
                    }
                }
                "f" => {
                    self.focus = FocusArea::Search;
                    self.search.select_all();
                }
                "i" => {
                    self.show_inspector = !self.show_inspector;
                    self.save_session();
                }
                "d" => self.flag_selected(cx),
                "backspace" | "delete" => self.delete_selected(cx),
                "c" => self.copy_selection(cx),
                "s" => {
                    self.save_session();
                    self.toast = Some("Session saved".to_string());
                }
                "l" => {
                    self.show_inspector = true;
                    self.checklist_editing = None;
                    self.checklist_buffer.clear();
                    self.focus = FocusArea::Checklist;
                    self.save_session();
                }
                "r" => {
                    if let Some(entity) = self.entity_for_view() {
                        let modal = modals::Modal::rename(self, entity);
                        self.open_modal(modal, cx);
                    } else {
                        self.toast = Some(
                            "Open a project, folder, tag or perspective to rename it".to_string(),
                        );
                    }
                }
                "," => self.open_modal(modals::Modal::settings(self), cx),
                key if matches!(key, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8") => {
                    if let Some(index) = key.chars().next().and_then(|ch| ch.to_digit(10)) {
                        if let Some(perspective) =
                            BuiltinPerspective::ALL.get((index as usize).saturating_sub(1))
                        {
                            self.open_view(View::Builtin(*perspective), cx);
                        }
                    }
                }
                "o" => self.open_modal(
                    modals::Modal::Import {
                        text: text::TextBuffer::empty(),
                        merge: true,
                    },
                    cx,
                ),
                "e" => {
                    if mods.shift {
                        let text = taskpaper::export(self.store.db());
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        self.toast = Some("TaskPaper export copied to clipboard".to_string());
                    }
                }
                "?" | "/" => self.open_modal(modals::Modal::Help, cx),
                "up" => self.move_selected(true, cx),
                "down" => self.move_selected(false, cx),
                _ => {}
            }
            cx.notify();
            return;
        }

        if self.focus == FocusArea::Sidebar {
            match stroke.key.as_str() {
                "up" => self.sidebar_move(-1, cx),
                "down" => self.sidebar_move(1, cx),
                "enter" => self.sidebar_open_current(cx),
                "escape" => {
                    self.focus = FocusArea::Content;
                    cx.notify();
                }
                _ => {}
            }
            window.focus(&self.focus_handle);
            return;
        }

        if self.focus == FocusArea::ChecklistList {
            match stroke.key.as_str() {
                "up" => self.checklist_move_cursor(-1, cx),
                "down" => self.checklist_move_cursor(1, cx),
                "space" => self.checklist_toggle(cx),
                "enter" => self.checklist_begin_edit(cx),
                "a" => {
                    self.checklist_editing = None;
                    self.checklist_buffer.clear();
                    self.focus = FocusArea::Checklist;
                    cx.notify();
                }
                "backspace" | "delete" => self.checklist_delete(cx),
                "escape" => {
                    self.checklist_cursor = None;
                    self.focus = FocusArea::Content;
                    cx.notify();
                }
                _ => {}
            }
            window.focus(&self.focus_handle);
            return;
        }

        match stroke.key.as_str() {
            "up" => self.move_selection(-1, mods.shift),
            "down" => self.move_selection(1, mods.shift),
            "home" => self.move_selection(-1000, mods.shift),
            "end" => self.move_selection(1000, mods.shift),
            "space" => self.complete_selected(true, cx),
            "enter" => {
                if let Some(id) = self.selected_one() {
                    let completed = self
                        .store
                        .db()
                        .tasks
                        .get(&id)
                        .is_some_and(|task| task.completed);
                    if completed {
                        self.toggle_complete(id, cx);
                    } else {
                        self.begin_edit(EditField::Title, cx);
                    }
                }
            }
            "escape" => {
                self.selected.clear();
                self.anchor = None;
                self.search.clear();
                self.focus = FocusArea::Content;
                self.save_session();
                cx.notify();
                return;
            }
            "tab" => {
                if mods.shift {
                    self.outdent_selected(cx);
                } else {
                    self.indent_selected(cx);
                }
            }
            "backspace" | "delete" => self.delete_selected(cx),
            "n" => {
                if self.view.shows_capture() {
                    self.focus = FocusArea::Capture;
                    self.capture.clear();
                } else {
                    self.focus = FocusArea::Search;
                    self.search.select_all();
                }
                cx.notify();
                return;
            }
            "/" => {
                self.focus = FocusArea::Search;
                self.search.select_all();
                cx.notify();
                return;
            }
            "!" => self.flag_selected(cx),
            "x" => {
                let completed = self.selected.iter().all(|id| {
                    self.store
                        .db()
                        .tasks
                        .get(id)
                        .is_some_and(|task| task.completed)
                });
                self.complete_selected(!completed, cx);
            }
            "i" => {
                self.show_inspector = !self.show_inspector;
                self.save_session();
                cx.notify();
                return;
            }
            "e" => self.begin_edit(EditField::Title, cx),
            key if matches!(key, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8") => {
                if let Some(index) = key.chars().next().and_then(|ch| ch.to_digit(10)) {
                    if let Some(perspective) =
                        BuiltinPerspective::ALL.get((index as usize).saturating_sub(1))
                    {
                        self.open_view(View::Builtin(*perspective), cx);
                    }
                }
            }
            _ => {}
        }
        window.focus(&self.focus_handle);
        cx.notify();
    }

    // ----------------------------------------------------------------- vim

    fn handle_vim_key(&mut self, stroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        if !self.vim_mode {
            return false;
        }
        match self.vim.feed(&stroke.key, stroke.modifiers) {
            vim::VimInput::Ignored => false,
            vim::VimInput::Pending => {
                cx.notify();
                true
            }
            vim::VimInput::Action(action) => {
                self.perform_vim(action, cx);
                cx.notify();
                true
            }
        }
    }

    fn perform_vim(&mut self, action: vim::VimAction, cx: &mut Context<Self>) {
        use vim::VimAction as A;
        match action {
            A::Cancel => {
                self.vim.reset();
                self.selected.clear();
                self.anchor = None;
                self.search.clear();
                self.checklist_cursor = None;
                self.focus = FocusArea::Content;
                self.save_session();
            }
            A::Help => self.open_modal(modals::Modal::Help, cx),
            A::Search => {
                self.focus = FocusArea::Search;
                self.search.select_all();
            }
            A::NextMatch => self.search_step(true, cx),
            A::PrevMatch => self.search_step(false, cx),
            A::Inspector => {
                self.show_inspector = !self.show_inspector;
                if !self.show_inspector && self.focus == FocusArea::ChecklistList {
                    self.focus = FocusArea::Content;
                }
                self.save_session();
            }
            A::FocusSidebar => {
                self.sidebar_cursor = self
                    .sidebar_cursor
                    .clone()
                    .or_else(|| self.target_for_view(&self.view));
                self.focus = FocusArea::Sidebar;
                self.save_session();
            }
            A::FocusContent => {
                self.checklist_cursor = None;
                self.focus = FocusArea::Content;
            }
            A::FocusChecklist => self.focus_checklist_pane(cx),
            A::Move(delta) => match self.focus {
                FocusArea::Sidebar => self.sidebar_move(delta, cx),
                FocusArea::ChecklistList => self.checklist_move_cursor(delta, cx),
                _ => {
                    let extend = false;
                    self.move_selection(delta, extend);
                    cx.notify();
                }
            },
            A::HalfPage(delta) => match self.focus {
                FocusArea::Sidebar => self.sidebar_move(delta * 10, cx),
                FocusArea::ChecklistList => self.checklist_move_cursor(delta * 10, cx),
                _ => {
                    self.move_selection(delta * 10, false);
                    cx.notify();
                }
            },
            A::ReorderDown => match self.focus {
                FocusArea::ChecklistList => self.checklist_reorder(true, cx),
                _ => self.move_selected(false, cx),
            },
            A::ReorderUp => match self.focus {
                FocusArea::ChecklistList => self.checklist_reorder(false, cx),
                _ => self.move_selected(true, cx),
            },
            A::Top => self.vim_jump(0, true, cx),
            A::Bottom => self.vim_jump(usize::MAX, false, cx),
            A::Goto(line) => self.vim_jump(line.saturating_sub(1), true, cx),
            A::Delete => match self.focus {
                FocusArea::ChecklistList => self.checklist_delete(cx),
                FocusArea::Sidebar => {
                    self.toast = Some("Use r to rename or ⌘⌫ to delete".to_string());
                }
                _ => self.delete_selected(cx),
            },
            A::Yank => self.copy_selection(cx),
            A::Paste => self.paste_clipboard(cx),
            A::Undo => self.undo(cx),
            A::Redo => self.redo(cx),
            A::Rename => match self.focus {
                FocusArea::Sidebar => self.rename_sidebar_target(cx),
                FocusArea::ChecklistList => self.checklist_begin_edit(cx),
                _ => self.begin_edit(EditField::Title, cx),
            },
            A::Edit => self.begin_edit(EditField::Title, cx),
            A::NewBelow => self.create_adjacent_task(false, cx),
            A::NewAbove => self.create_adjacent_task(true, cx),
            A::Complete => match self.focus {
                FocusArea::ChecklistList => self.checklist_toggle(cx),
                _ => self.complete_selected(true, cx),
            },
            A::Flag => self.flag_selected(cx),
            A::Indent => match self.focus {
                FocusArea::Sidebar => {
                    if !self.sidebar_expand(true, cx) {
                        self.sidebar_open_current(cx);
                    }
                }
                _ => self.indent_selected(cx),
            },
            A::Outdent => match self.focus {
                FocusArea::Sidebar => {
                    if !self.sidebar_expand(false, cx) {
                        self.sidebar_parent_move(cx);
                    }
                }
                _ => self.outdent_selected(cx),
            },
            A::Open => match self.focus {
                FocusArea::Sidebar => self.sidebar_open_current(cx),
                FocusArea::ChecklistList => self.checklist_begin_edit(cx),
                _ => self.open_selected(cx),
            },
        }
    }

    fn vim_jump(&mut self, index: usize, from_start: bool, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Sidebar => {
                let len = self.sidebar_targets().len();
                if len == 0 {
                    return;
                }
                let index = if index == usize::MAX {
                    len - 1
                } else {
                    index.min(len - 1)
                };
                self.sidebar_select(index, cx);
            }
            FocusArea::ChecklistList => {
                let len = self.checklist_ids().len();
                if len == 0 {
                    return;
                }
                let index = if index == usize::MAX {
                    len - 1
                } else {
                    index.min(len - 1)
                };
                self.checklist_cursor = Some(index);
                cx.notify();
            }
            _ => {
                let ids = self.visible_ids();
                if ids.is_empty() {
                    return;
                }
                let index = if index == usize::MAX {
                    ids.len() - 1
                } else if from_start {
                    index.min(ids.len() - 1)
                } else {
                    index.min(ids.len() - 1)
                };
                self.select_only(ids[index].clone());
                self.save_session();
                cx.notify();
            }
        }
    }

    fn search_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.search.is_empty() {
            self.toast = Some("Type / to search first".to_string());
            cx.notify();
            return;
        }
        if self.focus == FocusArea::Sidebar {
            self.focus = FocusArea::Content;
        }
        self.move_selection(if forward { 1 } else { -1 }, false);
        cx.notify();
    }

    fn open_selected(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_one() else {
            return;
        };
        let completed = self
            .store
            .db()
            .tasks
            .get(&id)
            .is_some_and(|task| task.completed);
        if completed {
            self.toggle_complete(id, cx);
        } else {
            self.begin_edit(EditField::Title, cx);
        }
    }

    fn focus_checklist_pane(&mut self, cx: &mut Context<Self>) {
        if self.selected_one().is_none() {
            self.toast = Some("Select a task first".to_string());
            cx.notify();
            return;
        }
        self.show_inspector = true;
        let len = self.checklist_ids().len();
        self.checklist_cursor = if len == 0 {
            None
        } else {
            Some(self.checklist_cursor.unwrap_or(0).min(len - 1))
        };
        self.focus = FocusArea::ChecklistList;
        self.save_session();
        cx.notify();
    }

    fn checklist_ids(&self) -> Vec<ChecklistId> {
        let Some(id) = self.selected_one() else {
            return Vec::new();
        };
        self.store
            .db()
            .tasks
            .get(&id)
            .map(|task| task.checklist.iter().map(|item| item.id.clone()).collect())
            .unwrap_or_default()
    }

    fn checklist_current(&self) -> Option<(TaskId, ChecklistId, String)> {
        let task = self.selected_one()?;
        let index = self.checklist_cursor?;
        let item = self
            .store
            .db()
            .tasks
            .get(&task)?
            .checklist
            .get(index)?
            .clone();
        Some((task, item.id, item.title))
    }

    fn checklist_move_cursor(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.checklist_ids().len();
        if len == 0 {
            self.checklist_cursor = None;
        } else {
            let current = self.checklist_cursor.unwrap_or(0) as isize;
            self.checklist_cursor = Some((current + delta).clamp(0, len as isize - 1) as usize);
        }
        cx.notify();
    }

    fn checklist_toggle(&mut self, cx: &mut Context<Self>) {
        let Some((task, item, _)) = self.checklist_current() else {
            return;
        };
        self.toggle_checklist_item(task, item, cx);
    }

    fn checklist_begin_edit(&mut self, cx: &mut Context<Self>) {
        let Some((_, item, title)) = self.checklist_current() else {
            return;
        };
        self.begin_checklist_edit(item, title, cx);
    }

    fn checklist_delete(&mut self, cx: &mut Context<Self>) {
        let Some((task, item, _)) = self.checklist_current() else {
            return;
        };
        self.delete_checklist_item(task, item, cx);
        let len = self.checklist_ids().len();
        self.checklist_cursor = if len == 0 {
            None
        } else {
            Some(self.checklist_cursor.unwrap_or(0).min(len - 1))
        };
        cx.notify();
    }

    fn checklist_reorder(&mut self, down: bool, cx: &mut Context<Self>) {
        let Some((task, item, _)) = self.checklist_current() else {
            return;
        };
        let Some(index) = self.checklist_cursor else {
            return;
        };
        let target = if down {
            index + 1
        } else {
            index.saturating_sub(1)
        };
        if target == index || target >= self.checklist_ids().len() {
            return;
        }
        self.move_checklist_item(task, item, target, cx);
        self.checklist_cursor = Some(target);
        cx.notify();
    }

    fn apply_view_defaults(&self, new_task: &mut NewTask) {
        match &self.view {
            View::Project(id) => {
                new_task.project = Some(id.clone());
                new_task.project_path = None;
            }
            View::Tag(id) => {
                if !new_task.tags.contains(id) {
                    new_task.tags.push(id.clone());
                }
            }
            _ => {}
        }
    }

    fn create_adjacent_task(&mut self, above: bool, cx: &mut Context<Self>) {
        let selected = self.selected_one();
        let mut new_task = NewTask::new("New task");
        self.apply_view_defaults(&mut new_task);
        if let Some(id) = &selected {
            if let Some(task) = self.store.db().tasks.get(id) {
                new_task.parent = task.parent.clone();
                if new_task.project.is_none() {
                    new_task.project = task.project.clone();
                }
                new_task.position = if above {
                    Position::Before(id.clone())
                } else {
                    Position::After(id.clone())
                };
            }
        } else {
            new_task.position = if above {
                Position::Start
            } else {
                Position::End
            };
        }
        if let Some(outcome) = self.execute(Command::CreateTask(new_task)) {
            if let Some(created) = outcome.created.first() {
                if let Ok(id) = self.store.db().resolve_task(created) {
                    self.select_only(id);
                    self.begin_edit(EditField::Title, cx);
                    self.field.select_all();
                }
            }
        }
        self.save_session();
        cx.notify();
    }

    fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.toast = Some("Clipboard is empty".to_string());
            cx.notify();
            return;
        };
        let lines: Vec<String> = text
            .lines()
            .map(|line| line.split('\t').next().unwrap_or(line).trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            self.toast = Some("Clipboard has no task text".to_string());
            cx.notify();
            return;
        }
        let mut created = 0;
        let mut failed = 0;
        for line in lines {
            match parse_quick(&line, now()) {
                Ok(entry) => {
                    let mut new_task = entry.into_new_task();
                    self.apply_view_defaults(&mut new_task);
                    self.execute(Command::CreateTask(new_task));
                    created += 1;
                }
                Err(_) => failed += 1,
            }
        }
        self.toast = Some(if failed == 0 {
            format!("Pasted {created} task(s)")
        } else {
            format!("Pasted {created} task(s), skipped {failed} line(s)")
        });
        self.save_session();
        cx.notify();
    }

    fn rename_sidebar_target(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.sidebar_cursor.clone() else {
            return;
        };
        let entity = match target {
            sidebar::SidebarTarget::Project(id) => modals::EntityRef::Project(id),
            sidebar::SidebarTarget::Folder(id) => modals::EntityRef::Folder(id),
            sidebar::SidebarTarget::Tag(id) => modals::EntityRef::Tag(id),
            sidebar::SidebarTarget::Custom(id) => modals::EntityRef::Perspective(id),
            sidebar::SidebarTarget::Builtin(_) => {
                self.toast = Some("Built-in perspectives cannot be renamed".to_string());
                cx.notify();
                return;
            }
        };
        let modal = modals::Modal::rename(self, entity);
        self.open_modal(modal, cx);
    }

    fn sidebar_open_current(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.sidebar_cursor.clone() else {
            return;
        };
        self.sidebar_activate(target, cx);
    }

    fn active_buffer_mut(&mut self) -> Option<&mut text::TextBuffer> {
        match self.focus {
            FocusArea::Capture => Some(&mut self.capture),
            FocusArea::Search => Some(&mut self.search),
            FocusArea::Field => Some(&mut self.field),
            FocusArea::TagInput => Some(&mut self.tag_buffer),
            FocusArea::Checklist => Some(&mut self.checklist_buffer),
            FocusArea::Modal => {
                let index = self.modal_focus;
                self.modal.as_mut().and_then(|modal| modal.field_mut(index))
            }
            FocusArea::Content => None,
            FocusArea::Sidebar | FocusArea::ChecklistList => None,
        }
    }

    fn submit_focus(&mut self, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Capture => self.commit_capture(cx),
            FocusArea::Search => {
                self.focus = FocusArea::Content;
                cx.notify();
            }
            FocusArea::Field => self.commit_field(cx),
            FocusArea::TagInput => self.add_tag_from_buffer(cx),
            FocusArea::Checklist => self.commit_checklist(cx),
            FocusArea::Modal => self.commit_modal(cx),
            FocusArea::Content | FocusArea::Sidebar | FocusArea::ChecklistList => {}
        }
    }

    fn cancel_focus(&mut self, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Capture => self.capture.clear(),
            FocusArea::Search => self.search.clear(),
            FocusArea::Field => {
                self.editing = None;
                self.field_error = None;
            }
            FocusArea::TagInput => self.tag_buffer.clear(),
            FocusArea::Checklist => {
                self.checklist_editing = None;
                self.checklist_buffer.clear();
            }
            FocusArea::Modal => {
                self.close_modal(cx);
                return;
            }
            FocusArea::Content => {}
            FocusArea::Sidebar => {
                self.sidebar_cursor = None;
            }
            FocusArea::ChecklistList => {
                self.checklist_cursor = None;
            }
        }
        self.focus = FocusArea::Content;
        cx.notify();
    }

    fn tab_focus(&mut self, backwards: bool, cx: &mut Context<Self>) {
        match self.focus {
            FocusArea::Field => {
                let next = self.editing.map(|field| {
                    if backwards {
                        field.previous()
                    } else {
                        field.next()
                    }
                });
                self.commit_field(cx);
                if let Some(next) = next {
                    self.begin_edit(next, cx);
                }
            }
            FocusArea::TagInput => {
                self.focus = FocusArea::Content;
                cx.notify();
            }
            FocusArea::Capture => {
                self.commit_capture(cx);
                self.focus = FocusArea::Content;
                cx.notify();
            }
            _ => {}
        }
    }
}

fn handle_buffer_key(
    buffer: &mut text::TextBuffer,
    stroke: &Keystroke,
    paste: Option<&str>,
) -> BufferKey {
    let mods = stroke.modifiers;
    if let Some(text) = paste {
        buffer.insert_str(text);
        return BufferKey::Changed;
    }
    if mods.platform || mods.control {
        return BufferKey::Ignored;
    }
    if mods.alt {
        match stroke.key.as_str() {
            "left" => {
                buffer.move_word_left(mods.shift);
                return BufferKey::Changed;
            }
            "right" => {
                buffer.move_word_right(mods.shift);
                return BufferKey::Changed;
            }
            _ => {}
        }
    }
    match stroke.key.as_str() {
        "left" => {
            buffer.move_left(mods.shift);
            BufferKey::Changed
        }
        "right" => {
            buffer.move_right(mods.shift);
            BufferKey::Changed
        }
        "up" => {
            buffer.home(mods.shift);
            BufferKey::Changed
        }
        "down" => {
            buffer.end(mods.shift);
            BufferKey::Changed
        }
        "home" => {
            buffer.home(mods.shift);
            BufferKey::Changed
        }
        "end" => {
            buffer.end(mods.shift);
            BufferKey::Changed
        }
        "backspace" => {
            buffer.backspace();
            BufferKey::Changed
        }
        "delete" => {
            buffer.delete_forward();
            BufferKey::Changed
        }
        "enter" | "return" => {
            if mods.shift {
                buffer.newline();
                BufferKey::Changed
            } else {
                BufferKey::Submit
            }
        }
        "escape" => BufferKey::Cancel,
        "tab" => BufferKey::Tab,
        _ => match stroke.key_char.as_deref() {
            Some(chars) if !chars.is_empty() => {
                buffer.insert_str(chars);
                BufferKey::Changed
            }
            _ => BufferKey::Ignored,
        },
    }
}

fn sid(value: impl Into<gpui::SharedString>) -> gpui::SharedString {
    value.into()
}

fn store_warning(store: &Store) -> Option<String> {
    store.warning().map(|warning| warning.to_string())
}

fn view_exists(db: &Db, view: &View) -> bool {
    match view {
        View::Builtin(_) => true,
        View::Project(id) => db.projects.contains_key(id),
        View::Folder(id) => db.folders.contains_key(id),
        View::Tag(id) => db.tags.contains_key(id),
        View::Custom(id) => db.perspectives.contains_key(id),
    }
}

fn project_in_folder(db: &Db, project: &ProjectId, folder: &FolderId) -> bool {
    if db.project_is_in_folder(project, folder, true) {
        return true;
    }
    db.projects
        .get(project)
        .and_then(|candidate| candidate.folder.as_ref())
        == Some(folder)
}

fn sort_slug(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Manual => "manual",
        SortBy::Due => "due",
        SortBy::Defer => "defer",
        SortBy::Name => "name",
        SortBy::Flagged => "flagged",
        SortBy::Estimate => "estimate",
        SortBy::Created => "created",
        SortBy::CompletedAt => "completed",
    }
}

fn sort_from_slug(value: &str) -> Option<SortBy> {
    match value {
        "manual" => Some(SortBy::Manual),
        "due" => Some(SortBy::Due),
        "defer" => Some(SortBy::Defer),
        "name" => Some(SortBy::Name),
        "flagged" => Some(SortBy::Flagged),
        "estimate" => Some(SortBy::Estimate),
        "created" => Some(SortBy::Created),
        "completed" => Some(SortBy::CompletedAt),
        _ => None,
    }
}

fn sort_label(sort: Option<SortBy>) -> &'static str {
    match sort {
        None => "View",
        Some(SortBy::Manual) => "Manual",
        Some(SortBy::Due) => "Due",
        Some(SortBy::Defer) => "Defer",
        Some(SortBy::Name) => "Name",
        Some(SortBy::Flagged) => "Flagged",
        Some(SortBy::Estimate) => "Estimate",
        Some(SortBy::Created) => "Created",
        Some(SortBy::CompletedAt) => "Completed",
    }
}

fn group_slug(group: GroupBy) -> &'static str {
    match group {
        GroupBy::None => "none",
        GroupBy::Project => "project",
        GroupBy::Folder => "folder",
        GroupBy::Tag => "tag",
        GroupBy::DueDate => "due",
        GroupBy::DeferDate => "defer",
        GroupBy::Flagged => "flagged",
        GroupBy::Estimate => "estimate",
    }
}

fn group_from_slug(value: &str) -> Option<GroupBy> {
    match value {
        "none" => Some(GroupBy::None),
        "project" => Some(GroupBy::Project),
        "folder" => Some(GroupBy::Folder),
        "tag" => Some(GroupBy::Tag),
        "due" => Some(GroupBy::DueDate),
        "defer" => Some(GroupBy::DeferDate),
        "flagged" => Some(GroupBy::Flagged),
        "estimate" => Some(GroupBy::Estimate),
        _ => None,
    }
}

fn group_label(group: Option<GroupBy>) -> &'static str {
    match group {
        None => "View",
        Some(GroupBy::None) => "None",
        Some(GroupBy::Project) => "Project",
        Some(GroupBy::Folder) => "Folder",
        Some(GroupBy::Tag) => "Tag",
        Some(GroupBy::DueDate) => "Due",
        Some(GroupBy::DeferDate) => "Defer",
        Some(GroupBy::Flagged) => "Flagged",
        Some(GroupBy::Estimate) => "Estimate",
    }
}

// ---------------------------------------------------------------- render

impl FlowlistApp {
    fn perspective_count(&self, perspective: BuiltinPerspective) -> usize {
        let db = self.store.db();
        let at = now();
        match perspective {
            BuiltinPerspective::Inbox => db.inbox_outline(false).len(),
            BuiltinPerspective::Projects => db
                .projects
                .values()
                .filter(|project| project.status.is_open())
                .count(),
            BuiltinPerspective::Forecast => {
                let today = local_date(at);
                db.tasks
                    .values()
                    .filter(|task| {
                        !task.completed
                            && (task.due.is_some_and(|stamp| stamp.local_date() <= today)
                                || task.defer.is_some_and(|stamp| stamp.local_date() <= today))
                    })
                    .count()
            }
            BuiltinPerspective::Flagged => db
                .tasks
                .values()
                .filter(|task| !task.completed && task.flagged)
                .count(),
            BuiltinPerspective::Review => query::review_projects(db, at).len(),
            BuiltinPerspective::Tags => db
                .flatten_tags()
                .into_iter()
                .filter(|row| db.tag_usage_count(&row.tag, true) > 0)
                .count(),
            BuiltinPerspective::Completed => {
                let days = db.settings.show_completed_days.max(1) as i64;
                let cutoff = at - chrono::Duration::days(days);
                db.tasks
                    .values()
                    .filter(|task| {
                        task.completed
                            && task
                                .completed_at
                                .is_some_and(|completed| completed >= cutoff)
                    })
                    .count()
            }
            BuiltinPerspective::Nearby => query::nearby(db, at).len(),
        }
    }

    fn maybe_sync(&mut self) {
        if self.last_sync.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_sync = Instant::now();
        if self.store.reload_if_changed().unwrap_or(false) {
            self.prune_selection();
        }
    }

    fn render_header(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(44.))
            .px_4()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.sidebar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_color(theme.accent)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("◈"),
                    )
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Flowlist"),
                    )
                    .child(div().text_color(theme.text_faint).child("/"))
                    .child(div().text_color(theme.text_dim).child(self.view_title())),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("undo")
                            .opacity(if self.store.can_undo() { 1.0 } else { 0.4 })
                            .tooltip(widgets::tip(theme, "Undo (⌘Z)"))
                            .child(widgets::icon_button(theme, "↶", false))
                            .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                    )
                    .child(
                        div()
                            .id("redo")
                            .opacity(if self.store.can_redo() { 1.0 } else { 0.4 })
                            .tooltip(widgets::tip(theme, "Redo (⇧⌘Z)"))
                            .child(widgets::icon_button(theme, "↷", false))
                            .on_click(cx.listener(|this, _, _, cx| this.redo(cx))),
                    )
                    .child(
                        div()
                            .id("help")
                            .tooltip(widgets::tip(theme, "Keyboard and mouse help (?)"))
                            .child(widgets::icon_button(theme, "?", false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_modal(modals::Modal::Help, cx)
                            })),
                    )
                    .child(
                        div()
                            .id("settings")
                            .tooltip(widgets::tip(theme, "Settings (⌘,)"))
                            .child(widgets::icon_button(theme, "⚙", false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_modal(modals::Modal::settings(this), cx)
                            })),
                    )
                    .child(
                        div()
                            .id("theme")
                            .tooltip(widgets::tip(theme, "Switch light / dark"))
                            .child(widgets::icon_button(theme, "◐", false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.theme = Theme::new(this.theme.mode().toggled());
                                this.save_session();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("inspector")
                            .tooltip(widgets::tip(theme, "Toggle inspector (i)"))
                            .child(widgets::icon_button(theme, "▤", self.show_inspector))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_inspector = !this.show_inspector;
                                this.save_session();
                                cx.notify();
                            })),
                    ),
            )
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        let at = now();
        let open = db.tasks.values().filter(|task| !task.completed).count();
        let available = db
            .tasks
            .values()
            .filter(|task| {
                !task.completed
                    && query::availability::availability(db, &task.id, at).is_available()
            })
            .count();
        let selected = self.selected.len();
        let undo = self
            .store
            .undo_label()
            .unwrap_or_else(|| "No undo".to_string());
        let revision = self.store.revision();
        let root = flowlist_core::paths::display_path(&self.store.paths().root);
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(26.))
            .px_4()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.sidebar)
            .text_xs()
            .text_color(theme.text_faint)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(format!("{open} open"))
                    .child(format!("{available} available"))
                    .when(selected > 0, |this| {
                        this.child(
                            div()
                                .text_color(theme.accent)
                                .child(format!("{selected} selected")),
                        )
                    })
                    .child(
                        div()
                            .id("status-undo")
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text_dim))
                            .child(undo)
                            .on_click(cx.listener(|this, _, _, cx| this.undo(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .when(self.vim_mode, |this| {
                        this.child(
                            div()
                                .text_color(if self.vim.is_pending() {
                                    theme.accent
                                } else {
                                    theme.text_faint
                                })
                                .child(if self.vim.is_pending() {
                                    "vim …"
                                } else {
                                    "vim"
                                }),
                        )
                    })
                    .child(format!("rev {revision}"))
                    .child(
                        div()
                            .max_w(px(360.))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(root),
                    ),
            )
    }

    fn render_toast(&self) -> impl IntoElement {
        let theme = self.theme;
        let message = self.toast.clone().unwrap_or_default();
        div()
            .absolute()
            .bottom(px(38.))
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .px_4()
                    .py_2()
                    .rounded_lg()
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_lg()
                    .text_sm()
                    .text_color(theme.text)
                    .child(message),
            )
    }

    fn render_empty(&self, title: &str, detail: &str) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .gap_2()
            .py_12()
            .child(div().text_color(theme.text_faint).text_2xl().child("◌"))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text_dim)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .max_w(px(340.))
                    .text_center()
                    .text_sm()
                    .text_color(theme.text_faint)
                    .child(detail.to_string()),
            )
    }
}

impl Focusable for FlowlistApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FlowlistApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.maybe_sync();
        if !self.focus_handle.is_focused(window) {
            window.focus(&self.focus_handle);
        }
        window.set_window_title(&format!("Flowlist — {}", self.view_title()));
        let theme = self.theme;

        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .text_color(theme.text)
            .font_family("SF Pro Text")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, _| window.focus(&this.focus_handle)),
            );
        root = root.child(self.render_header(window, cx));
        let mut body = div().flex().flex_1().min_h_0();
        body = body.child(self.render_sidebar(window, cx));
        body = body.child(self.render_content(window, cx));
        if self.show_inspector {
            body = body.child(self.render_inspector(window, cx));
        }
        root = root.child(body);
        root = root.child(self.render_status(window, cx));
        if self.toast.is_some() {
            root = root.child(self.render_toast());
        }
        if self.modal.is_some() {
            root = root.child(self.render_modal(window, cx));
        }
        root
    }
}

pub fn run(paths: DataPaths) -> Result<()> {
    Store::open(paths.clone())?;
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1240.), px(800.)), cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Flowlist".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let store = match Store::open(paths.clone()) {
                    Ok(store) => store,
                    Err(error) => {
                        eprintln!("could not open the Flowlist library: {error}");
                        std::process::exit(1);
                    }
                };
                let app = cx.new(|cx| FlowlistApp::new(store, cx));
                let handle = app.read(cx).focus_handle.clone();
                window.focus(&handle);
                app
            },
        );
        if let Err(error) = result {
            eprintln!("could not open the Flowlist window: {error}");
            return;
        }
        cx.activate(true);
    });
    Ok(())
}
