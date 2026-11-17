//! Modal sheets: new project/folder/tag/perspective, import, settings,
//! locations and the keyboard help card.

use gpui::{AnyElement, Context, IntoElement, Window, div, prelude::*, px, rgba};

use flowlist_core::ids::{FolderId, PerspectiveId, ProjectId, TagId, TaskId};
use flowlist_core::model::{ProjectIcon, ProjectKind};

use super::perspective::PerspectiveForm;
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

pub enum Modal {
    Rename {
        entity: EntityRef,
        name: TextBuffer,
    },
    NewProject {
        name: TextBuffer,
        notes: TextBuffer,
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

    pub fn new_project(_app: &FlowlistApp) -> Self {
        Modal::NewProject {
            name: TextBuffer::empty(),
            notes: TextBuffer::empty(),
            kind: ProjectKind::Parallel,
            icon: ProjectIcon::default(),
            icon_open: false,
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
                "Its projects move up to its parent. Undo restores the folder."
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
            _ => "Save",
        }
    }

    pub fn field_count(&self) -> usize {
        match self {
            Modal::Rename { .. } => 1,
            Modal::NewProject { .. } => 2,
            Modal::CreateProject { .. } => 0,
            Modal::DeleteProject { .. } => 0,
            Modal::DeleteFolder { .. } => 0,
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
            Modal::NewProject { name, notes, .. } => match index {
                0 => Some(name),
                1 => Some(notes),
                _ => None,
            },
            Modal::CreateProject { .. } => None,
            Modal::DeleteProject { .. } => None,
            Modal::DeleteFolder { .. } => None,
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
            Modal::NewProject { name, notes, .. } => match index {
                0 => Some(name),
                1 => Some(notes),
                _ => None,
            },
            Modal::CreateProject { .. } => None,
            Modal::DeleteProject { .. } => None,
            Modal::DeleteFolder { .. } => None,
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
                _ => "",
            },
            Modal::CreateProject { .. } => "",
            Modal::DeleteProject { .. } => "",
            Modal::DeleteFolder { .. } => "",
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
            if let Modal::DeleteProject { name, .. } | Modal::DeleteFolder { name, .. } = modal {
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

    fn render_perspective_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(Modal::Perspective { form }) = &self.modal else {
            return div();
        };
        let form = form.clone();
        let (projects, tags) = {
            let db = self.store.db();
            let mut projects: Vec<(ProjectId, String)> = db
                .projects
                .values()
                .map(|project| (project.id.clone(), project.name.clone()))
                .collect();
            projects.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            let mut tags: Vec<(TagId, String)> = db
                .tags
                .values()
                .map(|tag| (tag.id.clone(), tag.name.clone()))
                .collect();
            tags.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
            (projects, tags)
        };
        let project_label = form
            .project
            .as_ref()
            .and_then(|id| projects.iter().find(|(candidate, _)| candidate == id))
            .map(|(_, name)| name.as_str())
            .unwrap_or("Any");
        let tag_label = form
            .tag
            .as_ref()
            .and_then(|id| tags.iter().find(|(candidate, _)| candidate == id))
            .map(|(_, name)| name.as_str())
            .unwrap_or("Any");

        let toggle = |label: String, active: bool, id: &'static str| {
            div().id(id).child(widgets::button(theme, label, active))
        };

        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                toggle(
                    if form.inbox_only {
                        "Inbox only ✓".to_string()
                    } else {
                        "Inbox only".to_string()
                    },
                    form.inbox_only,
                    "modal-inbox",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.toggle_inbox();
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    if form.flagged_only {
                        "Flagged only ✓".to_string()
                    } else {
                        "Flagged only".to_string()
                    },
                    form.flagged_only,
                    "modal-flagged",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.flagged_only = !form.flagged_only;
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    if form.show_completed {
                        "Completed ✓".to_string()
                    } else {
                        "Completed".to_string()
                    },
                    form.show_completed,
                    "modal-completed",
                )
                .flow_tip(self, cx, theme, "Include completed tasks")
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.show_completed = !form.show_completed;
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    format!("Project · {project_label}"),
                    form.project.is_some(),
                    "modal-project",
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    let projects = projects.clone();
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        let next = match &form.project {
                            None => projects.first().map(|(id, _)| id.clone()),
                            Some(current) => {
                                let index = projects
                                    .iter()
                                    .position(|(id, _)| id == current)
                                    .map(|index| index + 1)
                                    .unwrap_or(0);
                                projects.get(index).map(|(id, _)| id.clone())
                            }
                        };
                        form.set_project(next);
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    format!("Tag · {tag_label}"),
                    form.tag.is_some(),
                    "modal-tag",
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    let tags = tags.clone();
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        let next = match &form.tag {
                            None => tags.first().map(|(id, _)| id.clone()),
                            Some(current) => {
                                let index = tags
                                    .iter()
                                    .position(|(id, _)| id == current)
                                    .map(|index| index + 1)
                                    .unwrap_or(0);
                                tags.get(index).map(|(id, _)| id.clone())
                            }
                        };
                        form.set_tag(next);
                    }
                    cx.notify();
                })),
            )
            .when(form.tag.is_some(), |this| {
                this.child(
                    toggle(
                        if form.tag_children {
                            "Child tags ✓".to_string()
                        } else {
                            "Child tags".to_string()
                        },
                        form.tag_children,
                        "modal-tag-children",
                    )
                    .flow_tip(self, cx, theme, "Match tasks tagged with a child tag")
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.tag_children = !form.tag_children;
                        }
                        cx.notify();
                    })),
                )
            })
            .child(
                toggle(
                    format!("Status · {}", form.status_label()),
                    form.status != flowlist_core::model::TaskStatus::Any,
                    "modal-status",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.cycle_status();
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    format!("Due · {}", form.due.label()),
                    form.due.window().is_some(),
                    "modal-due",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.cycle_due();
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    format!("Defer · {}", form.defer.label()),
                    form.defer.window().is_some(),
                    "modal-defer",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.cycle_defer();
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    format!("Group · {}", form.group_label()),
                    form.group != flowlist_core::model::GroupBy::None,
                    "modal-group",
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if let Some(Modal::Perspective { form }) = &mut this.modal {
                        form.cycle_group();
                    }
                    cx.notify();
                })),
            )
            .child(
                toggle(format!("Sort · {}", form.sort_label()), false, "modal-sort").on_click(
                    cx.listener(|this, _, _, cx| {
                        if let Some(Modal::Perspective { form }) = &mut this.modal {
                            form.cycle_sort();
                        }
                        cx.notify();
                    }),
                ),
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
