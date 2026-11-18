//! Modal sheets: new project/folder/tag/perspective, import, settings,
//! locations and the keyboard help card.

use gpui::{
    AnyElement, Bounds, Context, IntoElement, Window, div, point, prelude::*, px, rgba, size,
};

use flowlist_core::ids::{FolderId, PerspectiveId, ProjectId, TaskId};
use flowlist_core::model::{GroupBy, ProjectIcon, ProjectKind, SidebarOptions, SortBy, TaskStatus};
use flowlist_core::query;
use flowlist_core::time::now;

use super::icons::Icon;
use super::perspective::{PerspectiveForm, ViewChoice, ViewField};
use super::text::TextBuffer;
use super::theme::{Appearance, Theme, ThemeKind};
use super::widgets::TipExt;
use super::{FlowlistApp, icons, widgets};

/// What a rename sheet is editing. Keeping the typed id means a rename can
/// never accidentally target the wrong entity kind.
#[derive(Clone, Debug)]
pub enum EntityRef {
    Project(flowlist_core::ids::ProjectId),
    Folder(flowlist_core::ids::FolderId),
    Tag(flowlist_core::ids::TagId),
    Perspective(flowlist_core::ids::PerspectiveId),
}

/// What an attachment link sheet is attached to. Keeping the typed id means a
/// link can never land on the wrong task or project.
#[derive(Clone, Debug)]
pub enum AttachmentTarget {
    Task(TaskId),
    Project(ProjectId),
}

pub enum Modal {
    Rename {
        entity: EntityRef,
        name: TextBuffer,
    },
    NewProject {
        name: TextBuffer,
        notes: TextBuffer,
        /// Review cadence for this project, in days. Starts at the library
        /// default and can be changed per project.
        review_days: TextBuffer,
        kind: ProjectKind,
        icon: ProjectIcon,
        /// The icon grid stays folded away until the current icon is clicked.
        icon_open: bool,
    },
    /// Typing a project path that does not exist yet asks before creating it,
    /// so a typo cannot silently fill the sidebar with projects.
    CreateProject {
        task: TaskId,
        path: String,
    },
    /// Deleting a project trashes everything inside it. The sheet names the
    /// project and asks first; Undo still restores it in one step.
    DeleteProject {
        id: ProjectId,
        name: String,
    },
    /// Deleting a folder moves its projects up to its parent (or the library
    /// root). The sheet names the folder and asks first; Undo restores it.
    DeleteFolder {
        id: FolderId,
        name: String,
    },
    /// Adding an http(s) link to a task or project. Links share the attachment
    /// list with file paths, so the row badge and the Attachments view already
    /// include them.
    AttachLink {
        target: AttachmentTarget,
        url: TextBuffer,
    },
    NewFolder {
        name: TextBuffer,
    },
    NewTag {
        name: TextBuffer,
    },
    Perspective {
        form: PerspectiveForm,
    },
    Import {
        text: TextBuffer,
        merge: bool,
    },
    Settings {
        review_days: TextBuffer,
        completed_days: TextBuffer,
        changed_days: TextBuffer,
        radius: TextBuffer,
        trash_days: TextBuffer,
        vim_mode: bool,
        follow_system: bool,
        theme: ThemeKind,
        appearance: Appearance,
        sidebar: SidebarOptions,
    },
    Location {
        lat: TextBuffer,
        lon: TextBuffer,
    },
    TagLocation {
        tag: flowlist_core::ids::TagId,
        lat: TextBuffer,
        lon: TextBuffer,
    },
    Help,
}

/// Which sidebar preference a Settings toggle flips.
#[derive(Clone, Copy, Debug)]
enum SidebarFlag {
    ShowPerspectives,
    ShowPriorities,
    ShowLibrary,
    ShowSaved,
    ShowTags,
    CountsPerspectives,
    CountsPriorities,
    CountsLibrary,
    CountsTags,
}

impl SidebarFlag {
    fn toggle(self, sidebar: &mut SidebarOptions) {
        let value = match self {
            SidebarFlag::ShowPerspectives => &mut sidebar.show_perspectives,
            SidebarFlag::ShowPriorities => &mut sidebar.show_priorities,
            SidebarFlag::ShowLibrary => &mut sidebar.show_library,
            SidebarFlag::ShowSaved => &mut sidebar.show_saved,
            SidebarFlag::ShowTags => &mut sidebar.show_tags,
            SidebarFlag::CountsPerspectives => &mut sidebar.counts_perspectives,
            SidebarFlag::CountsPriorities => &mut sidebar.counts_priorities,
            SidebarFlag::CountsLibrary => &mut sidebar.counts_library,
            SidebarFlag::CountsTags => &mut sidebar.counts_tags,
        };
        *value = !*value;
    }
}

impl Modal {
    pub fn rename(app: &FlowlistApp, entity: EntityRef) -> Self {
        let db = app.store.db();
        let name = match &entity {
            EntityRef::Project(id) => db.projects.get(id).map(|project| project.name.clone()),
            EntityRef::Folder(id) => db.folders.get(id).map(|folder| folder.name.clone()),
            EntityRef::Tag(id) => db.tags.get(id).map(|tag| tag.name.clone()),
            EntityRef::Perspective(id) => db
                .perspectives
                .get(id)
                .map(|perspective| perspective.name.clone()),
        }
        .unwrap_or_default();
        // The name is already selected, so typing replaces it and a single
        // Backspace clears it. Previously the caret sat at the end and the
        // only way to change a name was to erase it one key at a time.
        let mut name = TextBuffer::new(name);
        name.select_all();
        Modal::Rename { entity, name }
    }

    pub fn delete_project(app: &FlowlistApp, id: ProjectId) -> Self {
        let name = app
            .store
            .db()
            .projects
            .get(&id)
            .map(|project| project.name.clone())
            .unwrap_or_default();
        Modal::DeleteProject { id, name }
    }

    pub fn delete_folder(app: &FlowlistApp, id: FolderId) -> Self {
        let name = app
            .store
            .db()
            .folders
            .get(&id)
            .map(|folder| folder.name.clone())
            .unwrap_or_default();
        Modal::DeleteFolder { id, name }
    }

    pub fn new_project(app: &FlowlistApp) -> Self {
        Modal::NewProject {
            name: TextBuffer::empty(),
            notes: TextBuffer::empty(),
            review_days: TextBuffer::new(app.store.db().settings.default_review_days.to_string()),
            kind: ProjectKind::Parallel,
            icon: ProjectIcon::default(),
            icon_open: false,
        }
    }

