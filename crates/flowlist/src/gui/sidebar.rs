//! The left rail: built-in perspectives, saved perspectives, the project tree
//! and the tag tree. Projects and tags are drop targets for task drags.

use gpui::{
    AnyElement, Context, DragMoveEvent, IntoElement, MouseButton, Window, div, prelude::*, px,
};

use flowlist_core::db::Db;
use flowlist_core::ids::{ContainerId, FolderId, PerspectiveId, ProjectId, TagId};
use flowlist_core::model::{
    BuiltinPerspective, IconSpec, Perspective, Priority, ProjectStatus, SidebarOptions,
};
use flowlist_core::query;
use flowlist_core::time::now;

use super::drag::{DragFolder, DragProject, DragTask};
use super::icons::{self, Icon};
use super::modals;
use super::widgets::TipExt;
use super::{FlowlistApp, FocusArea, FocusTarget, View, widgets};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SidebarTarget {
    Builtin(BuiltinPerspective),
    Custom(PerspectiveId),
    Folder(FolderId),
    Project(ProjectId),
    Tag(TagId),
    Priority(Priority),
}

impl SidebarTarget {
    /// Stable identifier for the row, used by the right-click menu and the
    /// GUI test-state file.
    pub(super) fn key(&self) -> String {
        match self {
            SidebarTarget::Builtin(perspective) => format!("builtin:{}", perspective.slug()),
            SidebarTarget::Custom(id) => format!("custom:{id}"),
            SidebarTarget::Folder(id) => format!("folder:{id}"),
            SidebarTarget::Project(id) => format!("project:{id}"),
            SidebarTarget::Tag(id) => format!("tag:{id}"),
            SidebarTarget::Priority(priority) => format!("priority:{}", priority.slug()),
        }
    }
}

/// What a project's sidebar badge shows. A project with work in it but
/// nothing left open has earned a tick; a brand-new project keeps its 0 so the
/// row still reports how many tasks are waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProjectBadge {
    Count(usize),
    /// Tasks exist but none are open, and the project is still active.
    Stalled,
    Done,
    Dropped,
}

/// A project with no tasks shows `0`; an empty single-action bucket behaves the
/// same way. A parallel or sequential project with tasks but none left open is
/// stalled until the user marks it done or adds another action.
pub(super) fn project_badge(
    open: usize,
    total: usize,
    kind: flowlist_core::model::ProjectKind,
    status: ProjectStatus,
) -> ProjectBadge {
    match status {
        ProjectStatus::Done => ProjectBadge::Done,
        ProjectStatus::Dropped => ProjectBadge::Dropped,
        ProjectStatus::Active | ProjectStatus::OnHold => {
            if total > 0 && open == 0 && kind != flowlist_core::model::ProjectKind::SingleActions
            {
                ProjectBadge::Stalled
            } else {
                ProjectBadge::Count(open)
            }
        }
    }
}

/// The right-click menu on a sidebar row. The position is in window
/// coordinates, so the menu opens exactly under the pointer.
#[derive(Clone, Debug)]
pub(super) struct SidebarMenu {
    target: SidebarTarget,
    position: gpui::Point<gpui::Pixels>,
}

impl SidebarMenu {
    /// Stable identifier for the row the menu belongs to, used by the GUI
    /// test-state file.
    pub(super) fn key(&self) -> String {
        self.target.key()
    }
}

fn glyph(perspective: BuiltinPerspective) -> Icon {
    match perspective {
        BuiltinPerspective::Inbox => Icon::Inbox,
        BuiltinPerspective::Today => Icon::Sun,
        BuiltinPerspective::Projects => Icon::Projects,
        BuiltinPerspective::Forecast => Icon::Forecast,
        BuiltinPerspective::Flagged => Icon::Flagged,
        BuiltinPerspective::Review => Icon::Review,
        BuiltinPerspective::Tags => Icon::Tags,
        BuiltinPerspective::Completed => Icon::Completed,
        BuiltinPerspective::Archive => Icon::Archive,
        BuiltinPerspective::Changed => Icon::Clock,
        BuiltinPerspective::Attachments => Icon::Paperclip,
        BuiltinPerspective::Fullscreen => Icon::Clock,
        BuiltinPerspective::Nearby => Icon::Nearby,
        BuiltinPerspective::Trash => Icon::Trash,
    }
}

impl FlowlistApp {
    /// Wrap an element so its painted box is recorded for GUI probes. Used on a
    /// row's icon, which marks where the name starts and therefore how deep the
    /// row is nested.
    fn measured(&self, key: String, element: impl IntoElement) -> impl IntoElement {
        let boxes = self.sidebar_name_boxes.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(element)
    }

    /// Wrap a whole row so its click target is recorded for GUI probes, not
    /// just the icon slot that `measured` records.
    fn row_box(&self, key: String, element: impl IntoElement) -> impl IntoElement {
        let boxes = self.sidebar_row_boxes.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(element)
    }

    /// Wrap a count badge so the test state can report which rows show numbers
    /// and what those numbers are.
    fn count_box(&self, key: String, value: usize, element: impl IntoElement) -> impl IntoElement {
        let boxes = self.sidebar_count_boxes.clone();
        let values = self.sidebar_count_values.clone();
        values.borrow_mut().insert(key.clone(), value);
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(element)
    }

    /// The name slot. The row width belongs to the name, so long names end
    /// in an ellipsis. Row actions live in the right-click menu, which leaves
    /// the full width for names.
    ///
    /// The slot sits a hair above the flex center: the icon and the count box
    /// in these rows center on their own drawing, while a line of text carries
    /// its descender space below the baseline. Without the nudge the name
    /// reads as sitting low next to them.
    fn row_name(&self, name: impl Into<gpui::SharedString>) -> impl IntoElement {
        div()
            .flex_1()
            .min_w_0()
            .relative()
            .top(px(-2.))
            .line_clamp(1)
            .text_ellipsis()
            .child(name.into())
    }

