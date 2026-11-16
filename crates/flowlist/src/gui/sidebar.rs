//! The left rail: built-in perspectives, saved perspectives, the project tree
//! and the tag tree. Projects and tags are drop targets for task drags.

use gpui::{AnyElement, Context, IntoElement, MouseButton, Window, div, prelude::*, px};

use flowlist_core::ids::{ContainerId, FolderId, PerspectiveId, ProjectId, TagId};
use flowlist_core::model::{BuiltinPerspective, Priority, ProjectStatus};

use super::drag::DragTask;
use super::icons::{self, Icon};
use super::modals;
use super::widgets::TipExt;
use super::{FlowlistApp, FocusArea, View, widgets};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SidebarTarget {
    Builtin(BuiltinPerspective),
    Custom(PerspectiveId),
    Folder(FolderId),
    Project(ProjectId),
    Tag(TagId),
    Priority(Priority),
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
        match &self.target {
            SidebarTarget::Builtin(perspective) => format!("builtin:{}", perspective.slug()),
            SidebarTarget::Custom(id) => format!("custom:{id}"),
            SidebarTarget::Folder(id) => format!("folder:{id}"),
            SidebarTarget::Project(id) => format!("project:{id}"),
            SidebarTarget::Tag(id) => format!("tag:{id}"),
            SidebarTarget::Priority(priority) => format!("priority:{}", priority.slug()),
        }
    }
}

fn glyph(perspective: BuiltinPerspective) -> Icon {
    match perspective {
        BuiltinPerspective::Inbox => Icon::Inbox,
        BuiltinPerspective::Projects => Icon::Projects,
        BuiltinPerspective::Forecast => Icon::Forecast,
        BuiltinPerspective::Flagged => Icon::Flagged,
        BuiltinPerspective::Review => Icon::Review,
        BuiltinPerspective::Tags => Icon::Tags,
        BuiltinPerspective::Completed => Icon::Completed,
        BuiltinPerspective::Changed => Icon::Clock,
        BuiltinPerspective::Attachments => Icon::Paperclip,
        BuiltinPerspective::Nearby => Icon::Nearby,
        BuiltinPerspective::Trash => Icon::Trash,
    }
}

impl FlowlistApp {
    /// True while the pointer rests on this sidebar row. Hover-only actions
    /// render only then, so an idle row gives its whole width to the name.
    fn row_hovered(&self, target: &SidebarTarget) -> bool {
        self.sidebar_hover.as_ref() == Some(target)
    }