    pub fn attach_link(target: AttachmentTarget) -> Self {
        Modal::AttachLink {
            target,
            url: TextBuffer::empty(),
        }
    }

    pub fn new_folder(_app: &FlowlistApp) -> Self {
        Modal::NewFolder {
            name: TextBuffer::empty(),
        }
    }

    pub fn new_tag(_app: &FlowlistApp) -> Self {
        Modal::NewTag {
            name: TextBuffer::empty(),
        }
    }

    pub fn new_perspective(_app: &FlowlistApp) -> Self {
        Modal::Perspective {
            form: PerspectiveForm::new(),
        }
    }

    /// Open the editor on an existing perspective. The form keeps the id so
    /// saving updates the saved query instead of creating a second one.
    pub fn perspective(app: &FlowlistApp, id: &PerspectiveId) -> Option<Self> {
        let perspective = app.store.db().perspectives.get(id)?;
        Some(Modal::Perspective {
            form: PerspectiveForm::edit(perspective),
        })
    }

    pub fn settings(app: &FlowlistApp) -> Self {
        let settings = &app.store.db().settings;
        Modal::Settings {
            review_days: TextBuffer::new(settings.default_review_days.to_string()),
            completed_days: TextBuffer::new(settings.show_completed_days.to_string()),
            changed_days: TextBuffer::new(settings.changed_days.to_string()),
            radius: TextBuffer::new(settings.nearby_radius_km.to_string()),
            trash_days: TextBuffer::new(settings.trash_retention_days.to_string()),
            vim_mode: app.vim_mode,
            follow_system: app.follow_system,
            theme: app.theme.kind(),
            appearance: app.theme.appearance(),
            sidebar: settings.sidebar.clone(),
        }
    }

    pub fn location(app: &FlowlistApp) -> Self {
        let location = app.store.db().settings.current_location;
        Modal::Location {
            lat: TextBuffer::new(location.map(|geo| geo.lat.to_string()).unwrap_or_default()),
            lon: TextBuffer::new(location.map(|geo| geo.lon.to_string()).unwrap_or_default()),
        }
    }

    pub fn tag_location(app: &FlowlistApp, tag: flowlist_core::ids::TagId) -> Self {
        let location = app.store.db().tags.get(&tag).and_then(|tag| tag.location);
        Modal::TagLocation {
            tag,
            lat: TextBuffer::new(location.map(|geo| geo.lat.to_string()).unwrap_or_default()),
            lon: TextBuffer::new(location.map(|geo| geo.lon.to_string()).unwrap_or_default()),
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Modal::Rename { .. } => "Rename",
            Modal::NewProject { .. } => "New project",
            Modal::CreateProject { .. } => "Create project",
            Modal::DeleteProject { .. } => "Delete project",
            Modal::DeleteFolder { .. } => "Delete folder",
            Modal::AttachLink { .. } => "Attach link",
            Modal::NewFolder { .. } => "New folder",
            Modal::NewTag { .. } => "New tag",
            Modal::Perspective { .. } => "Perspective",
            Modal::Import { .. } => "Import library",
            Modal::Settings { .. } => "Settings",
            Modal::Location { .. } => "Set location",
            Modal::TagLocation { .. } => "Tag location",
            Modal::Help => "Keyboard & mouse",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Modal::Rename { .. } => "Names are shared by the sidebar, CLI and MCP server.",
            Modal::NewProject { .. } => {
                "Projects hold an outline of tasks; folders keep projects together."
            }
            Modal::CreateProject { .. } => {
                "This path is not in the library yet. Create it and file the task there?"
            }
            Modal::DeleteProject { .. } => {
                "Its tasks move to Trash. Undo restores the project and its tasks."
            }
            Modal::DeleteFolder { .. } => {
                "Its projects and their tasks move to Trash with it. Undo restores everything."
            }
            Modal::AttachLink { .. } => {
                "Paste an http(s) address; it opens in your browser. Undo removes it."
            }
            Modal::NewFolder { .. } => "Folders group projects. They can be nested.",
            Modal::NewTag { .. } => "Tags are contexts: places, tools or people.",
            Modal::Perspective { .. } => {
                "Saved filters with their own conditions, grouping and sort order."
            }
            Modal::Import { .. } => {
                "Paste Flowlist JSON or TaskPaper text. Merge keeps existing items."
            }
            Modal::Settings { .. } => "Library-wide defaults shared by the GUI, CLI and MCP.",
            Modal::Location { .. } => {
                "Nearby uses this point and the radius below to rank tagged tasks."
            }
            Modal::TagLocation { .. } => "Give this context a latitude and longitude.",
            Modal::Help => "",
        }
    }

    /// Project creation sheets carry their own Cancel button in the footer,
    /// so they do not need a second close control in the top corner.
    pub fn shows_close(&self) -> bool {
        !matches!(self, Modal::NewProject { .. } | Modal::CreateProject { .. })
    }