    /// The inspector's pencil. Sidebar rows delegate their rename action to the
    /// right-click menu instead.
    pub(super) fn rename_button(
        &self,
        key: String,
        entity: modals::EntityRef,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        div()
            .id(super::sid(key))
            .px_1()
            .rounded_sm()
            .text_color(theme.text_faint)
            .cursor_pointer()
            .hover(|style| style.text_color(theme.text).bg(theme.hover))
            .flow_tip(self, cx, theme, "Rename")
            .child(icons::icon(Icon::Pencil, 12.0, theme.text_faint))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                let modal = modals::Modal::rename(this, entity.clone());
                this.open_modal(modal, cx);
            }))
    }

    /// A saved view earns its rail row only while its filter finds work. The
    /// check uses the same query the view itself runs, so completed-task and
    /// date windows agree with what opening it would show.
    pub(super) fn perspective_has_tasks(&self, perspective: &Perspective) -> bool {
        let db = self.store.db();
        query::run(
            db,
            &perspective.filter,
            perspective.sort,
            perspective.group_by,
            perspective.show_completed,
            now(),
        )
        .iter()
        .any(|group| !group.rows.is_empty() || !group.projects.is_empty())
    }

    pub(super) fn sidebar_targets(&self) -> Vec<SidebarTarget> {
        let db = self.store.db();
        let options = &db.settings.sidebar;
        let mut out = Vec::new();
        for perspective in BuiltinPerspective::ALL {
            let target = SidebarTarget::Builtin(perspective);
            if !SidebarOptions::can_hide(&target.key()) || !options.is_hidden(&target.key()) {
                out.push(target);
            }
        }
        if self.is_open("section:priorities", true) {
            for priority in Priority::ALL {
                let target = SidebarTarget::Priority(priority);
                if !options.is_hidden(&target.key()) {
                    out.push(target);
                }
            }
        }
        let saved = ordered_perspectives(db);
        if self.is_open("section:saved", true) {
            for perspective in &saved {
                let target = SidebarTarget::Custom(perspective.id.clone());
                if options.is_hidden(&target.key()) {
                    continue;
                }
                // A pinned view stays even when hide-empty would drop it.
                if !perspective.pinned
                    && self.hide_empty_views
                    && !self.perspective_has_tasks(perspective)
                {
                    continue;
                }
                out.push(target);
            }
        }
        if self.is_open("section:library", true) {
            self.container_targets(None, &mut out);
        }
        if self.is_open("section:tags", true) {
            self.tag_targets(None, &mut out);
        }
        out
    }

    fn container_targets(&self, parent: Option<FolderId>, out: &mut Vec<SidebarTarget>) {
        let db = self.store.db();
        let sidebar = &db.settings.sidebar;
        for container in db.container_children(parent) {
            match container {
                ContainerId::Folder(id) => {
                    let target = SidebarTarget::Folder(id.clone());
                    if sidebar.is_hidden(&target.key()) {
                        continue;
                    }
                    let open = self.is_open(&format!("folder:{id}"), false);
                    let child_parent = id.clone();
                    out.push(target);
                    if open {
                        self.container_targets(Some(child_parent), out);
                    }
                }
                ContainerId::Project(id) => {
                    let target = SidebarTarget::Project(id.clone());
                    if sidebar.is_hidden(&target.key()) {
                        continue;
                    }
                    if db
                        .projects
                        .get(&id)
                        .is_some_and(|project| !project.is_archived())
                    {
                        out.push(target);
                    }
                }
            }
        }
    }

    fn tag_targets(&self, parent: Option<TagId>, out: &mut Vec<SidebarTarget>) {
        let db = self.store.db();
        let sidebar = &db.settings.sidebar;
        let tags = db.tags_sorted_by_usage(parent.as_ref());
        for tag in tags {
            let target = SidebarTarget::Tag(tag.clone());
            if sidebar.is_hidden(&target.key()) {
                continue;
            }
            let open = self.is_open(&format!("tag:{tag}"), true);
            let child_parent = tag.clone();
            out.push(target);
            if open && !db.tag_children(&child_parent).is_empty() {
                self.tag_targets(Some(child_parent), out);
            }
        }
    }

    pub(super) fn target_for_view(&self, view: &View) -> Option<SidebarTarget> {
        let target = match view {
            View::Builtin(perspective) => SidebarTarget::Builtin(*perspective),
            View::Project(id) => SidebarTarget::Project(id.clone()),
            View::Folder(id) => SidebarTarget::Folder(id.clone()),
            View::Tag(id) => SidebarTarget::Tag(id.clone()),
            View::Custom(id) => SidebarTarget::Custom(id.clone()),
            View::Priority(priority) => SidebarTarget::Priority(*priority),
        };
        let key = target.key();
        if SidebarOptions::can_hide(&key) && self.store.db().settings.sidebar.is_hidden(&key) {
            None
        } else {
            Some(target)
        }
    }

    pub(super) fn sidebar_cursor_is(&self, target: &SidebarTarget) -> bool {
        self.focus == FocusArea::Sidebar && self.sidebar_cursor.as_ref() == Some(target)
    }

    pub(super) fn sidebar_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let targets = self.sidebar_targets();
        if targets.is_empty() {
            return;
        }
        let current = self
            .sidebar_cursor
            .as_ref()
            .and_then(|cursor| targets.iter().position(|target| target == cursor));
        let next = match current {
            Some(index) => (index as isize + delta).clamp(0, targets.len() as isize - 1) as usize,
            None => {
                if delta < 0 {
                    targets.len() - 1
                } else {
                    0
                }
            }
        };
        self.sidebar_cursor = Some(targets[next].clone());
        cx.notify();
    }

    pub(super) fn sidebar_select(&mut self, index: usize, cx: &mut Context<Self>) {
        let targets = self.sidebar_targets();
        if targets.is_empty() {
            return;
        }
        let index = index.min(targets.len() - 1);
        self.sidebar_cursor = Some(targets[index].clone());
        cx.notify();
    }

    pub(super) fn sidebar_activate(&mut self, target: SidebarTarget, cx: &mut Context<Self>) {
        self.sidebar_selection.clear();
        let view = match &target {
            SidebarTarget::Builtin(perspective) => View::Builtin(*perspective),
            SidebarTarget::Custom(id) => View::Custom(id.clone()),
            SidebarTarget::Folder(id) => View::Folder(id.clone()),
            SidebarTarget::Project(id) => View::Project(id.clone()),
            SidebarTarget::Tag(id) => View::Tag(id.clone()),
            SidebarTarget::Priority(priority) => View::Priority(*priority),
        };
        self.open_view(view, cx);
        self.save_session();
        self.sidebar_cursor = Some(target);
        self.focus = FocusArea::Sidebar;
        self.save_session();
        cx.notify();
    }

    /// Open a folder row. Its chevron alone expands or folds the tree.
    pub(super) fn open_folder_row(&mut self, id: FolderId, cx: &mut Context<Self>) {
        self.sidebar_activate(SidebarTarget::Folder(id), cx);
    }

    pub(super) fn sidebar_expand(&mut self, expand: bool, cx: &mut Context<Self>) -> bool {
        let Some(target) = self.sidebar_cursor.clone() else {
            return false;
        };
        let (key, has_children) = match &target {
            SidebarTarget::Folder(id) => (
                format!("folder:{id}"),
                !self
                    .store
                    .db()
                    .container_children(Some(id.clone()))
                    .is_empty(),
            ),
            SidebarTarget::Tag(id) => (
                format!("tag:{id}"),
                !self.store.db().tag_children(id).is_empty(),
            ),
            _ => return false,
        };
        let open = self.is_open(&key, false);
        if expand {
            if !open && has_children {
                self.toggle_open(key, false);
                cx.notify();
                return true;
            }
            return false;
        }
        if open && has_children {
            self.toggle_open(key, true);
            cx.notify();
            return true;
        }
        false
    }

    pub(super) fn sidebar_parent(&self) -> Option<SidebarTarget> {
        let target = self.sidebar_cursor.clone()?;
        let db = self.store.db();
        match target {
            SidebarTarget::Folder(id) => db
                .folders
                .get(&id)
                .and_then(|folder| folder.parent.clone())
                .map(SidebarTarget::Folder),
            SidebarTarget::Project(id) => db
                .projects
                .get(&id)
                .and_then(|project| project.folder.clone())
                .map(SidebarTarget::Folder),
            SidebarTarget::Tag(id) => db
                .tags
                .get(&id)
                .and_then(|tag| tag.parent.clone())
                .map(SidebarTarget::Tag),
            _ => None,
        }
    }

    pub(super) fn sidebar_parent_move(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.sidebar_parent() {
            self.sidebar_cursor = Some(parent);
            cx.notify();
        }
    }

    pub(super) fn open_sidebar_menu(
        &mut self,
        target: SidebarTarget,
        position: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.sidebar_cursor = Some(target.clone());
        self.focus = FocusArea::Sidebar;
        self.sort_menu_open = false;
        self.group_menu_open = false;
        self.priority_menu_open = false;
        self.sidebar_menu = Some(SidebarMenu { target, position });
        cx.notify();
    }

    fn close_sidebar_menu(&mut self, cx: &mut Context<Self>) {
        self.sidebar_menu = None;
        cx.notify();
    }

    fn delete_sidebar_entity(
        &mut self,
        command: flowlist_core::command::Command,
        view: View,
        cx: &mut Context<Self>,
    ) {
        self.execute(command);
        if self.view == view {
            self.view = View::Builtin(BuiltinPerspective::Inbox);
            self.selected.clear();
            self.anchor = None;
        }
        self.save_session();
        cx.notify();
    }

    fn sidebar_menu_item(
        &self,
        id: String,
        label: impl Into<String>,
        danger: bool,
        checked: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut FlowlistApp, &mut Context<FlowlistApp>) + 'static,
    ) -> AnyElement {
        let theme = self.theme;
        let boxes = self.sidebar_menu_boxes.clone();
        let box_key = id.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        box_key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .id(super::sid(id))
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_1()
            .rounded_sm()
            .text_xs()
            .text_color(if danger {
                theme.danger
            } else if checked {
                theme.accent
            } else {
                theme.text_dim
            })
            .bg(if checked {
                theme.selection
            } else {
                theme.clear
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .child(label.into())
            .when(checked, |this| {
                this.child(icons::icon(Icon::Check, 11.0, theme.accent))
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.close_sidebar_menu(cx);
                on_click(this, cx);
            }))
            .into_any_element()
    }

    fn sidebar_menu_divider(&self) -> AnyElement {
        div()
            .h(px(1.))
            .my_1()
            .w_full()
            .bg(self.theme.border_soft)
            .into_any_element()
    }

    pub(super) fn render_sidebar_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(menu) = self.sidebar_menu.as_ref() else {
            return div().into_any_element();
        };
        let target = menu.target.clone();
        let db = self.store.db();
        let mut items: Vec<AnyElement> = Vec::new();
        let target_focus = match &target {
            SidebarTarget::Project(id) => Some(FocusTarget::Project(id.clone())),
            SidebarTarget::Folder(id) => Some(FocusTarget::Folder(id.clone())),
            _ => None,
        };
        let multi = self.sidebar_selection.len() > 1
            && target_focus
                .as_ref()
                .is_some_and(|target| self.sidebar_selection.contains(target));

        if multi {
            let selection = self.sidebar_selection.clone();
            let count = selection.len();
            let projects: Vec<ProjectId> = selection
                .iter()
                .filter_map(|target| match target {
                    FocusTarget::Project(id) => Some(id.clone()),
                    FocusTarget::Folder(_) => None,
                })
                .collect();
            let folders: Vec<FolderId> = selection
                .iter()
                .filter_map(|target| match target {
                    FocusTarget::Folder(id) => Some(id.clone()),
                    FocusTarget::Project(_) => None,
                })
                .collect();
            let same_focus = self.focus_targets.len() == count
                && selection
                    .iter()
                    .all(|target| self.focus_targets.contains(target));
            items.push(self.sidebar_menu_item(
                "menu-focus-selection".to_string(),
                if same_focus {
                    "Stop focusing".to_string()
                } else {
                    format!("Focus on {count} selected")
                },
                false,
                false,
                cx,
                move |this, cx| {
                    if same_focus {
                        this.clear_focus(cx);
                    } else {
                        this.set_focus_targets(selection.clone(), cx);
                    }
                },
            ));
            if !projects.is_empty() {
                items.push(self.sidebar_menu_divider());
                for option in ProjectStatus::ALL {
                    let ids = projects.clone();
                    items.push(self.sidebar_menu_item(
                        format!("menu-selection-status:{}", option.slug()),
                        if folders.is_empty() {
                            option.label().to_string()
                        } else {
                            format!("Mark projects {}", option.label())
                        },
                        false,
                        false,
                        cx,
                        move |this, cx| {
                            this.execute(
                                flowlist_core::command::Command::SetProjectStatuses {
                                    ids: ids.clone(),
                                    status: option,
                                },
                            );
                            cx.notify();
                        },
                    ));
                }
            }
            items.push(self.sidebar_menu_divider());
            items.push(self.sidebar_menu_item(
                "menu-delete-selection".to_string(),
                format!("Delete {count} selected"),
                true,
                false,
                cx,
                move |this, cx| {
                    let modal = modals::Modal::delete_containers(
                        projects.clone(),
                        folders.clone(),
                    );
                    this.open_modal(modal, cx);
                },
            ));
        } else {
        match target.clone() {
            SidebarTarget::Builtin(perspective) => {
                let open_target = SidebarTarget::Builtin(perspective);
                items.push(self.sidebar_menu_item(
                    "menu-open-view".to_string(),
                    format!("Open {}", perspective.label()),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
                if perspective == BuiltinPerspective::Trash {
                    items.push(self.sidebar_menu_item(
                        "menu-empty-trash".to_string(),
                        "Empty Trash",
                        true,
                        false,
                        cx,
                        |this, cx| this.empty_trash(cx),
                    ));
                }
            }
            SidebarTarget::Priority(priority) => {
                let open_target = SidebarTarget::Priority(priority);
                items.push(self.sidebar_menu_item(
                    "menu-open-view".to_string(),
                    format!("Open {} priority", priority.label()),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
                if self.priority_filter == Some(priority) {
                    items.push(self.sidebar_menu_item(
                        "menu-clear-priority".to_string(),
                        "Clear priority filter",
                        false,
                        false,
                        cx,
                        |this, cx| {
                            this.priority_filter = None;
                            this.save_session();
                            cx.notify();
                        },
                    ));
                }
            }
            SidebarTarget::Custom(id) => {
                let open_target = SidebarTarget::Custom(id.clone());
                let edit_id = id.clone();
                let delete_id = id.clone();
                let name = db
                    .perspectives
                    .get(&id)
                    .map(|perspective| perspective.name.clone())
                    .unwrap_or_else(|| "view".to_string());
                items.push(self.sidebar_menu_item(
                    "menu-open-view".to_string(),
                    format!("Open “{name}”"),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
                items.push(self.sidebar_menu_item(
                    "menu-edit-view".to_string(),
                    "Edit perspective…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        if let Some(modal) = modals::Modal::perspective(this, &edit_id) {
                            this.open_modal(modal, cx);
                        }
                    },
                ));
                let pin_id = id.clone();
                let pinned = db
                    .perspectives
                    .get(&id)
                    .is_some_and(|perspective| perspective.pinned);
                items.push(self.sidebar_menu_item(
                    "menu-pin-view".to_string(),
                    if pinned {
                        "Unpin from top"
                    } else {
                        "Pin to top"
                    },
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        let Some(mut perspective) =
                            this.store.db().perspectives.get(&pin_id).cloned()
                        else {
                            return;
                        };
                        perspective.pinned = !perspective.pinned;
                        if this
                            .store
                            .execute(flowlist_core::command::Command::SavePerspective {
                                perspective,
                            })
                            .is_ok()
                        {
                            cx.notify();
                        }
                    },
                ));
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-delete-view".to_string(),
                    "Delete perspective",
                    true,
                    false,
                    cx,
                    move |this, cx| {
                        this.delete_sidebar_entity(
                            flowlist_core::command::Command::DeletePerspective {
                                id: delete_id.clone(),
                            },
                            View::Custom(delete_id.clone()),
                            cx,
                        )
                    },
                ));
            }
            SidebarTarget::Folder(id) => {
                let open_target = SidebarTarget::Folder(id.clone());
                let rename_id = id.clone();
                let edit_folder_id = id.clone();
                let delete_id = id.clone();
                let project_folder = id.clone();
                let child_folder = id.clone();
                let name = db
                    .folders
                    .get(&id)
                    .map(|folder| folder.name.clone())
                    .unwrap_or_else(|| "folder".to_string());
                let archive_projects: Vec<ProjectId> = db
                    .projects
                    .values()
                    .filter(|project| {
                        !project.is_archived() && db.project_is_in_folder(&project.id, &id, true)
                    })
                    .map(|project| project.id.clone())
                    .collect();
                let unarchive_projects: Vec<ProjectId> = db
                    .projects
                    .values()
                    .filter(|project| {
                        project.is_archived() && db.project_is_in_folder(&project.id, &id, true)
                    })
                    .map(|project| project.id.clone())
                    .collect();
                items.push(self.sidebar_menu_item(
                    "menu-open-view".to_string(),
                    format!("Open “{name}”"),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
                let focus_name = name.clone();
                let focus_target = FocusTarget::Folder(id.clone());
                let focused = self.focus_targets.len() == 1
                    && self.focus_targets[0] == focus_target;
                let selected = self.sidebar_selection.contains(&focus_target);
                let selection_count = self.sidebar_selection.len();
                items.push(self.sidebar_menu_item(
                    "menu-focus-folder".to_string(),
                    if selected && selection_count > 1 {
                        format!("Focus on {selection_count} selected")
                    } else if focused {
                        "Stop focusing".to_string()
                    } else {
                        format!("Focus on “{focus_name}”")
                    },
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        if focused && !selected {
                            this.clear_focus(cx);
                        } else {
                            this.focus_from_sidebar(focus_target.clone(), cx);
                        }
                    },
                ));
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-rename-folder".to_string(),
                    "Rename…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        let modal = modals::Modal::rename(
                            this,
                            modals::EntityRef::Folder(rename_id.clone()),
                        );
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_item(
                    "menu-edit-folder".to_string(),
                    "Edit folder…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        if let Some(modal) =
                            modals::Modal::edit_folder(this, &edit_folder_id)
                        {
                            this.open_modal(modal, cx);
                        }
                    },
                ));
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-new-project-here".to_string(),
                    "New project here…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        this.view = View::Folder(project_folder.clone());
                        let modal = modals::Modal::new_project(this);
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_item(
                    "menu-new-folder-here".to_string(),
                    "New subfolder…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        this.view = View::Folder(child_folder.clone());
                        let modal = modals::Modal::new_folder(this);
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_divider());
                if !archive_projects.is_empty() {
                    let ids = archive_projects.clone();
                    items.push(self.sidebar_menu_item(
                        "menu-archive-folder-projects".to_string(),
                        "Archive projects",
                        false,
                        false,
                        cx,
                        move |this, cx| {
                            this.execute(flowlist_core::command::Command::ArchiveProjects {
                                ids: ids.clone(),
                                archived: true,
                            });
                            cx.notify();
                        },
                    ));
                }
                if !unarchive_projects.is_empty() {
                    let ids = unarchive_projects.clone();
                    items.push(self.sidebar_menu_item(
                        "menu-unarchive-folder-projects".to_string(),
                        "Unarchive projects",
                        false,
                        false,
                        cx,
                        move |this, cx| {
                            this.execute(flowlist_core::command::Command::ArchiveProjects {
                                ids: ids.clone(),
                                archived: false,
                            });
                            cx.notify();
                        },
                    ));
                }
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-delete-folder".to_string(),
                    "Delete folder",
                    true,
                    false,
                    cx,
                    move |this, cx| {
                        let modal = modals::Modal::delete_folder(this, delete_id.clone());
                        this.open_modal(modal, cx);
                    },
                ));
            }
            SidebarTarget::Project(id) => {
                let open_target = SidebarTarget::Project(id.clone());
                let edit_id = id.clone();
                let edit_project_id = id.clone();
                let delete_id = id.clone();
                let name = db
                    .projects
                    .get(&id)
                    .map(|project| project.name.clone())
                    .unwrap_or_else(|| "project".to_string());
                let status = db.projects.get(&id).map(|project| project.status);
                let archived = db
                    .projects
                    .get(&id)
                    .is_some_and(|project| project.is_archived());
                items.push(self.sidebar_menu_item(
                    "menu-open-project".to_string(),
                    format!("Open “{name}”"),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
                let focus_name = name.clone();
                let focus_target = FocusTarget::Project(id.clone());
                let focused = self.focus_targets.len() == 1
                    && self.focus_targets[0] == focus_target;
                let selected = self.sidebar_selection.contains(&focus_target);
                let selection_count = self.sidebar_selection.len();
                items.push(self.sidebar_menu_item(
                    "menu-focus-project".to_string(),
                    if selected && selection_count > 1 {
                        format!("Focus on {selection_count} selected")
                    } else if focused {
                        "Stop focusing".to_string()
                    } else {
                        format!("Focus on “{focus_name}”")
                    },
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        if focused && !selected {
                            this.clear_focus(cx);
                        } else {
                            this.focus_from_sidebar(focus_target.clone(), cx);
                        }
                    },
                ));
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-rename-project".to_string(),
                    "Rename…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        let modal = modals::Modal::rename(
                            this,
                            modals::EntityRef::Project(edit_id.clone()),
                        );
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_item(
                    "menu-edit-project".to_string(),
                    "Edit project…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        if let Some(modal) =
                            modals::Modal::edit_project(this, &edit_project_id)
                        {
                            this.open_modal(modal, cx);
                        }
                    },
                ));
                items.push(self.sidebar_menu_divider());
                for option in ProjectStatus::ALL {
                    let status_id = id.clone();
                    items.push(self.sidebar_menu_item(
                        format!("menu-project-status:{}", option.slug()),
                        option.label(),
                        false,
                        status == Some(option),
                        cx,
                        move |this, cx| {
                            this.execute(flowlist_core::command::Command::SetProjectStatus {
                                id: status_id.clone(),
                                status: option,
                            });
                            cx.notify();
                        },
                    ));
                }
                items.push(self.sidebar_menu_divider());
                {
                    let archive_id = id.clone();
                    items.push(self.sidebar_menu_item(
                        "menu-archive-project".to_string(),
                        if archived {
                            "Unarchive project"
                        } else {
                            "Archive project"
                        },
                        false,
                        false,
                        cx,
                        move |this, cx| {
                            this.execute(flowlist_core::command::Command::ArchiveProjects {
                                ids: vec![archive_id.clone()],
                                archived: !archived,
                            });
                            cx.notify();
                        },
                    ));
                }
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-delete-project".to_string(),
                    "Delete project",
                    true,
                    false,
                    cx,
                    move |this, cx| {
                        let modal = modals::Modal::delete_project(this, delete_id.clone());
                        this.open_modal(modal, cx);
                    },
                ));
            }
            SidebarTarget::Tag(id) => {
                let open_target = SidebarTarget::Tag(id.clone());
                let rename_id = id.clone();
                let delete_id = id.clone();
                let location_id = id.clone();
                let child_tag = id.clone();
                let name = db
                    .tags
                    .get(&id)
                    .map(|tag| tag.name.clone())
                    .unwrap_or_else(|| "tag".to_string());
                items.push(self.sidebar_menu_item(
                    "menu-open-tag".to_string(),
                    format!("Open “{name}”"),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
                items.push(self.sidebar_menu_item(
                    "menu-rename-tag".to_string(),
                    "Rename…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        let modal =
                            modals::Modal::rename(this, modals::EntityRef::Tag(rename_id.clone()));
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_item(
                    "menu-locate-tag".to_string(),
                    "Set location…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        let modal = modals::Modal::tag_location(this, location_id.clone());
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_item(
                    "menu-new-subtag".to_string(),
                    "New subtag…",
                    false,
                    false,
                    cx,
                    move |this, cx| {
                        this.view = View::Tag(child_tag.clone());
                        let modal = modals::Modal::new_tag(this);
                        this.open_modal(modal, cx);
                    },
                ));
                items.push(self.sidebar_menu_divider());
                items.push(self.sidebar_menu_item(
                    "menu-delete-tag".to_string(),
                    "Delete tag",
                    true,
                    false,
                    cx,
                    move |this, cx| {
                        let modal = modals::Modal::delete_tag(this, &delete_id);
                        this.open_modal(modal, cx);
                    },
                ));
            }
        }
        }

        let menu_div = div()
            .id("sidebar-menu")
            .min_w(px(210.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg()
            .children(items);
        let viewport = self.window_size.unwrap_or((1254.0, 1404.0));
        let x = f32::from(menu.position.x).min((viewport.0 - 210.).max(0.));
        let y = f32::from(menu.position.y).min((viewport.1 - 240.).max(0.));
        div()
            .id("sidebar-menu-layer")
            .absolute()
            .occlude()
            .left(px(x))
            .top(px(y))
            .child(menu_div)
            .into_any_element()
    }

    pub(super) fn render_sidebar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        // Rebuilt every frame so a badge that disappears cannot leave a stale
        // value behind for the next view or focus scope. Boxes stay measured
        // across frames; they are hit targets, not values.
        self.sidebar_count_values.borrow_mut().clear();
        let options = self.store.db().settings.sidebar.clone();
        // A project drag owns the sidebar highlight only while it is live.
        if !cx.has_active_drag() {
            self.sidebar_drag_target = None;
        }
        let mut rail = div()
            .flex()
            .flex_col()
            .w(px(super::SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .bg(theme.glass_fill(180.))
            .border_r_1()
            .border_color(theme.border)
            .text_sm()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.focus = FocusArea::Sidebar;
                    if this.sidebar_cursor.is_none() {
                        this.sidebar_cursor = this.target_for_view(&this.view);
                    }
                    cx.notify();
                }),
            )
            // Capture-phase drag handlers run shallowest-first, so the rail
            // clears the highlight and each row below sets it again. Typing the
            // listeners to the project and folder payloads means task drags
            // never touch this state.
            .on_drag_move::<DragProject>(cx.listener(|this, _, _, cx| {
                this.set_drag_target(None, cx);
            }))
            .on_drag_move::<DragFolder>(cx.listener(|this, _, _, cx| {
                this.set_drag_target(None, cx);
            }));

        // Perspectives are the rail's backbone: Inbox, Today and Trash cannot
        // be hidden, and the rest come from the per-item Settings matrix.
        rail = rail.child(widgets::section_label(theme, "PERSPECTIVES"));
        for perspective in BuiltinPerspective::ALL {
            let target = SidebarTarget::Builtin(perspective);
            if !SidebarOptions::can_hide(&target.key()) || !options.is_hidden(&target.key()) {
                rail = rail.child(self.perspective_row(perspective, cx));
            }
        }

        let visible: Vec<Priority> = Priority::ALL
            .into_iter()
            .filter(|priority| !options.is_hidden(&SidebarTarget::Priority(*priority).key()))
            .collect();
        if !visible.is_empty() {
            rail = rail.child(self.sidebar_section("priorities", "PRIORITIES", cx));
            if self.is_open("section:priorities", true) {
                for priority in visible {
                    rail = rail.child(self.priority_row(priority, cx));
                }
            }
        }

        let mut lower = div()
            .id("sidebar-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.sidebar_scroll)
            // Dropping a project on empty sidebar space takes it out of its
            // folder, the same as the LIBRARY heading. The heading and folder
            // rows paint after this element, so their handlers run later in the
            // capture phase and win.
            .on_drag_move::<DragProject>(cx.listener(
                |this, event: &DragMoveEvent<DragProject>, _, cx| {
                    if event.bounds.contains(&event.event.position) {
                        this.set_drag_target(Some("library".to_string()), cx);
                    }
                },
            ))
            .on_drop::<DragProject>(
                cx.listener(|this, payload: &DragProject, _, cx| this.drop_on_library(payload, cx)),
            )
            .on_drag_move::<DragFolder>(cx.listener(
                |this, event: &DragMoveEvent<DragFolder>, _, cx| {
                    if event.bounds.contains(&event.event.position) {
                        this.set_drag_target(Some("library".to_string()), cx);
                    }
                },
            ))
            .on_drop::<DragFolder>(cx.listener(|this, payload: &DragFolder, _, cx| {
                this.drop_folder_on_library(payload, cx)
            }));

        let saved = ordered_perspectives(self.store.db());
        let custom: Vec<PerspectiveRow> = saved
            .into_iter()
            .filter(|perspective| {
                !options.is_hidden(&format!("custom:{}", perspective.id))
            })
            .filter(|perspective| {
                perspective.pinned
                    || !self.hide_empty_views
                    || self.perspective_has_tasks(perspective)
            })
            .map(|perspective| PerspectiveRow {
                id: perspective.id.clone(),
                name: perspective.name.clone(),
                pinned: perspective.pinned,
                icon: perspective.icon,
            })
            .collect();
        if !custom.is_empty() {
            lower = lower.child(self.sidebar_section("saved", "SAVED", cx));
            if self.is_open("section:saved", true) {
                for row in custom {
                    lower = lower.child(self.custom_perspective_row(row, cx));
                }
            }
        }

        // Dropping a project on the heading takes it out of its folder.
        let library_rows = self.store.db().flatten_containers();
        let library_empty = library_rows.is_empty();
        let library_visible = {
            let db = self.store.db();
            let sidebar = &db.settings.sidebar;
            library_rows.iter().any(|row| match &row.container {
                ContainerId::Folder(id) => !sidebar.is_hidden(&format!("folder:{id}")),
                ContainerId::Project(id) => {
                    db.projects
                        .get(id)
                        .is_some_and(|project| !project.is_archived())
                        && !sidebar.is_hidden(&format!("project:{id}"))
                }
            })
        };
        if library_visible || library_empty {
            let library_active = self.sidebar_drag_target.as_deref() == Some("library");
            let library_boxes = self.sidebar_row_boxes.clone();
            lower = lower.child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            library_boxes.borrow_mut().insert(
                                "library".to_string(),
                                [
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ],
                            );
                        }
                    })
                    .on_drag_move::<DragProject>(cx.listener(
                        |this, event: &DragMoveEvent<DragProject>, _, cx| {
                            if event.bounds.contains(&event.event.position) {
                                this.set_drag_target(Some("library".to_string()), cx);
                            }
                        },
                    ))
                    .on_drop::<DragProject>(cx.listener(|this, payload: &DragProject, _, cx| {
                        this.drop_on_library(payload, cx)
                    }))
                    .on_drag_move::<DragFolder>(cx.listener(
                        |this, event: &DragMoveEvent<DragFolder>, _, cx| {
                            if event.bounds.contains(&event.event.position) {
                                this.set_drag_target(Some("library".to_string()), cx);
                            }
                        },
                    ))
                    .on_drop::<DragFolder>(cx.listener(|this, payload: &DragFolder, _, cx| {
                        this.drop_folder_on_library(payload, cx)
                    }))
                    .child(
                        div()
                            .id("sidebar-library")
                            .when(library_active, |this| this.rounded_md().bg(theme.selection))
                            .child(self.sidebar_section("library", "LIBRARY", cx)),
                    ),
            );
            if self.is_open("section:library", true) {
                lower = lower.children(self.container_rows(None, 0, cx));
            }
        }

        let visible_tags = {
            let db = self.store.db();
            let sidebar = &db.settings.sidebar;
            db.tags
                .keys()
                .any(|id| !sidebar.is_hidden(&format!("tag:{id}")))
        };
        if visible_tags {
            lower = lower.child(self.sidebar_section("tags", "TAGS", cx));
            if self.is_open("section:tags", true) {
                let (empty, has_general, has_geotags) = {
                    let db = self.store.db();
                    let sidebar = &db.settings.sidebar;
                    let roots: Vec<TagId> = db
                        .tags_sorted_by_usage(None)
                        .into_iter()
                        .filter(|id| !sidebar.is_hidden(&format!("tag:{id}")))
                        .collect();
                    (
                        roots.is_empty(),
                        roots.iter().any(|id| {
                            db.tags.get(id).is_some_and(|tag| tag.location.is_none())
                        }),
                        roots.iter().any(|id| {
                            db.tags.get(id).is_some_and(|tag| tag.location.is_some())
                        }),
                    )
                };
                if empty {
                    lower = lower.child(
                        div()
                            .px_3()
                            .pb_2()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("No tags yet — add one with #tag in quick entry."),
                    );
                } else {
                    // Only label the split when both kinds exist. A library
                    // with only ordinary tags should read as a plain tag list,
                    // and likewise for a library with only geotags.
                    let show_group_labels = has_general && has_geotags;
                    if has_general {
                        if show_group_labels {
                            lower = lower.child(tag_group_label("GENERAL", theme));
                        }
                        lower = lower.children(self.tag_rows_filtered(None, Some(false), cx));
                    }
                    if has_geotags {
                        if show_group_labels {
                            lower = lower.child(tag_group_label("GEOTAGS", theme));
                        }
                        lower = lower.children(self.tag_rows_filtered(None, Some(true), cx));
                    }
                }
            }
        }

        rail = rail.child(lower);
        // Two explicit rows keep the four creation actions aligned instead
        // of letting wrap count decide the layout at different rail widths.
        let foot_row = |left: gpui::AnyElement, right: gpui::AnyElement| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().flex_1().child(left))
                .child(div().flex_1().child(right))
        };
        rail.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .border_t_1()
                .border_color(theme.border_soft)
                .child(foot_row(
                    div()
                        .id("new-project")
                        .child(widgets::ghost_button_with_icon(
                            theme,
                            Icon::Plus,
                            "Project",
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_project(this), cx)
                        }))
                        .into_any_element(),
                    div()
                        .id("new-folder")
                        .child(widgets::ghost_button_with_icon(theme, Icon::Plus, "Folder"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_folder(this), cx)
                        }))
                        .into_any_element(),
                ))
                .child(foot_row(
                    div()
                        .id("new-tag")
                        .child(widgets::ghost_button_with_icon(theme, Icon::Plus, "Tag"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_tag(this), cx)
                        }))
                        .into_any_element(),
                    div()
                        .id("new-perspective")
                        .child(widgets::ghost_button_with_icon(
                            theme,
                            Icon::Perspective,
                            "View",
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_perspective(this), cx)
                        }))
                        .into_any_element(),
                )),
        )
    }

    /// A collapsible section heading in the rail. Sections default open and
    /// remember their fold in the same in-session state as folders and groups.
    fn sidebar_section(
        &self,
        key: &'static str,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let state_key = format!("section:{key}");
        let open = self.is_open(&state_key, true);
        let toggle_key = state_key.clone();
        let boxes = self.sidebar_section_boxes.clone();
        let box_key = key.to_string();
        let group = format!("sidebar-section:{key}");
        let chevron_group = group.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        box_key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(
                div()
                    .id(super::sid(format!("sidebar-section:{key}")))
                    .group(group)
                    .flex()
                    .items_center()
                    .gap_1()
                    .mx_2()
                    .px_1()
                    .pt_3()
                    .pb_1()
                    .rounded_md()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text_faint)
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.text_dim))
                    .flow_tip(
                        self,
                        cx,
                        theme,
                        if open {
                            format!("Collapse {label}")
                        } else {
                            format!("Expand {label}")
                        },
                    )
                    .child(label)
                    .child(div().flex_1())
                    // The disclosure stays out of the way until the pointer is
                    // on the heading, so a resting rail reads as labels only.
                    .child(
                        div()
                            .opacity(0.0)
                            .group_hover(chevron_group, |style| style.opacity(1.0))
                            .child(icons::icon(
                                if open {
                                    Icon::ChevronDown
                                } else {
                                    Icon::ChevronRight
                                },
                                11.0,
                                theme.text_faint,
                            )),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_expanded(toggle_key.clone(), true, cx)
                    })),
            )
    }

    fn perspective_row(
        &self,
        perspective: BuiltinPerspective,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let active = self.view == View::Builtin(perspective);
        let count = self.perspective_count(perspective);
        let key = SidebarTarget::Builtin(perspective).key();
        let show_count = !self.store.db().settings.sidebar.is_count_hidden(&key);
        let icon_color = if active { theme.accent } else { theme.text_dim };
        let cursor = self.sidebar_cursor_is(&SidebarTarget::Builtin(perspective));
        let row = div()
            .id(super::sid(format!("perspective:{}", perspective.slug())))
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .mx_2()
            .px_2()
            .rounded_md()
            .bg(if active { theme.selection } else { theme.clear })
            .when(cursor, |this| this.bg(theme.hover))
            .text_color(if active { theme.accent } else { theme.text_dim })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .child(
                div()
                    .w(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icons::icon(glyph(perspective), 15.0, icon_color)),
            )
            .child(div().flex_1().min_w_0().child(perspective.label()))
            .when(show_count && count > 0, |this| {
                this.child(self.count_box(key.clone(), count, widgets::count(theme, count)))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    this.open_sidebar_menu(SidebarTarget::Builtin(perspective), event.position, cx)
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.sidebar_activate(SidebarTarget::Builtin(perspective), cx)
            }));
        self.row_box(format!("builtin:{}", perspective.slug()), row)
    }

    fn priority_row(&self, priority: Priority, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let active = self.view == View::Priority(priority);
        let count = self
            .store
            .db()
            .tasks
            .values()
            .filter(|task| {
                !task.completed
                    && !task.is_trashed()
                    && task.priority == Some(priority)
                    && self.focus_allows_task(&task.id)
            })
            .count();
        let key = SidebarTarget::Priority(priority).key();
        let show_count = !self.store.db().settings.sidebar.is_count_hidden(&key);
        let color = priority_color(theme, priority);
        let icon_color = if active { theme.accent } else { color };
        let cursor = self.sidebar_cursor_is(&SidebarTarget::Priority(priority));
        let row = div()
            .id(super::sid(format!("priority:{}", priority.slug())))
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .mx_2()
            .px_2()
            .rounded_md()
            .bg(if active { theme.selection } else { theme.clear })
            .when(cursor, |this| this.bg(theme.hover))
            .text_color(if active { theme.accent } else { theme.text_dim })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .child(
                div()
                    .w(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icons::icon(Icon::Flagged, 14.0, icon_color)),
            )
            .child(div().flex_1().min_w_0().child(priority.label()))
            .when(show_count && count > 0, |this| {
                this.child(self.count_box(key.clone(), count, widgets::count(theme, count)))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    this.open_sidebar_menu(SidebarTarget::Priority(priority), event.position, cx)
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.sidebar_activate(SidebarTarget::Priority(priority), cx)
            }));
        self.row_box(format!("priority:{}", priority.slug()), row)
    }

    fn custom_perspective_row(
        &self,
        row: PerspectiveRow,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let active = self.view == View::Custom(row.id.clone());
        let icon_color = if active { theme.accent } else { theme.text_dim };
        let count: usize = self
            .store
            .db()
            .perspectives
            .get(&row.id)
            .map(|perspective| {
                query::custom(self.store.db(), perspective, now())
                    .iter()
                    .map(|group| {
                        group
                            .rows
                            .iter()
                            .filter(|row| self.focus_allows_task(&row.id))
                            .count()
                            + group
                                .projects
                                .iter()
                                .filter(|project| self.focus_allows_project(project))
                                .count()
                    })
                    .sum()
            })
            .unwrap_or(0);
        let count_key = format!("custom:{}", row.id);
        let show_count = !self
            .store
            .db()
            .settings
            .sidebar
            .is_count_hidden(&count_key);
        let id = row.id.clone();
        let pinned = row.pinned;
        let target_id = row.id.clone();
        let cursor = self.sidebar_cursor_is(&SidebarTarget::Custom(target_id.clone()));
        let row_boxes = self.sidebar_row_boxes.clone();
        let row_key = format!("custom:{}", row.id);
        let row = div()
            .id(super::sid(format!("custom:{}", row.id)))
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .mx_2()
            .px_2()
            .rounded_md()
            .bg(if active { theme.selection } else { theme.clear })
            .when(cursor, |this| this.bg(theme.hover))
            .text_color(if active { theme.accent } else { theme.text_dim })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .child(
                div()
                    .w(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(match row.icon {
                        Some(icon) => {
                            widgets::icon_spec(&icon, 14.0, icon_color).into_any_element()
                        }
                        None if pinned => {
                            icons::icon(Icon::Pin, 14.0, icon_color).into_any_element()
                        }
                        None => icons::icon(Icon::Perspective, 14.0, icon_color)
                            .into_any_element(),
                    }),
            )
            .child(self.row_name(row.name))
            .when(show_count && count > 0, |this| {
                this.child(self.count_box(
                    count_key,
                    count,
                    widgets::count(theme, count),
                ))
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    this.open_sidebar_menu(
                        SidebarTarget::Custom(target_id.clone()),
                        event.position,
                        cx,
                    )
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.sidebar_activate(SidebarTarget::Custom(id.clone()), cx)
            }));
        // Saved perspectives are rows too, so probes can reach one by id
        // instead of guessing at a y offset below the built-in views.
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    row_boxes.borrow_mut().insert(
                        row_key.clone(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            })
            .child(row)
    }

    fn container_rows(
        &self,
        parent: Option<FolderId>,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let db = self.store.db();
        let sidebar = &db.settings.sidebar;
        let children = db.container_children(parent.clone()).to_vec();
        let mut rows = Vec::new();
        for container in children {
            match container {
                ContainerId::Folder(id) => {
                    let key = format!("folder:{id}");
                    if sidebar.is_hidden(&key) {
                        continue;
                    }
                    // Folders start folded on every launch; opening one is a
                    // deliberate act and the in-session tree remembers it.
                    let open = self.is_open(&key, false);
                    // The tree's indentation already carries the path, so a
                    // row shows its own name rather than "Parent / Child".
                    let name = db
                        .folders
                        .get(&id)
                        .map(|folder| folder.name.clone())
                        .unwrap_or_default();
                    let count = db
                        .projects
                        .values()
                        .filter(|project| {
                            !project.is_archived()
                                && project.status.is_open()
                                && db.project_is_in_folder(&project.id, &id, true)
                                && !db.project_is_ticked(&project.id)
                                && self.focus_allows_project(&project.id)
                        })
                        .count();
                    let show_count = !sidebar.is_count_hidden(&key);
                    let view_id = id.clone();
                    let target_id = id.clone();
                    let toggle_key = key.clone();
                    let active = self.view == View::Folder(id.clone());
                    let cursor = self.sidebar_cursor_is(&SidebarTarget::Folder(target_id.clone()));
                    let focus_selected =
                        self.sidebar_selection.contains(&FocusTarget::Folder(id.clone()));
                    let move_key = key.clone();
                    let folder_move_key = key.clone();
                    let folder_move_target = id.clone();
                    let drag_folder_id = id.clone();
                    let drag_folder_label = name.clone();
                    let drag_theme = self.theme;
                    let drop_here = self.sidebar_drag_target.as_deref() == Some(key.as_str());
                    let mut row = div()
                        .id(super::sid(format!("sidebar-folder:{id}")))
                        .flex()
                        .items_center()
                        .gap_1()
                        .h(px(26.))
                        .mx_2()
                        .pl(px(4.0 + depth as f32 * 14.0))
                        .pr_2()
                        .rounded_md()
                        .text_color(self.theme.text_dim)
                        .cursor_pointer()
                        .when(drop_here, |this| {
                            this.bg(self.theme.selection).text_color(self.theme.accent)
                        })
                        .hover(|style| style.bg(self.theme.hover))
                        .on_drag_move::<DragProject>(cx.listener(
                            move |this, event: &DragMoveEvent<DragProject>, _, cx| {
                                if event.bounds.contains(&event.event.position) {
                                    this.set_drag_target(Some(move_key.clone()), cx);
                                }
                            },
                        ))
                        // A folder lands on another folder to nest, but never
                        // on itself or on one of its own children.
                        .on_drag_move::<DragFolder>(cx.listener(
                            move |this, event: &DragMoveEvent<DragFolder>, _, cx| {
                                if event.bounds.contains(&event.event.position) {
                                    let dragged = event.drag(cx).id.clone();
                                    if this.folder_can_hold(&dragged, &folder_move_target) {
                                        this.set_drag_target(Some(folder_move_key.clone()), cx);
                                    } else {
                                        this.set_drag_target(None, cx);
                                    }
                                }
                            },
                        ))
                        .when(cursor, |this| this.bg(self.theme.hover))
                        .child(
                            div()
                                .id(super::sid(format!("folder-toggle:{id}")))
                                .w(px(14.))
                                .text_color(self.theme.text_faint)
                                .flow_tip(
                                    self,
                                    cx,
                                    self.theme,
                                    if open {
                                        "Collapse folder"
                                    } else {
                                        "Expand folder"
                                    },
                                )
                                .child(icons::icon(
                                    if open {
                                        Icon::ChevronDown
                                    } else {
                                        Icon::ChevronRight
                                    },
                                    12.0,
                                    self.theme.text_faint,
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.toggle_expanded(toggle_key.clone(), false, cx);
                                })),
                        )
                        .child(
                            self.measured(
                                format!("folder:{id}"),
                                div()
                                    .w(px(16.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(icons::icon(
                                        if open { Icon::FolderOpen } else { Icon::Folder },
                                        14.0,
                                        self.theme.text_faint,
                                    )),
                            ),
                        )
                        .child(self.row_name(name))
                        .when(focus_selected, |this| {
                            this.child(
                                div()
                                    .size(px(6.))
                                    .rounded_full()
                                    .bg(self.theme.accent),
                            )
                        })
                        .when(show_count && count > 0, |this| {
                            this.child(self.count_box(
                                format!("folder:{id}"),
                                count,
                                widgets::count(self.theme, count),
                            ))
                        })
                        .on_drag(
                            DragFolder::folder(drag_folder_id.clone(), drag_folder_label.clone()),
                            move |payload, position, window, cx| {
                                super::drag::folder_ghost(payload, position, window, cx, drag_theme)
                            },
                        )
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                this.open_sidebar_menu(
                                    SidebarTarget::Folder(target_id.clone()),
                                    event.position,
                                    cx,
                                )
                            }),
                        )
                        .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                            if event.modifiers().platform {
                                this.toggle_sidebar_selection(
                                    FocusTarget::Folder(view_id.clone()),
                                    cx,
                                );
                            } else {
                                this.clear_sidebar_selection(cx);
                                this.open_folder_row(view_id.clone(), cx);
                            }
                        }));
                    if active {
                        row = row.bg(self.theme.selection).text_color(self.theme.accent);
                    }
                    if !self.focus_targets.is_empty()
                        && !self
                            .focus_targets
                            .iter()
                            .any(|target| target.allows_folder(db, &id))
                    {
                        row = row.opacity(0.38);
                    }
                    let anchor_boxes = self.sidebar_row_boxes.clone();
                    let anchor_key = format!("folder:{id}");
                    let drop_folder = id.clone();
                    let drop_folder_onto = id.clone();
                    rows.push(
                        div()
                            .on_children_prepainted(move |bounds, _, _| {
                                if let Some(bounds) = bounds.first() {
                                    anchor_boxes.borrow_mut().insert(
                                        anchor_key.clone(),
                                        [
                                            f32::from(bounds.origin.x),
                                            f32::from(bounds.origin.y),
                                            f32::from(bounds.size.width),
                                            f32::from(bounds.size.height),
                                        ],
                                    );
                                }
                            })
                            .on_drop::<DragProject>(cx.listener(
                                move |this, payload: &DragProject, _, cx| {
                                    this.drop_on_folder(drop_folder.clone(), payload, cx)
                                },
                            ))
                            .on_drop::<DragFolder>(cx.listener(
                                move |this, payload: &DragFolder, _, cx| {
                                    this.drop_folder_on_folder(
                                        drop_folder_onto.clone(),
                                        payload,
                                        cx,
                                    )
                                },
                            ))
                            .child(row)
                            .into_any_element(),
                    );
                    if open {
                        rows.extend(self.container_rows(Some(id), depth + 1, cx));
                    }
                }
                ContainerId::Project(id) => {
                    let Some(project) = db.projects.get(&id) else {
                        continue;
                    };
                    if project.is_archived() || sidebar.is_hidden(&format!("project:{id}")) {
                        continue;
                    }
                    let focused = self.focus_allows_project(&id);
                    let (done, total) = if focused {
                        db.project_progress(&id)
                    } else {
                        (0, 0)
                    };
                    let open = if focused { db.project_open_count(&id) } else { 0 };
                    let badge =
                        project_badge(open, total, project.effective_kind(), project.status);
                    let show_count = !sidebar.is_count_hidden(&format!("project:{id}"));
                    let show_badge = project.status == ProjectStatus::OnHold
                        || matches!(badge, ProjectBadge::Done | ProjectBadge::Dropped)
                        || (show_count && open > 0);
                    let view_id = id.clone();
                    let target_id = id.clone();
                    let drop_id = id.clone();
                    let active = self.view == View::Project(id.clone());
                    let focus_selected =
                        self.sidebar_selection.contains(&FocusTarget::Project(id.clone()));
                    let cursor = self.sidebar_cursor_is(&SidebarTarget::Project(target_id.clone()));
                    let drag_id = id.clone();
                    let drag_theme = self.theme;
                    let mut row = div()
                        .id(super::sid(format!("sidebar-project:{id}")))
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(px(26.))
                        .mx_2()
                        .pl(px(20.0 + depth as f32 * 14.0))
                        .pr_2()
                        .rounded_md()
                        .bg(if active {
                            self.theme.selection
                        } else {
                            self.theme.clear
                        })
                        .when(cursor, |this| this.bg(self.theme.hover))
                        .text_color(if active {
                            self.theme.accent
                        } else {
                            self.theme.text
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(self.theme.hover))
                        // Project rows are not project drop targets. This
                        // paints after the scroller, so it overwrites the
                        // scroller's Library target for one row.
                        .on_drag_move::<DragProject>(cx.listener(
                            |this, event: &DragMoveEvent<DragProject>, _, cx| {
                                if event.bounds.contains(&event.event.position) {
                                    this.set_drag_target(None, cx);
                                }
                            },
                        ))
                        .on_drag_move::<DragFolder>(cx.listener(
                            |this, event: &DragMoveEvent<DragFolder>, _, cx| {
                                if event.bounds.contains(&event.event.position) {
                                    this.set_drag_target(None, cx);
                                }
                            },
                        ))
                        .child(self.measured(
                            format!("project:{id}"),
                            widgets::icon_spec(
                                &project.icon,
                                14.0,
                                if active {
                                    self.theme.accent
                                } else {
                                    self.theme.text_faint
                                },
                            ),
                        ))
                        .child(self.row_name(project.name.clone()))
                        .when(focus_selected, |this| {
                            this.child(
                                div()
                                    .size(px(6.))
                                    .rounded_full()
                                    .bg(self.theme.accent),
                            )
                        })
                        .when(show_badge, |this| {
                            this.child({
                                let tip = if project.status == ProjectStatus::OnHold {
                                    "On hold · tasks cannot be completed".to_string()
                                } else {
                                    match badge {
                                        ProjectBadge::Stalled => {
                                            format!("All caught up · {done} of {total} done")
                                        }
                                        ProjectBadge::Done => "Done".to_string(),
                                        ProjectBadge::Dropped => "Dropped".to_string(),
                                        ProjectBadge::Count(_) => {
                                            format!("{open} open · {done} of {total} done")
                                        }
                                    }
                                };
                                let badge_id = if project.status == ProjectStatus::OnHold {
                                    format!("project-hold:{id}")
                                } else {
                                    format!("project-count:{id}")
                                };
                                div()
                                    .id(super::sid(badge_id))
                                    .flow_tip(self, cx, self.theme, tip)
                                    .child(if project.status == ProjectStatus::OnHold {
                                        div()
                                            .min_w(px(18.))
                                            .px_1()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(icons::icon(
                                                Icon::Pause,
                                                11.0,
                                                self.theme.warning,
                                            ))
                                            .into_any_element()
                                    } else {
                                        match badge {
                                        ProjectBadge::Stalled => div()
                                            .min_w(px(18.))
                                            .into_any_element(),
                                        ProjectBadge::Done => div()
                                            .min_w(px(18.))
                                            .px_1()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(icons::icon(
                                                Icon::Check,
                                                11.0,
                                                self.theme.success,
                                            ))
                                            .into_any_element(),
                                        ProjectBadge::Dropped => div()
                                            .min_w(px(18.))
                                            .px_1()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(icons::icon(
                                                Icon::Close,
                                                11.0,
                                                self.theme.text_faint,
                                            ))
                                            .into_any_element(),
                                        ProjectBadge::Count(open) => {
                                            widgets::count(self.theme, open).into_any_element()
                                        }
                                        }
                                    })
                            })
                        })
                        .on_drag(
                            DragProject::project(drag_id.clone(), project.name.clone()),
                            move |payload, position, window, cx| {
                                super::drag::project_ghost(
                                    payload, position, window, cx, drag_theme,
                                )
                            },
                        )
                        .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                            this.drop_on_project(drop_id.clone(), payload, cx)
                        }))
                        .on_drop::<DragProject>(cx.listener(|_, _: &DragProject, _, cx| {
                            cx.stop_propagation();
                        }))
                        .on_drop::<DragFolder>(cx.listener(|_, _: &DragFolder, _, cx| {
                            cx.stop_propagation();
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                this.open_sidebar_menu(
                                    SidebarTarget::Project(target_id.clone()),
                                    event.position,
                                    cx,
                                )
                            }),
                        )
                        .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                            if event.modifiers().platform {
                                this.toggle_sidebar_selection(
                                    FocusTarget::Project(view_id.clone()),
                                    cx,
                                );
                            } else {
                                this.clear_sidebar_selection(cx);
                                this.sidebar_activate(
                                    SidebarTarget::Project(view_id.clone()),
                                    cx,
                                );
                            }
                        }));
                    if !self.focus_targets.is_empty()
                        && !self
                            .focus_targets
                            .iter()
                            .any(|target| target.allows_project(db, &id))
                    {
                        row = row.opacity(0.38);
                    }
                    let anchor_boxes = self.sidebar_row_boxes.clone();
                    let anchor_key = format!("project:{id}");
                    rows.push(
                        div()
                            .on_children_prepainted(move |bounds, _, _| {
                                if let Some(bounds) = bounds.first() {
                                    anchor_boxes.borrow_mut().insert(
                                        anchor_key.clone(),
                                        [
                                            f32::from(bounds.origin.x),
                                            f32::from(bounds.origin.y),
                                            f32::from(bounds.size.width),
                                            f32::from(bounds.size.height),
                                        ],
                                    );
                                }
                            })
                            .child(row)
                            .into_any_element(),
                    );
                }
            }
        }
        if rows.is_empty() {
            // Every child was hidden from Settings: an "Empty" line would
            // claim the folder lost its work. Only a really empty tree says so.
            if !db.container_children(parent.clone()).is_empty() {
                return rows;
            }
            let key = match &parent {
                Some(folder) => format!("empty:{folder}"),
                None => "empty:root".to_string(),
            };
            rows.push(
                div()
                    .pl(px(50.0 + depth as f32 * 14.0))
                    .py_2()
                    .text_xs()
                    .text_color(self.theme.text_faint)
                    .child(self.measured(
                        key,
                        if depth == 0 {
                            "No projects yet"
                        } else {
                            "Empty"
                        },
                    ))
                    .into_any_element(),
            );
        }
        rows
    }

    fn tag_rows_filtered(
        &self,
        parent: Option<TagId>,
        geotag_filter: Option<bool>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let db = self.store.db();
        let sidebar = &db.settings.sidebar;
        let tags = db
            .tags_sorted_by_usage(parent.as_ref())
            .into_iter()
            .filter(|id| {
                geotag_filter.map_or(true, |want| {
                    db.tags
                        .get(id)
                        .is_some_and(|tag| tag.location.is_some() == want)
                })
            })
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        for tag in tags {
            let Some(entity) = db.tags.get(&tag) else {
                continue;
            };
            let key = format!("tag:{tag}");
            if sidebar.is_hidden(&key) {
                continue;
            }
            let open = self.is_open(&key, true);
            let children = !db.tag_children(&tag).is_empty();
            let usage = self.focus_tag_usage(&tag);
            let show_count = !sidebar.is_count_hidden(&key);
            let name = entity.name.clone();
            let located = entity.location.is_some();
            let status = entity.status;
            let view_id = tag.clone();
            let target_id = tag.clone();
            let drop_id = tag.clone();
            let active = self.view == View::Tag(tag.clone());
            let toggle_key = key.clone();
            let depth_pad = if parent.is_some() { 26.0 } else { 18.0 };
            let cursor = self.sidebar_cursor_is(&SidebarTarget::Tag(target_id.clone()));
            let mut row = div()
                .id(super::sid(format!("sidebar-tag:{tag}")))
                .flex()
                .items_center()
                .gap_1()
                .h(px(26.))
                .mx_2()
                .pl(px(depth_pad))
                .pr_2()
                .rounded_md()
                .bg(if active {
                    self.theme.selection
                } else {
                    self.theme.clear
                })
                .when(cursor, |this| this.bg(self.theme.hover))
                .text_color(if active {
                    self.theme.accent
                } else {
                    self.theme.text_dim
                })
                .cursor_pointer()
                .hover(|style| style.bg(self.theme.hover))
                .child(if children {
                    div()
                        .id(super::sid(format!("tag-toggle:{tag}")))
                        .w(px(12.))
                        .text_color(self.theme.text_faint)
                        .flow_tip(
                            self,
                            cx,
                            self.theme,
                            if open { "Collapse tag" } else { "Expand tag" },
                        )
                        .child(icons::icon(
                            if open {
                                Icon::ChevronDown
                            } else {
                                Icon::ChevronRight
                            },
                            12.0,
                            self.theme.text_faint,
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_expanded(toggle_key.clone(), true, cx);
                        }))
                        .into_any_element()
                } else {
                    div().w(px(12.)).into_any_element()
                })
                .child(
                    div()
                        .w(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icons::icon(
                            if located { Icon::Nearby } else { Icon::Tag },
                            13.0,
                            if located {
                                self.theme.success
                            } else {
                                self.theme.text_faint
                            },
                        )),
                )
                .child(self.row_name(name))
                .when(
                    status != flowlist_core::model::TagStatus::Available,
                    |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(self.theme.warning)
                                .child(status.label()),
                        )
                    },
                )
                .when(show_count && usage > 0, |this| {
                    this.child(self.count_box(
                        format!("tag:{tag}"),
                        usage,
                        widgets::count(self.theme, usage),
                    ))
                })
                .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                    this.drop_on_tag(drop_id.clone(), payload, cx)
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                        this.open_sidebar_menu(
                            SidebarTarget::Tag(target_id.clone()),
                            event.position,
                            cx,
                        )
                    }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.sidebar_activate(SidebarTarget::Tag(view_id.clone()), cx)
                }));
            if active {
                row = row.bg(self.theme.selection);
            }
            if !self.focus_targets.is_empty() && usage == 0 {
                row = row.opacity(0.38);
            }
            rows.push(self.row_box(key, row).into_any_element());
            if open && children {
                rows.extend(self.tag_rows_filtered(Some(tag), None, cx));
            }
        }
        rows
    }
}