    fn set_row_hover(&mut self, target: SidebarTarget, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            if self.sidebar_hover.as_ref() != Some(&target) {
                self.sidebar_hover = Some(target);
                cx.notify();
            }
        } else if self.sidebar_hover.as_ref() == Some(&target) {
            // Only clear when this row still owns the hover, so entering a new
            // row before leaving the old one cannot wipe the new state.
            self.sidebar_hover = None;
            cx.notify();
        }
    }

    /// The name slot. At rest the row width belongs to the name, so long names
    /// end in an ellipsis. On hover the row makes room for the pencil and the
    /// ×, and the name fades out at that edge instead of adding dots.
    ///
    /// The slot sits a hair above the flex center: the icon and the count box
    /// in these rows center on their own drawing, while a line of text carries
    /// its descender space below the baseline. Without the nudge the name
    /// reads as sitting low next to them.
    fn row_name(&self, name: impl Into<gpui::SharedString>, hovered: bool) -> impl IntoElement {
        let name = name.into();
        if hovered {
            div()
                .flex_1()
                .min_w_0()
                .relative()
                .top(px(-2.))
                .overflow_hidden()
                .child(div().whitespace_nowrap().child(name))
                .child(div().absolute().top_0().right_0().h_full().w(px(22.)).bg(
                    gpui::linear_gradient(
                        90.,
                        gpui::linear_color_stop(gpui::transparent_black(), 0.),
                        gpui::linear_color_stop(self.theme.hover, 1.),
                    ),
                ))
        } else {
            div()
                .flex_1()
                .min_w_0()
                .relative()
                .top(px(-2.))
                .line_clamp(1)
                .text_ellipsis()
                .child(name)
        }
    }

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

    /// The pencil on a saved perspective opens the full editor rather than a
    /// bare rename sheet.
    fn edit_perspective_button(
        &self,
        key: String,
        id: flowlist_core::ids::PerspectiveId,
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
            .flow_tip(self, cx, theme, "Edit perspective")
            .child(icons::icon(Icon::Pencil, 12.0, theme.text_faint))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                if let Some(modal) = modals::Modal::perspective(this, &id) {
                    this.open_modal(modal, cx);
                }
            }))
    }

    /// Tags can carry coordinates for the Nearby perspective.
    fn tag_location_button(
        &self,
        key: String,
        tag: flowlist_core::ids::TagId,
        located: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        div()
            .id(super::sid(key))
            .px_1()
            .rounded_sm()
            .text_color(if located {
                theme.success
            } else {
                theme.text_faint
            })
            .cursor_pointer()
            .hover(|style| style.text_color(theme.text).bg(theme.hover))
            .flow_tip(
                self,
                cx,
                theme,
                if located {
                    "Change location"
                } else {
                    "Set location"
                },
            )
            .child(icons::icon(
                Icon::Crosshair,
                12.0,
                if located {
                    theme.success
                } else {
                    theme.text_faint
                },
            ))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                let modal = modals::Modal::tag_location(this, tag.clone());
                this.open_modal(modal, cx);
            }))
    }

    fn delete_entity_button(
        &self,
        key: String,
        command: flowlist_core::command::Command,
        view: View,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        div()
            .id(super::sid(key))
            .px_1()
            .rounded_sm()
            .text_color(theme.text_faint)
            .cursor_pointer()
            .hover(|style| style.text_color(theme.danger).bg(theme.hover))
            .flow_tip(self, cx, theme, "Delete")
            .child(icons::icon(Icon::Close, 12.0, theme.danger))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                if let flowlist_core::command::Command::DeleteProject { id } = &command {
                    // A project takes its whole outline to Trash, so the
                    // sidebar asks before removing it.
                    let modal = modals::Modal::delete_project(this, id.clone());
                    this.open_modal(modal, cx);
                } else {
                    this.execute(command.clone());
                    if this.view == view {
                        this.view = View::Builtin(BuiltinPerspective::Inbox);
                    }
                    this.save_session();
                    cx.notify();
                }
            }))
    }

    pub(super) fn sidebar_targets(&self) -> Vec<SidebarTarget> {
        let db = self.store.db();
        let mut out = Vec::new();
        for perspective in BuiltinPerspective::ALL {
            out.push(SidebarTarget::Builtin(perspective));
        }
        for priority in Priority::ALL {
            out.push(SidebarTarget::Priority(priority));
        }
        for perspective in db.perspectives.values() {
            out.push(SidebarTarget::Custom(perspective.id.clone()));
        }
        self.container_targets(None, &mut out);
        self.tag_targets(None, &mut out);
        out
    }

    fn container_targets(&self, parent: Option<FolderId>, out: &mut Vec<SidebarTarget>) {
        let db = self.store.db();
        for container in db.container_children(parent) {
            match container {
                ContainerId::Folder(id) => {
                    let open = self.is_open(&format!("folder:{id}"), true);
                    let child_parent = id.clone();
                    out.push(SidebarTarget::Folder(id.clone()));
                    if open {
                        self.container_targets(Some(child_parent), out);
                    }
                }
                ContainerId::Project(id) => {
                    if db
                        .projects
                        .get(&id)
                        .is_some_and(|project| project.status.is_open())
                    {
                        out.push(SidebarTarget::Project(id.clone()));
                    }
                }
            }
        }
    }

    fn tag_targets(&self, parent: Option<TagId>, out: &mut Vec<SidebarTarget>) {
        let db = self.store.db();
        let tags = match &parent {
            Some(tag) => db.tag_children(tag).to_vec(),
            None => db.tag_roots().to_vec(),
        };
        for tag in tags {
            let open = self.is_open(&format!("tag:{tag}"), true);
            let child_parent = tag.clone();
            out.push(SidebarTarget::Tag(tag));
            if open && !db.tag_children(&child_parent).is_empty() {
                self.tag_targets(Some(child_parent), out);
            }
        }
    }

    pub(super) fn target_for_view(&self, view: &View) -> Option<SidebarTarget> {
        match view {
            View::Builtin(perspective) => Some(SidebarTarget::Builtin(*perspective)),
            View::Project(id) => Some(SidebarTarget::Project(id.clone())),
            View::Folder(id) => Some(SidebarTarget::Folder(id.clone())),
            View::Tag(id) => Some(SidebarTarget::Tag(id.clone())),
            View::Custom(id) => Some(SidebarTarget::Custom(id.clone())),
            View::Priority(priority) => Some(SidebarTarget::Priority(*priority)),
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
        let open = self.is_open(&key, true);
        if expand {
            if !open && has_children {
                self.toggle_open(key);
                cx.notify();
                return true;
            }
            return false;
        }
        if open && has_children {
            self.toggle_open(key);
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
        div()
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
                let delete_id = id.clone();
                let project_folder = id.clone();
                let child_folder = id.clone();
                let name = db
                    .folders
                    .get(&id)
                    .map(|folder| folder.name.clone())
                    .unwrap_or_else(|| "folder".to_string());
                items.push(self.sidebar_menu_item(
                    "menu-open-view".to_string(),
                    format!("Open “{name}”"),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
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
                items.push(self.sidebar_menu_item(
                    "menu-delete-folder".to_string(),
                    "Delete folder",
                    true,
                    false,
                    cx,
                    move |this, cx| {
                        this.delete_sidebar_entity(
                            flowlist_core::command::Command::DeleteFolder {
                                id: delete_id.clone(),
                            },
                            View::Folder(delete_id.clone()),
                            cx,
                        )
                    },
                ));
            }
            SidebarTarget::Project(id) => {
                let open_target = SidebarTarget::Project(id.clone());
                let edit_id = id.clone();
                let delete_id = id.clone();
                let name = db
                    .projects
                    .get(&id)
                    .map(|project| project.name.clone())
                    .unwrap_or_else(|| "project".to_string());
                let status = db.projects.get(&id).map(|project| project.status);
                items.push(self.sidebar_menu_item(
                    "menu-open-project".to_string(),
                    format!("Open “{name}”"),
                    false,
                    false,
                    cx,
                    move |this, cx| this.sidebar_activate(open_target.clone(), cx),
                ));
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
                items.push(self.sidebar_menu_item(
                    "menu-delete-project".to_string(),
                    "Delete project",
                    true,
                    false,
                    cx,
                    move |this, cx| {
                        this.delete_sidebar_entity(
                            flowlist_core::command::Command::DeleteProject {
                                id: delete_id.clone(),
                            },
                            View::Project(delete_id.clone()),
                            cx,
                        )
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
                        this.delete_sidebar_entity(
                            flowlist_core::command::Command::DeleteTag {
                                id: delete_id.clone(),
                            },
                            View::Tag(delete_id.clone()),
                            cx,
                        )
                    },
                ));
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
            );

        rail = rail.child(widgets::section_label(theme, "PERSPECTIVES"));
        for perspective in BuiltinPerspective::ALL {
            rail = rail.child(self.perspective_row(perspective, cx));
        }

        rail = rail.child(widgets::section_label(theme, "PRIORITIES"));
        for priority in Priority::ALL {
            rail = rail.child(self.priority_row(priority, cx));
        }

        let mut lower = div()
            .id("sidebar-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.sidebar_scroll);

        let custom: Vec<PerspectiveRow> = self
            .store
            .db()
            .perspectives
            .values()
            .map(|perspective| PerspectiveRow {
                id: perspective.id.clone(),
                name: perspective.name.clone(),
            })
            .collect();
        if !custom.is_empty() {
            lower = lower.child(widgets::section_label(theme, "SAVED"));
            for row in custom {
                lower = lower.child(self.custom_perspective_row(row, cx));
            }
        }

        lower = lower.child(widgets::section_label(theme, "LIBRARY"));
        lower = lower.children(self.container_rows(None, cx));

        lower = lower.child(widgets::section_label(theme, "TAGS"));
        lower = lower.children(self.tag_rows(None, cx));
        if self.store.db().tags.is_empty() {
            lower = lower.child(
                div()
                    .px_3()
                    .pb_2()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No tags yet — add one with #tag in quick entry."),
            );
        }

        rail = rail.child(lower);
        rail.child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .p_2()
                .border_t_1()
                .border_color(theme.border_soft)
                .child(
                    div()
                        .id("new-project")
                        .flex_1()
                        .child(widgets::button_with_icon(
                            theme,
                            Icon::Plus,
                            "Project",
                            false,
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_project(this), cx)
                        })),
                )
                .child(
                    div()
                        .id("new-folder")
                        .flex_1()
                        .child(widgets::button_with_icon(
                            theme,
                            Icon::Plus,
                            "Folder",
                            false,
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_folder(this), cx)
                        })),
                )
                .child(
                    div()
                        .id("new-tag")
                        .flex_1()
                        .child(widgets::button_with_icon(theme, Icon::Plus, "Tag", false))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_tag(this), cx)
                        })),
                )
                .child(
                    div()
                        .id("new-perspective")
                        .flex_1()
                        .child(widgets::button_with_icon(
                            theme,
                            Icon::Perspective,
                            "View",
                            false,
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_perspective(this), cx)
                        })),
                ),
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
        let icon_color = if active { theme.accent } else { theme.text_dim };
        let cursor = self.sidebar_cursor_is(&SidebarTarget::Builtin(perspective));
        div()
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
            .when(count > 0, |this| this.child(widgets::count(theme, count)))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    this.open_sidebar_menu(SidebarTarget::Builtin(perspective), event.position, cx)
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.sidebar_activate(SidebarTarget::Builtin(perspective), cx)
            }))
    }

    fn priority_row(&self, priority: Priority, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let active = self.view == View::Priority(priority);
        let count = self
            .store
            .db()
            .tasks
            .values()
            .filter(|task| !task.completed && !task.is_trashed() && task.priority == Some(priority))
            .count();
        let color = priority_color(theme, priority);
        let icon_color = if active { theme.accent } else { color };
        let cursor = self.sidebar_cursor_is(&SidebarTarget::Priority(priority));
        div()
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
            .when(count > 0, |this| this.child(widgets::count(theme, count)))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    this.open_sidebar_menu(SidebarTarget::Priority(priority), event.position, cx)
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.sidebar_activate(SidebarTarget::Priority(priority), cx)
            }))
    }

    fn custom_perspective_row(
        &self,
        row: PerspectiveRow,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let active = self.view == View::Custom(row.id.clone());
        let icon_color = if active { theme.accent } else { theme.text_dim };
        let id = row.id.clone();
        let target_id = row.id.clone();
        let rename_id = row.id.clone();
        let delete_id = row.id.clone();
        let delete_view = View::Custom(row.id.clone());
        let cursor = self.sidebar_cursor_is(&SidebarTarget::Custom(target_id.clone()));
        let hovered = self.row_hovered(&SidebarTarget::Custom(target_id.clone()));
        let hover_id = id.clone();
        div()
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
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                this.set_row_hover(SidebarTarget::Custom(hover_id.clone()), *hovered, cx);
            }))
            .child(
                div()
                    .w(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icons::icon(Icon::Perspective, 14.0, icon_color)),
            )
            .child(self.row_name(row.name, hovered))
            .when(hovered, |this| {
                this.child(self.edit_perspective_button(
                    format!("edit-perspective:{rename_id}"),
                    rename_id,
                    cx,
                ))
                .child(self.delete_entity_button(
                    format!("delete-perspective:{delete_id}"),
                    flowlist_core::command::Command::DeletePerspective { id: delete_id },
                    delete_view,
                    cx,
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
            }))
    }

    fn container_rows(&self, parent: Option<FolderId>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let db = self.store.db();
        let children = db.container_children(parent.clone()).to_vec();
        let mut rows = Vec::new();
        for container in children {
            match container {
                ContainerId::Folder(id) => {
                    let key = format!("folder:{id}");
                    let open = self.is_open(&key, true);
                    let name = db.folder_path(&id);
                    let count = db.folder_open_project_count(&id);
                    let view_id = id.clone();
                    let target_id = id.clone();
                    let toggle_key = key.clone();
                    let rename_id = id.clone();
                    let delete_id = id.clone();
                    let delete_view = View::Folder(id.clone());
                    let active = self.view == View::Folder(id.clone());
                    let cursor = self.sidebar_cursor_is(&SidebarTarget::Folder(target_id.clone()));
                    let hovered = self.row_hovered(&SidebarTarget::Folder(target_id.clone()));
                    let hover_id = id.clone();
                    let mut row = div()
                        .id(super::sid(format!("sidebar-folder:{id}")))
                        .flex()
                        .items_center()
                        .gap_1()
                        .h(px(26.))
                        .mx_2()
                        .pl_1()
                        .pr_2()
                        .rounded_md()
                        .text_color(self.theme.text_dim)
                        .cursor_pointer()
                        .hover(|style| style.bg(self.theme.hover))
                        .when(cursor, |this| this.bg(self.theme.hover))
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            this.set_row_hover(
                                SidebarTarget::Folder(hover_id.clone()),
                                *hovered,
                                cx,
                            );
                        }))
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
                                    this.toggle_expanded(toggle_key.clone(), cx);
                                })),
                        )
                        .child(
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
                        )
                        .child(self.row_name(name, hovered))
                        .when(hovered, |this| {
                            this.child(self.rename_button(
                                format!("rename-folder:{rename_id}"),
                                modals::EntityRef::Folder(rename_id),
                                cx,
                            ))
                            .child(self.delete_entity_button(
                                format!("delete-folder:{delete_id}"),
                                flowlist_core::command::Command::DeleteFolder { id: delete_id },
                                delete_view,
                                cx,
                            ))
                        })
                        .when(count > 0, |this| {
                            this.child(widgets::count(self.theme, count))
                        })
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
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.sidebar_activate(SidebarTarget::Folder(view_id.clone()), cx)
                        }));
                    if active {
                        row = row.bg(self.theme.selection).text_color(self.theme.accent);
                    }
                    rows.push(row.into_any_element());
                    if open {
                        rows.extend(self.container_rows(Some(id), cx));
                    }
                }
                ContainerId::Project(id) => {
                    let Some(project) = db.projects.get(&id) else {
                        continue;
                    };
                    if !project.status.is_open() {
                        continue;
                    }
                    let (done, total) = db.project_progress(&id);
                    let open = db.project_open_count(&id);
                    let view_id = id.clone();
                    let target_id = id.clone();
                    let drop_id = id.clone();
                    let rename_id = id.clone();
                    let delete_id = id.clone();
                    let active = self.view == View::Project(id.clone());
                    let cursor = self.sidebar_cursor_is(&SidebarTarget::Project(target_id.clone()));
                    let hovered = self.row_hovered(&SidebarTarget::Project(target_id.clone()));
                    let hover_id = id.clone();
                    let mut row = div()
                        .id(super::sid(format!("sidebar-project:{id}")))
                        .flex()
                        .items_center()
                        .gap_2()
                        .h(px(26.))
                        .mx_2()
                        .pl_5()
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
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            this.set_row_hover(
                                SidebarTarget::Project(hover_id.clone()),
                                *hovered,
                                cx,
                            );
                        }))
                        .child(widgets::icon_for_project(
                            project.icon,
                            14.0,
                            if active {
                                self.theme.accent
                            } else {
                                self.theme.text_faint
                            },
                        ))
                        .child(self.row_name(project.name.clone(), hovered))
                        .when(hovered, |this| {
                            this.child(self.rename_button(
                                format!("rename-project:{rename_id}"),
                                modals::EntityRef::Project(rename_id),
                                cx,
                            ))
                            .child(self.delete_entity_button(
                                format!("delete-project:{delete_id}"),
                                flowlist_core::command::Command::DeleteProject { id: delete_id },
                                View::Project(id.clone()),
                                cx,
                            ))
                        })
                        .child(
                            div()
                                .id(super::sid(format!("project-count:{id}")))
                                .flow_tip(
                                    self,
                                    cx,
                                    self.theme,
                                    format!("{open} open · {done} of {total} done"),
                                )
                                .child(widgets::count(self.theme, open)),
                        )
                        .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                            this.drop_on_project(drop_id.clone(), payload, cx)
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
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.sidebar_activate(SidebarTarget::Project(view_id.clone()), cx)
                        }));
                    if project.flagged_for_review() {
                        row = row.child(div().size(px(5.)).rounded_full().bg(self.theme.warning));
                    }
                    rows.push(row.into_any_element());
                }
            }
        }
        if rows.is_empty() {
            rows.push(
                div()
                    .px_4()
                    .py_2()
                    .text_xs()
                    .text_color(self.theme.text_faint)
                    .child("No projects yet")
                    .into_any_element(),
            );
        }
        rows
    }

    fn tag_rows(&self, parent: Option<TagId>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let db = self.store.db();
        let tags = match &parent {
            Some(tag) => db.tag_children(tag).to_vec(),
            None => db.tag_roots().to_vec(),
        };
        let mut rows = Vec::new();
        for tag in tags {
            let Some(entity) = db.tags.get(&tag) else {
                continue;
            };
            let key = format!("tag:{tag}");
            let open = self.is_open(&key, true);
            let children = !db.tag_children(&tag).is_empty();
            let usage = db.tag_usage_count(&tag, true);
            let name = entity.name.clone();
            let located = entity.location.is_some();
            let status = entity.status;
            let view_id = tag.clone();
            let target_id = tag.clone();
            let drop_id = tag.clone();
            let rename_id = tag.clone();
            let delete_id = tag.clone();
            let delete_view = View::Tag(tag.clone());
            let active = self.view == View::Tag(tag.clone());
            let toggle_key = key.clone();
            let depth_pad = if parent.is_some() { 26.0 } else { 18.0 };
            let cursor = self.sidebar_cursor_is(&SidebarTarget::Tag(target_id.clone()));
            let hovered = self.row_hovered(&SidebarTarget::Tag(target_id.clone()));
            let hover_id = tag.clone();
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
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    this.set_row_hover(SidebarTarget::Tag(hover_id.clone()), *hovered, cx);
                }))
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
                            this.toggle_expanded(toggle_key.clone(), cx);
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
                .child(self.row_name(name, hovered))
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
                // The crosshair also marks a selected tag, so it stays put on
                // the active row and only borrows width from hovered rows.
                .when(hovered || active, |this| {
                    this.child(self.tag_location_button(
                        format!("location-tag:{tag}"),
                        tag.clone(),
                        located,
                        cx,
                    ))
                })
                .when(hovered, |this| {
                    this.child(self.rename_button(
                        format!("rename-tag:{rename_id}"),
                        modals::EntityRef::Tag(rename_id),
                        cx,
                    ))
                    .child(self.delete_entity_button(
                        format!("delete-tag:{delete_id}"),
                        flowlist_core::command::Command::DeleteTag { id: delete_id },
                        delete_view,
                        cx,
                    ))
                })
                .when(usage > 0, |this| {
                    this.child(widgets::count(self.theme, usage))
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
            rows.push(row.into_any_element());
            if open && children {
                rows.extend(self.tag_rows(Some(tag), cx));
            }
        }
        rows
    }
}

struct PerspectiveRow {
    id: flowlist_core::ids::PerspectiveId,
    name: String,
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
    match priority {
        Priority::High => theme.danger,
        Priority::Medium => theme.warning,
        Priority::Low => theme.text_faint,
    }
}