    pub fn confirm_label(&self) -> &'static str {
        match self {
            Modal::CreateProject { .. } => "Create",
            Modal::DeleteProject { .. } => "Delete",
            Modal::DeleteFolder { .. } => "Delete",
            Modal::AttachLink { .. } => "Attach",
            _ => "Save",
        }
    }

    pub fn field_count(&self) -> usize {
        match self {
            Modal::Rename { .. } => 1,
            Modal::NewProject { .. } => 3,
            Modal::CreateProject { .. } => 0,
            Modal::DeleteProject { .. } => 0,
            Modal::DeleteFolder { .. } => 0,
            Modal::AttachLink { .. } => 1,
            Modal::NewFolder { .. } | Modal::NewTag { .. } => 1,
            Modal::Perspective { .. } => 1,
            Modal::Import { .. } => 1,
            Modal::Settings { .. } => 5,
            Modal::Location { .. } | Modal::TagLocation { .. } => 2,
            Modal::Help => 0,
        }
    }

    pub fn field(&self, index: usize) -> Option<&TextBuffer> {
        match self {
            Modal::Rename { name, .. } => (index == 0).then_some(name),
            Modal::NewProject {
                name,
                notes,
                review_days,
                ..
            } => match index {
                0 => Some(name),
                1 => Some(notes),
                2 => Some(review_days),
                _ => None,
            },
            Modal::CreateProject { .. } => None,
            Modal::DeleteProject { .. } => None,
            Modal::DeleteFolder { .. } => None,
            Modal::AttachLink { url, .. } => (index == 0).then_some(url),
            Modal::NewFolder { name } | Modal::NewTag { name } => (index == 0).then_some(name),
            Modal::Perspective { form } => (index == 0).then_some(&form.name),
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                changed_days,
                radius,
                trash_days,
                ..
            } => match index {
                0 => Some(review_days),
                1 => Some(completed_days),
                2 => Some(changed_days),
                3 => Some(radius),
                4 => Some(trash_days),
                _ => None,
            },
            Modal::Location { lat, lon } | Modal::TagLocation { lat, lon, .. } => match index {
                0 => Some(lat),
                1 => Some(lon),
                _ => None,
            },
            Modal::Help => None,
        }
    }

    pub fn field_mut(&mut self, index: usize) -> Option<&mut TextBuffer> {
        match self {
            Modal::Rename { name, .. } => (index == 0).then_some(name),
            Modal::NewProject {
                name,
                notes,
                review_days,
                ..
            } => match index {
                0 => Some(name),
                1 => Some(notes),
                2 => Some(review_days),
                _ => None,
            },
            Modal::CreateProject { .. } => None,
            Modal::DeleteProject { .. } => None,
            Modal::DeleteFolder { .. } => None,
            Modal::AttachLink { url, .. } => (index == 0).then_some(url),
            Modal::NewFolder { name } | Modal::NewTag { name } => (index == 0).then_some(name),
            Modal::Perspective { form } => (index == 0).then_some(&mut form.name),
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                changed_days,
                radius,
                trash_days,
                ..
            } => match index {
                0 => Some(review_days),
                1 => Some(completed_days),
                2 => Some(changed_days),
                3 => Some(radius),
                4 => Some(trash_days),
                _ => None,
            },
            Modal::Location { lat, lon } | Modal::TagLocation { lat, lon, .. } => match index {
                0 => Some(lat),
                1 => Some(lon),
                _ => None,
            },
            Modal::Help => None,
        }
    }

    pub fn field_label(&self, index: usize) -> &'static str {
        match self {
            Modal::Rename { .. } => "Name",
            Modal::NewProject { .. } => match index {
                0 => "Name",
                1 => "Notes",
                2 => "Review every (days)",
                _ => "",
            },
            Modal::CreateProject { .. } => "",
            Modal::DeleteProject { .. } => "",
            Modal::DeleteFolder { .. } => "",
            Modal::AttachLink { .. } => "Address",
            Modal::NewFolder { .. } | Modal::NewTag { .. } => "Name",
            Modal::Perspective { .. } => "Name",
            Modal::Import { .. } => "Library text",
            Modal::Settings { .. } => match index {
                0 => "Default review days",
                1 => "Completed history (days)",
                2 => "Changed history (days)",
                3 => "Nearby radius (km)",
                4 => "Trash retention (days, 0 keeps forever)",
                _ => "",
            },
            Modal::Location { .. } => match index {
                0 => "Latitude",
                1 => "Longitude",
                _ => "",
            },
            Modal::TagLocation { .. } => match index {
                0 => "Latitude",
                1 => "Longitude",
                _ => "",
            },
            Modal::Help => "",
        }
    }

    fn multiline(&self, index: usize) -> bool {
        matches!(self, Modal::Import { .. }) && index == 0
    }

    fn width(&self) -> f32 {
        match self {
            Modal::Import { .. } | Modal::Help => 620.0,
            Modal::Perspective { .. } => 680.0,
            Modal::Settings { .. } => 580.0,
            _ => 460.0,
        }
    }
}