/// Inbox, Today and Trash are never hidden, not even with the section toggle.
fn anchors_rail(perspective: BuiltinPerspective) -> bool {
    matches!(
        perspective,
        BuiltinPerspective::Inbox | BuiltinPerspective::Today | BuiltinPerspective::Trash
    )
}

fn tag_group_label(label: &'static str, theme: super::theme::Theme) -> gpui::Div {
    div()
        .pl(px(26.))
        .pr_3()
        .pt_2()
        .pb_1()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.text_faint)
        .child(label)
}

/// Saved views in rail order: pinned first, then creation order, then name.
fn ordered_perspectives(db: &Db) -> Vec<Perspective> {
    let mut views: Vec<Perspective> = db.perspectives.values().cloned().collect();
    views.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then(a.position.cmp(&b.position))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    views
}

struct PerspectiveRow {
    id: flowlist_core::ids::PerspectiveId,
    name: String,
    pinned: bool,
    icon: Option<IconSpec>,
}

trait ReviewBadge {
    fn flagged_for_review(&self) -> bool;
}

impl ReviewBadge for flowlist_core::model::Project {
    fn flagged_for_review(&self) -> bool {
        self.is_due_for_review(flowlist_core::time::local_date(flowlist_core::time::now()))
    }
}

pub(super) fn priority_color(theme: super::theme::Theme, priority: Priority) -> gpui::Hsla {
    super::theme::priority_color(theme, priority)
}