impl FlowlistApp {
    pub(super) fn render_modal(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(modal) = self.modal.as_ref() else {
            return div().into_any_element();
        };
        let title = modal.title().to_string();
        let subtitle = modal.subtitle().to_string();
        let width = modal.width();
        let field_count = modal.field_count();
        let confirm_label = modal.confirm_label();
        let is_help = matches!(modal, Modal::Help);
        let is_perspective = matches!(modal, Modal::Perspective { .. });
        let is_import = matches!(modal, Modal::Import { .. });
        let is_settings = matches!(modal, Modal::Settings { .. });
        let is_new_project = matches!(modal, Modal::NewProject { .. });
        // Project creation already has Cancel and Create at the bottom, so a
        // second close control in the corner is redundant.
        let shows_close = modal.shows_close();
        let is_tag_location = matches!(modal, Modal::TagLocation { .. });
        let mut header = div().flex().items_start().justify_between().child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title),
                )
                .when(!subtitle.is_empty(), |this| {
                    this.child(
                        div()
                            .max_w(px(480.))
                            .text_sm()
                            .text_color(theme.text_faint)
                            .child(subtitle),
                    )
                }),
        );
        if shows_close {
            header = header.child(
                div()
                    .id("modal-close")
                    .cursor_pointer()
                    .text_color(theme.text_faint)
                    .hover(|style| style.text_color(theme.text))
                    .flow_tip(self, cx, theme, "Close (Esc)")
                    .child(icons::icon(icons::Icon::Close, 14.0, theme.text_faint))
                    .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
            );
        }

        // The settings sheet is taller than the window on short displays. A
        // scroll region keeps every control, especially the full theme grid,
        // reachable instead of clipped by the card's rounded corners.
        let mut body = div()
            .id("modal-body")
            .flex()
            .flex_col()
            .gap_3()
            .flex_1()
            .min_h_0()
            .pr_1()
            .overflow_y_scroll()
            .track_scroll(&self.modal_scroll);

        if is_help {
            body = body.child(help_body(theme));
        } else {
            // Theme and appearance are the first thing people open Settings
            // for; keep them above the numeric fields so they are visible
            // without scrolling on a normal window.
            if is_settings {
                body = body.child(self.render_settings_controls(cx));
                // The rail controls are visual, so they sit with Appearance
                // instead of being buried under the numeric defaults.
                body = body.child(self.render_sidebar_settings(cx));
            }
            if let Modal::CreateProject { path, .. } = modal {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.field)
                        .text_sm()
                        .text_color(theme.text)
                        .child(path.clone()),
                );
            }
            if let Modal::DeleteProject { name, .. } = modal {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.field)
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(name.clone()),
                );
            }
            if let Modal::DeleteFolder { id, name } = modal {
                // Name the folder and say how much goes with it, so the
                // cascade is never a surprise.
                let projects = self.store.db().project_count(id);
                let detail = match projects {
                    0 => "It has no projects inside.".to_string(),
                    1 => "Its 1 project, and every task in it, moves to Trash too.".to_string(),
                    count => {
                        format!("Its {count} projects, and every task in them, move to Trash too.")
                    }
                };
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.danger)
                        .bg(theme.field)
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.text)
                                .child(name.clone()),
                        )
                        .child(div().text_xs().text_color(theme.text_faint).child(detail)),
                );
            }
            for index in 0..field_count {
                body = body.child(self.render_modal_field(index, cx));
            }
            if is_new_project {
                body = body.child(self.render_new_project_controls(cx));
            }
            if is_perspective {
                body = body.child(self.render_perspective_controls(cx));
            }
            if is_import {
                body = body.child(self.render_import_controls(cx));
            }
            if is_tag_location {
                body = body.child(self.render_tag_location_controls(cx));
            }
        }

        let mut card = div()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(width))
            .max_h(px(640.))
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_2xl()
            .overflow_hidden()
            .child(header)
            .child(body);

        if !is_help {
            card = card.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pt_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child(if field_count > 0 {
                                "Esc close · ⇥ next field"
                            } else {
                                "Esc close"
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("modal-cancel")
                                    .child(widgets::button(theme, "Cancel", false))
                                    .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                            )
                            .child(
                                div()
                                    .id("modal-save")
                                    .child(widgets::button(theme, confirm_label, true))
                                    .on_click(cx.listener(|this, _, _, cx| this.commit_modal(cx))),
                            ),
                    ),
            );
        }

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            // The dimmed backdrop must swallow clicks meant for the window
            // underneath, not select a row or chip through the sheet.
            .occlude()
            .bg(rgba(0x00000055))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .max_h(px(720.))
                    .child(card)
                    .into_any_element(),
            )
            .into_any_element()
    }

    fn render_modal_field(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(modal) = self.modal.as_ref() else {
            return div().into_any_element();
        };
        let Some(buffer) = modal.field(index) else {
            return div().into_any_element();
        };
        let label = modal.field_label(index).to_string();
        let focused = self.modal_focus == index;
        let multiline = modal.multiline(index);
        let field = if multiline {
            widgets::text_area(
                theme,
                buffer,
                "Paste JSON or TaskPaper here",
                focused,
                180.0,
            )
            .into_any_element()
        } else {
            widgets::text_field_with_cursor(
                theme,
                buffer,
                "",
                focused,
                cx.listener(move |this, hit: &widgets::TextHit, _, cx| {
                    this.set_modal_cursor(index, hit.index, hit.extend, cx);
                }),
            )
            .into_any_element()
        };
        let anchor = self.modal_field_anchor.clone();
        div()
            .id(super::sid(format!("modal-field:{index}")))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child(label))
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            anchor.set(Some(*bounds));
                        }
                    })
                    .child(field),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.modal_focus = index;
                this.focus = super::FocusArea::Modal;
                cx.notify();
            }))
            .into_any_element()
    }

    /// The perspective editor is a small settings sheet rather than a row of
    /// cycling chips: every condition has its own labelled row and opens a menu
    /// of every value, so choosing one is a jump instead of a lap.
    fn render_perspective_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div();
        };
        let form = form.clone();
        let db = self.store.db();
        let project_label = form
            .project
            .as_ref()
            .map(|id| db.project_path(id))
            .unwrap_or_else(|| "Any".to_string());
        let tag_label = form
            .tag
            .as_ref()
            .and_then(|id| db.tags.get(id))
            .map(|tag| tag.name.clone())
            .unwrap_or_else(|| "Any".to_string());
        let extra = form.extra_condition_count();
        // A live count turns the sheet into a preview: every change shows how
        // many tasks the saved view would list.
        let preview = query::run(
            db,
            &form.filter(),
            form.sort,
            form.group,
            form.show_completed,
            now(),
        );
        let matches = preview.iter().map(|group| group.rows.len()).sum::<usize>();

        let toggle = |label: &'static str, active: bool, id: &'static str| {
            div()
                .id(id)
                .child(widgets::toggle_chip(theme, label, active))
        };
        let toggles = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(
                toggle("Inbox only", form.inbox_only, "modal-inbox").on_click(cx.listener(
                    |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.toggle_inbox();
                        }
                        cx.notify();
                    },
                )),
            )
            .child(
                toggle("Flagged only", form.flagged_only, "modal-flagged").on_click(cx.listener(
                    |this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.flagged_only = !form.flagged_only;
                        }
                        cx.notify();
                    },
                )),
            )
            .child(
                toggle("Completed", form.show_completed, "modal-completed")
                    .flow_tip(self, cx, theme, "Include completed tasks")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.show_completed = !form.show_completed;
                        }
                        cx.notify();
                    })),
            );

        let project_tag_row = div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Project,
                project_label,
                form.project.is_some(),
                cx,
            )))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.perspective_select(
                        ViewField::Tag,
                        tag_label,
                        form.tag.is_some(),
                        cx,
                    ))
                    .when(form.tag.is_some(), |this| {
                        this.child(
                            div()
                                .id("modal-tag-children")
                                .flex()
                                .flow_tip(self, cx, theme, "Match tasks tagged with a child tag")
                                .child(widgets::toggle_chip(theme, "Child tags", form.tag_children))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                                        form.tag_children = !form.tag_children;
                                    }
                                    cx.notify();
                                })),
                        )
                    }),
            );

        let windows_row = div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Status,
                form.status_label().to_string(),
                form.status != TaskStatus::Any,
                cx,
            )))
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Due,
                form.due.label().to_string(),
                form.due.window().is_some(),
                cx,
            )))
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Defer,
                form.defer.label().to_string(),
                form.defer.window().is_some(),
                cx,
            )));

        let layout_row = div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Group,
                form.group_label().to_string(),
                form.group != GroupBy::None,
                cx,
            )))
            .child(div().flex_1().min_w_0().child(self.perspective_select(
                ViewField::Sort,
                form.sort_label().to_string(),
                false,
                cx,
            )));

        let heading = |text: &'static str| {
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_faint)
                .child(text)
        };
        let summary = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.field)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(icons::icon(Icon::Perspective, 13.0, theme.text_faint))
                    .child(div().text_sm().text_color(theme.text).child(format!(
                        "{matches} {} match",
                        if matches == 1 { "task" } else { "tasks" }
                    )))
                    .when(extra > 0, |this| {
                        this.child(div().text_xs().text_color(theme.text_faint).child(format!(
                            "· {extra} CLI condition{} kept",
                            if extra == 1 { "" } else { "s" }
                        )))
                    }),
            )
            .when(form.has_filters(), |this| {
                this.child(
                    div()
                        .on_children_prepainted(self.view_probe("clear".to_string()))
                        .child(
                            div()
                                .id("modal-clear-filters")
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(theme.accent)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .child("Clear filters")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                                        form.clear_filters();
                                    }
                                    cx.notify();
                                })),
                        ),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(heading("FILTERS"))
            .child(toggles)
            .child(project_tag_row)
            .child(windows_row)
            .child(widgets::divider(theme))
            .child(heading("LAYOUT"))
            .child(layout_row)
            .child(widgets::divider(theme))
            .child(summary)
    }

    /// Record where a perspective control painted so its menu can be a root
    /// overlay and a GUI probe can click the control or one of its options.
    pub(super) fn view_probe(
        &self,
        key: String,
    ) -> impl Fn(Vec<Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.view_boxes.clone();
        move |bounds, _, _| {
            let Some(bounds) = bounds.first() else {
                return;
            };
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
    }

    /// One labelled menu button in the perspective editor. The value reads like
    /// a field and the chevron says it opens a list rather than cycling.
    fn perspective_select(
        &self,
        field: ViewField,
        value: String,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let key = field.key().to_string();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .min_w_0()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(field.label()),
            )
            .child(
                div().on_children_prepainted(self.view_probe(key)).child(
                    div()
                        .id(super::sid(format!("modal-{}", field.key())))
                        .h(px(28.))
                        .px_2()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .rounded_md()
                        .border_1()
                        .border_color(if active { theme.accent } else { theme.border })
                        .bg(if active { theme.selection } else { theme.field })
                        .text_sm()
                        .text_color(if active { theme.accent } else { theme.text })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(value),
                        )
                        .child(icons::icon(
                            Icon::ChevronDown,
                            11.0,
                            if active {
                                theme.accent
                            } else {
                                theme.text_faint
                            },
                        ))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.toggle_view_menu(field, cx)),
                        ),
                ),
            )
    }

    pub(super) fn view_choices(&self, field: ViewField) -> Vec<ViewChoice> {
        let db = self.store.db();
        match field {
            ViewField::Project => {
                let mut projects: Vec<_> = db.projects.values().collect();
                projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                let mut choices = vec![ViewChoice::Project(None)];
                choices.extend(
                    projects
                        .into_iter()
                        .map(|project| ViewChoice::Project(Some(project.id.clone()))),
                );
                choices
            }
            ViewField::Tag => {
                let mut tags: Vec<_> = db.tags.values().collect();
                tags.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                let mut choices = vec![ViewChoice::Tag(None)];
                choices.extend(
                    tags.into_iter()
                        .map(|tag| ViewChoice::Tag(Some(tag.id.clone()))),
                );
                choices
            }
            ViewField::Status => [
                TaskStatus::Any,
                TaskStatus::Available,
                TaskStatus::Blocked,
                TaskStatus::Deferred,
                TaskStatus::Completed,
            ]
            .into_iter()
            .map(ViewChoice::Status)
            .collect(),
            ViewField::Due => [
                super::perspective::DueChoice::Any,
                super::perspective::DueChoice::Overdue,
                super::perspective::DueChoice::Today,
                super::perspective::DueChoice::Next7Days,
                super::perspective::DueChoice::NoDate,
            ]
            .into_iter()
            .map(ViewChoice::Due)
            .collect(),
            ViewField::Defer => [
                super::perspective::DeferChoice::Any,
                super::perspective::DeferChoice::Today,
                super::perspective::DeferChoice::Next7Days,
                super::perspective::DeferChoice::NoDate,
            ]
            .into_iter()
            .map(ViewChoice::Defer)
            .collect(),
            ViewField::Group => [
                GroupBy::None,
                GroupBy::Project,
                GroupBy::Folder,
                GroupBy::Tag,
                GroupBy::DueDate,
                GroupBy::DeferDate,
                GroupBy::Flagged,
                GroupBy::Estimate,
            ]
            .into_iter()
            .map(ViewChoice::Group)
            .collect(),
            ViewField::Sort => [
                SortBy::Manual,
                SortBy::Due,
                SortBy::Defer,
                SortBy::Name,
                SortBy::Flagged,
                SortBy::Estimate,
                SortBy::Created,
                SortBy::CompletedAt,
            ]
            .into_iter()
            .map(ViewChoice::Sort)
            .collect(),
        }
    }

    fn view_choice_label(&self, choice: &ViewChoice) -> String {
        match choice {
            ViewChoice::Project(None) => "Any".to_string(),
            ViewChoice::Project(Some(id)) => self.store.db().project_path(id),
            ViewChoice::Tag(None) => "Any".to_string(),
            ViewChoice::Tag(Some(id)) => self
                .store
                .db()
                .tags
                .get(id)
                .map(|tag| tag.name.clone())
                .unwrap_or_else(|| "Unknown tag".to_string()),
            ViewChoice::Status(status) => match status {
                TaskStatus::Any => "Any",
                TaskStatus::Available => "Available",
                TaskStatus::Blocked => "Blocked",
                TaskStatus::Deferred => "Deferred",
                TaskStatus::Completed => "Completed",
            }
            .to_string(),
            ViewChoice::Due(choice) => choice.label().to_string(),
            ViewChoice::Defer(choice) => choice.label().to_string(),
            ViewChoice::Group(group) => super::group_label(Some(*group)).to_string(),
            ViewChoice::Sort(sort) => super::sort_label(Some(*sort)).to_string(),
        }
    }

    fn view_choice_selected(&self, form: &PerspectiveForm, choice: &ViewChoice) -> bool {
        match choice {
            ViewChoice::Project(id) => &form.project == id,
            ViewChoice::Tag(id) => &form.tag == id,
            ViewChoice::Status(status) => form.status == *status,
            ViewChoice::Due(choice) => form.due == *choice,
            ViewChoice::Defer(choice) => form.defer == *choice,
            ViewChoice::Group(group) => form.group == *group,
            ViewChoice::Sort(sort) => form.sort == *sort,
        }
    }

    fn toggle_view_menu(&mut self, field: ViewField, cx: &mut Context<Self>) {
        if self.view_menu == Some(field) {
            self.view_menu = None;
        } else {
            let cursor = {
                let Some(Modal::Perspective { form }) = &self.modal else {
                    return;
                };
                self.view_choices(field)
                    .iter()
                    .position(|choice| self.view_choice_selected(form, choice))
                    .unwrap_or(0)
            };
            self.view_menu = Some(field);
            self.view_menu_cursor = cursor;
            self.view_menu_scroll.set_offset(point(px(0.), px(0.)));
        }
        cx.notify();
    }

    pub(super) fn choose_view_choice(&mut self, choice: ViewChoice, cx: &mut Context<Self>) {
        if let Some(Modal::Perspective { form }) = &mut self.modal {
            match choice {
                ViewChoice::Project(id) => form.set_project(id),
                ViewChoice::Tag(id) => form.set_tag(id),
                ViewChoice::Status(status) => form.status = status,
                ViewChoice::Due(choice) => form.due = choice,
                ViewChoice::Defer(choice) => form.defer = choice,
                ViewChoice::Group(group) => form.group = group,
                ViewChoice::Sort(sort) => form.sort = sort,
            }
        }
        self.view_menu = None;
        cx.notify();
    }

    fn render_view_menu(&self, field: ViewField, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div().into_any_element();
        };
        let choices = self.view_choices(field);
        let mut menu = div()
            .id("view-menu")
            .min_w(px(220.))
            .max_w(px(320.))
            .max_h(px(300.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg()
            .overflow_y_scroll()
            .track_scroll(&self.view_menu_scroll);
        for (index, choice) in choices.iter().enumerate() {
            let selected = self.view_choice_selected(form, choice);
            let cursor = self.view_menu_cursor == index;
            let label = self.view_choice_label(choice);
            let slug = choice.slug();
            let option_key = format!("option:{}:{slug}", field.key());
            let captured = choice.clone();
            menu = menu.child(
                div()
                    .on_children_prepainted(self.view_probe(option_key))
                    .child(
                        div()
                            .id(super::sid(format!("view-option:{}:{slug}", field.key())))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .text_xs()
                            .text_color(if selected {
                                theme.accent
                            } else {
                                theme.text_dim
                            })
                            .bg(if selected {
                                theme.selection
                            } else if cursor {
                                theme.hover
                            } else {
                                theme.clear
                            })
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(label),
                            )
                            .when(selected, |this| {
                                this.child(icons::icon(Icon::Check, 11.0, theme.accent))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.choose_view_choice(captured.clone(), cx);
                            })),
                    ),
            );
        }
        menu.into_any_element()
    }

    /// The root-layer menu for the open perspective select. It paints after the
    /// sheet so a project or tag list can extend past the card without being
    /// clipped by the scrolling body.
    pub(super) fn render_view_menu_layer(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let field = self.view_menu?;
        let (x, y) = {
            let boxes = self.view_boxes.borrow();
            let [x, y, width, height] = *boxes.get(field.key())?;
            let anchor = Bounds {
                origin: point(px(x), px(y)),
                size: size(px(width), px(height)),
            };
            let count = self.view_choices(field).len() as f32;
            let menu_height = (count * 25.0 + 10.0).min(300.0);
            super::menu_position(
                anchor,
                self.window_size.unwrap_or((1254.0, 1404.0)),
                240.0,
                menu_height,
            )
        };
        Some(
            div()
                .id("view-menu-layer")
                .absolute()
                .occlude()
                .left(px(x))
                .top(px(y))
                .child(self.render_view_menu(field, cx))
                .into_any_element(),
        )
    }

    fn render_tag_location_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::TagLocation { tag, .. }) = &self.modal else {
            return div();
        };
        let tag = tag.clone();
        div().flex().items_center().gap_2().child(
            div()
                .id("modal-clear-location")
                .child(widgets::button(theme, "Clear location", false))
                .flow_tip(self, cx, theme, "Remove coordinates from this tag")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.execute(flowlist_core::command::Command::UpdateTag {
                        id: tag.clone(),
                        patch: flowlist_core::command::TagPatch {
                            location: Some(None),
                            ..Default::default()
                        },
                    });
                    this.close_modal(cx);
                })),
        )
    }

    fn render_new_project_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let (kind, icon, icon_open) = match &self.modal {
            Some(Modal::NewProject {
                kind,
                icon,
                icon_open,
                ..
            }) => (*kind, *icon, *icon_open),
            _ => (ProjectKind::Parallel, ProjectIcon::default(), false),
        };
        let mut row = div().flex().flex_col().gap_1().child(
            div()
                .text_xs()
                .text_color(theme.text_faint)
                .child("Project type"),
        );
        let mut chips = div().flex().items_center().gap_1();
        for candidate in ProjectKind::ALL {
            let active = kind == candidate;
            chips = chips.child(
                div()
                    .id(super::sid(format!("project-kind:{}", candidate.slug())))
                    .cursor_pointer()
                    .child(widgets::toggle_chip(theme, candidate.label(), active))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(Modal::NewProject { kind, .. }) = &mut this.modal {
                            *kind = candidate;
                        }
                        cx.notify();
                    })),
            );
        }
        row = row.child(chips).child(
            div()
                .text_xs()
                .text_color(theme.text_faint)
                .child(kind.hint()),
        );
        let icon_row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child("Icon"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("new-project-icon-toggle")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(if icon_open {
                                theme.accent
                            } else {
                                theme.border_soft
                            })
                            .bg(if icon_open {
                                theme.selection
                            } else {
                                theme.clear
                            })
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, format!("Icon · {}", icon.label()))
                            .hover(|style| style.bg(theme.hover))
                            .child(widgets::icon_for_project(icon, 14.0, theme.text_dim))
                            .child(icons::icon(
                                super::icons::Icon::ChevronDown,
                                10.0,
                                theme.text_faint,
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::NewProject { icon_open, .. }) = &mut this.modal {
                                    *icon_open = !*icon_open;
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child(icon.label()),
                    ),
            )
            .when(icon_open, |this| {
                let mut icons = div().flex().items_center().gap_1();
                for candidate in ProjectIcon::ALL {
                    let active = icon == candidate;
                    icons = icons.child(
                        div()
                            .id(super::sid(format!("project-icon:{}", candidate.slug())))
                            .size(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .border_1()
                            .border_color(if active {
                                theme.accent
                            } else {
                                theme.border_soft
                            })
                            .bg(if active { theme.selection } else { theme.clear })
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, candidate.label())
                            .hover(|style| style.bg(theme.hover))
                            .child(widgets::icon_for_project(candidate, 14.0, theme.text_dim))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(Modal::NewProject {
                                    icon, icon_open, ..
                                }) = &mut this.modal
                                {
                                    *icon = candidate;
                                    *icon_open = false;
                                }
                                cx.notify();
                            })),
                    );
                }
                this.child(icons)
            });
        row.child(icon_row)
    }

    fn render_settings_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let (selected, appearance) = match &self.modal {
            Some(Modal::Settings {
                theme, appearance, ..
            }) => (*theme, *appearance),
            _ => (theme.kind(), theme.appearance()),
        };
        let vim = matches!(&self.modal, Some(Modal::Settings { vim_mode: true, .. }));
        let follow_system = matches!(
            &self.modal,
            Some(Modal::Settings {
                follow_system: true,
                ..
            })
        );

        let mut modes = div()
            .flex()
            .items_center()
            .gap_1()
            .p_0p5()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field);
        for mode in Appearance::ALL {
            let active = appearance == mode;
            modes = modes.child(
                div()
                    .on_children_prepainted(self.theme_probe(format!("appearance:{}", mode.slug())))
                    .child(
                        div()
                            .id(mode.slug())
                            .h(px(24.))
                            .px_3()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .bg(if active { theme.raised } else { theme.clear })
                            .text_color(if active { theme.text } else { theme.text_dim })
                            .text_xs()
                            .when(active, |this| this.font_weight(gpui::FontWeight::SEMIBOLD))
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .child(mode.label())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(Modal::Settings {
                                    theme, appearance, ..
                                }) = &mut this.modal
                                {
                                    *appearance = mode;
                                    // Preview the choice right away; Cancel restores
                                    // the theme the sheet opened with.
                                    this.theme = Theme::new(*theme, mode);
                                }
                                cx.notify();
                            })),
                    ),
            );
        }

        let mut grid = div().flex().flex_col().gap_2().w_full();
        for chunk in ThemeKind::ALL.chunks(5) {
            let mut row = div().flex().items_start().gap_2().w_full();
            for kind in chunk {
                row = row.child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .on_children_prepainted(self.theme_probe(format!("theme:{}", kind.slug())))
                        .child(self.theme_swatch(*kind, selected == *kind, appearance, cx)),
                );
            }
            // Keep the last row aligned with full rows when it is short.
            for _ in chunk.len()..5 {
                row = row.child(div().flex_1());
            }
            grid = grid.child(row);
        }

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().text_color(theme.text_faint).child("Theme"))
                    .child(modes),
            )
            .child(grid)
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(format!(
                        "{} · {} · ⌘T light/dark · ⇧⌘T next family · Preview is live",
                        selected.hint(),
                        appearance.label(),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("modal-vim-mode")
                            .child(widgets::button(
                                theme,
                                if vim { "Vim mode ✓" } else { "Vim mode off" },
                                vim,
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::Settings { vim_mode, .. }) = &mut this.modal {
                                    *vim_mode = !*vim_mode;
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().text_xs().text_color(theme.text_faint).child(
                        "hjkl motions, counts, operators and pane focus. ⌘ shortcuts stay available.",
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("modal-follow-system")
                            .child(widgets::button(
                                theme,
                                if follow_system {
                                    "Follow system ✓"
                                } else {
                                    "Fixed appearance"
                                },
                                follow_system,
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let Some(Modal::Settings { follow_system, .. }) = &mut this.modal
                                {
                                    *follow_system = !*follow_system;
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().text_xs().text_color(theme.text_faint).child(
                        "Take light or dark from macOS and keep the chosen family.",
                    )),
            )
    }

    fn render_sidebar_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::Settings { sidebar, .. }) = &self.modal else {
            return div().into_any_element();
        };
        let sidebar = sidebar.clone();
        let label = |text: &str| {
            div()
                .text_xs()
                .text_color(theme.text_faint)
                .child(text.to_string())
        };
        let toggle = |key: &'static str,
                      text: &'static str,
                      active: bool,
                      flag: SidebarFlag,
                      cx: &mut Context<Self>| {
            self.sidebar_toggle(key, text, active, flag, cx)
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_2()
            .border_t_1()
            .border_color(theme.border_soft)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Sidebar"),
            )
            .child(label("Sections"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(toggle(
                        "sidebar-show-perspectives",
                        "Perspectives",
                        sidebar.show_perspectives,
                        SidebarFlag::ShowPerspectives,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-show-priorities",
                        "Priorities",
                        sidebar.show_priorities,
                        SidebarFlag::ShowPriorities,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-show-library",
                        "Library",
                        sidebar.show_library,
                        SidebarFlag::ShowLibrary,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-show-saved",
                        "Saved",
                        sidebar.show_saved,
                        SidebarFlag::ShowSaved,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-show-tags",
                        "Tags",
                        sidebar.show_tags,
                        SidebarFlag::ShowTags,
                        cx,
                    )),
            )
            .child(label("Counts"))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(toggle(
                        "sidebar-counts-perspectives",
                        "Perspectives",
                        sidebar.counts_perspectives,
                        SidebarFlag::CountsPerspectives,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-counts-priorities",
                        "Priorities",
                        sidebar.counts_priorities,
                        SidebarFlag::CountsPriorities,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-counts-library",
                        "Library",
                        sidebar.counts_library,
                        SidebarFlag::CountsLibrary,
                        cx,
                    ))
                    .child(toggle(
                        "sidebar-counts-tags",
                        "Tags",
                        sidebar.counts_tags,
                        SidebarFlag::CountsTags,
                        cx,
                    )),
            )
            .child(div().text_xs().text_color(theme.text_faint).child(
                "Sections fold from their heading in the rail. Counts are quiet by default.",
            ))
            .into_any_element()
    }

    fn sidebar_toggle(
        &self,
        key: &'static str,
        label: &'static str,
        active: bool,
        flag: SidebarFlag,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let boxes = self.settings_toggle_boxes.clone();
        let box_key = key.to_string();
        div()
            .id(key)
            .child(
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
                    .child(widgets::button(
                        theme,
                        if active {
                            format!("{label} ✓")
                        } else {
                            label.to_string()
                        },
                        active,
                    )),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(Modal::Settings { sidebar, .. }) = &mut this.modal {
                    flag.toggle(sidebar);
                }
                cx.notify();
            }))
    }

    /// Record where a theme control painted so a GUI probe can click it.
    fn theme_probe(
        &self,
        key: String,
    ) -> impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.theme_boxes.clone();
        move |bounds, _, _| {
            let Some(bounds) = bounds.first() else {
                return;
            };
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
    }

    fn theme_swatch(
        &self,
        kind: ThemeKind,
        selected: bool,
        appearance: Appearance,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let light = Theme::new(kind, Appearance::Light);
        let dark = Theme::new(kind, Appearance::Dark);
        div()
            .id(kind.slug())
            .flex_1()
            .min_w_0()
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(if selected { theme.accent } else { theme.border })
            .bg(if selected {
                theme.selection
            } else {
                theme.field
            })
            .cursor_pointer()
            .hover(|style| {
                style.bg(if selected {
                    theme.selection
                } else {
                    theme.hover
                })
            })
            .child(
                div()
                    .flex()
                    .h(px(38.))
                    .w_full()
                    .rounded_md()
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme.border_soft)
                    .child(self.half_preview(light, appearance == Appearance::Light))
                    .child(self.half_preview(dark, appearance == Appearance::Dark)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_1()
                    .pt_1()
                    .child(
                        div()
                            .min_w_0()
                            .text_xs()
                            .font_weight(if selected {
                                gpui::FontWeight::SEMIBOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if selected { theme.text } else { theme.text_dim })
                            .child(kind.label()),
                    )
                    .when(selected, |this| {
                        this.child(icons::icon(icons::Icon::Check, 11.0, theme.accent))
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(Modal::Settings {
                    theme, appearance, ..
                }) = &mut this.modal
                {
                    *theme = kind;
                    // Show the family on the whole window before Save.
                    this.theme = Theme::new(kind, *appearance);
                }
                cx.notify();
            }))
    }

    /// One half of a theme swatch: the selected appearance is drawn at full
    /// strength, the other side recedes so the pair reads as one family.
    fn half_preview(&self, preview: Theme, active: bool) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(preview.bg)
            .opacity(if active { 1.0 } else { 0.42 })
            .child(div().h(px(9.)).w_full().bg(preview.glass_fill(90.)))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .px_1()
                    .child(div().h(px(4.)).w(px(16.)).rounded_full().bg(preview.accent)),
            )
    }

    fn render_import_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let merge = matches!(&self.modal, Some(Modal::Import { merge: true, .. }));
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("modal-merge")
                    .child(widgets::button(
                        theme,
                        if merge {
                            "Merge ✓"
                        } else {
                            "Replace library"
                        },
                        merge,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Import { merge, .. }) = &mut this.modal {
                            *merge = !*merge;
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(if merge {
                        "Merge adds new items and keeps existing ids."
                    } else {
                        "Replace swaps the whole library for the pasted one."
                    }),
            )
    }
}

fn help_body(theme: super::theme::Theme) -> impl IntoElement {
    let sections: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "Work with tasks",
            vec![
                ("Space", "complete or reopen"),
                ("!", "flag or unflag"),
                ("⏎", "edit the selected title"),
                ("⌘⌫", "delete selection or the sidebar entity"),
                ("Tab / ⇧Tab", "nest / unnest"),
                ("za / zo / zc", "toggle / open / close subtasks"),
                ("⌥↑ / ⌥↓", "move within siblings"),
                ("⌘D", "toggle flag on selection"),
            ],
        ),
        (
            "Checklists",
            vec![
                ("⌘L", "add a checklist step"),
                ("Click ○", "tick or untick a step"),
                ("Click step", "rename a step"),
                ("↑ / ↓ in inspector", "reorder a step"),
                ("×", "delete a step"),
            ],
        ),
        (
            "Navigate",
            vec![
                ("↑ / ↓", "move selection"),
                ("⇧↑ / ⇧↓", "extend selection"),
                ("⌘1…⌘8", "jump to perspective"),
                ("n", "quick capture"),
                ("/", "search"),
                ("⌘F", "search"),
                ("i", "toggle inspector"),
            ],
        ),
        (
            "Vim mode",
            vec![
                ("j / k", "move down / up (counts: 3j)"),
                ("h / l", "outdent / indent, or collapse / expand"),
                ("gg / G", "top / bottom (5G jumps to line 5)"),
                ("Ctrl-d / Ctrl-u", "half page down / up"),
                ("o / O", "new task below / above"),
                ("x", "complete or reopen"),
                ("!", "flag or unflag"),
                ("dd / yy / p", "delete / yank / paste"),
                ("u / Ctrl-r", "undo / redo"),
                ("r / e", "rename / edit title"),
                ("dd", "delete the sidebar entity"),
                ("⇧L", "set a tag or library location"),
                ("⇧J / ⇧K", "move task down / up among siblings"),
                ("Ctrl-w h l j", "focus sidebar / content / checklist"),
                ("?", "this help"),
                ("Esc", "clear counts, selection and search"),
            ],
        ),
        (
            "Library",
            vec![
                ("⌘Z / ⇧⌘Z", "undo / redo"),
                ("⌘N", "quick capture"),
                ("⇧⌘N", "new project"),
                ("⌘T", "toggle light / dark"),
                ("⇧⌘T / ⌥⌘T", "next / previous theme family"),
                ("⇧T", "toggle light / dark in vim mode"),
                ("⌘,", "settings"),
                ("⌘C", "copy selected tasks"),
                ("⌘R", "rename current project/folder/tag/perspective"),
                ("⌥⌘N", "new folder"),
                ("⌃⌘N", "new tag"),
                ("⇧⌘P", "new saved perspective"),
                ("⌘O", "import"),
                ("⇧⌘E", "copy TaskPaper export"),
                ("? / ⌘?", "this help"),
            ],
        ),
        (
            "Mouse",
            vec![
                ("Click", "select a task"),
                ("⌘Click", "add or remove from selection"),
                ("⇧Click", "select a range"),
                ("Double-click", "edit the title"),
                ("Drag", "nest under a task, or drop on a project/tag"),
                ("Click the flag", "toggle flag"),
                ("Click ○", "toggle complete"),
            ],
        ),
    ];
    let mut body = div()
        .id("help-scroll")
        .flex()
        .flex_col()
        .gap_3()
        .max_h(px(480.))
        .overflow_y_scroll();
    for (title, rows) in sections {
        let mut block = div().flex().flex_col().gap_1().child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text_faint)
                .child(title),
        );
        for (key, label) in rows {
            block = block.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(92.))
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(theme.text_dim)
                            .child(key),
                    )
                    .child(div().text_sm().text_color(theme.text_dim).child(label)),
            );
        }
        body = body.child(block);
    }
    body.child(
        div()
            .text_xs()
            .text_color(theme.text_faint)
            .child("The GUI, CLI and MCP server all write to the same library."),
    )
}