#[cfg(test)]
mod tests {
    use super::{ProjectBadge, anchors_rail, project_badge};
    use flowlist_core::model::{ProjectKind, ProjectStatus};
    use flowlist_core::model::BuiltinPerspective;

    #[test]
    fn the_three_anchors_never_leave_the_rail() {
        assert!(anchors_rail(BuiltinPerspective::Inbox));
        assert!(anchors_rail(BuiltinPerspective::Today));
        assert!(anchors_rail(BuiltinPerspective::Trash));
        assert!(!anchors_rail(BuiltinPerspective::Flagged));
        assert!(!anchors_rail(BuiltinPerspective::Archive));
    }

    #[test]
    fn a_new_project_keeps_its_zero() {
        assert_eq!(
            project_badge(0, 0, ProjectKind::Parallel, ProjectStatus::Active),
            ProjectBadge::Count(0)
        );
    }

    #[test]
    fn finished_work_becomes_a_tick() {
        assert_eq!(
            project_badge(0, 1, ProjectKind::Parallel, ProjectStatus::Active),
            ProjectBadge::Stalled
        );
        assert_eq!(
            project_badge(0, 12, ProjectKind::Sequential, ProjectStatus::Active),
            ProjectBadge::Stalled
        );
        assert_eq!(
            project_badge(0, 12, ProjectKind::SingleActions, ProjectStatus::Active),
            ProjectBadge::Count(0)
        );
    }

    #[test]
    fn any_open_task_keeps_the_count() {
        assert_eq!(
            project_badge(1, 2, ProjectKind::Parallel, ProjectStatus::Active),
            ProjectBadge::Count(1)
        );
        assert_eq!(
            project_badge(2, 3, ProjectKind::Parallel, ProjectStatus::Active),
            ProjectBadge::Count(2)
        );
    }
}
