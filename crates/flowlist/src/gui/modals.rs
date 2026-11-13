//! Modal sheets: new project/folder/tag/perspective, import, settings,
//! locations and the keyboard help card.

use gpui::{AnyElement, Context, IntoElement, Window, div, prelude::*, px, rgba};

use flowlist_core::ids::{PerspectiveId, ProjectId, TagId};

use super::perspective::PerspectiveForm;
use super::text::TextBuffer;
use super::{FlowlistApp, widgets};

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
        radius: TextBuffer,
        vim_mode: bool,
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
        Modal::Rename {
            entity,
            name: TextBuffer::new(name),
        }
    }

    pub fn new_project(_app: &FlowlistApp) -> Self {
        Modal::NewProject {
            name: TextBuffer::empty(),
            notes: TextBuffer::empty(),
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
            radius: TextBuffer::new(settings.nearby_radius_km.to_string()),
            vim_mode: app.vim_mode,
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

    pub fn field_count(&self) -> usize {
        match self {
            Modal::Rename { .. } => 1,
            Modal::NewProject { .. } => 2,
            Modal::NewFolder { .. } | Modal::NewTag { .. } => 1,
            Modal::Perspective { .. } => 1,
            Modal::Import { .. } => 1,
            Modal::Settings { .. } => 3,
            Modal::Location { .. } | Modal::TagLocation { .. } => 2,
            Modal::Help => 0,
        }
    }

    pub fn field(&self, index: usize) -> Option<&TextBuffer> {
        match self {
            Modal::Rename { name, .. } => (index == 0).then_some(name),
            Modal::NewProject { name, notes } => match index {
                0 => Some(name),
                1 => Some(notes),
                _ => None,
            },
            Modal::NewFolder { name } | Modal::NewTag { name } => (index == 0).then_some(name),
            Modal::Perspective { form } => (index == 0).then_some(&form.name),
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                radius,
                ..
            } => match index {
                0 => Some(review_days),
                1 => Some(completed_days),
                2 => Some(radius),
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
            Modal::NewProject { name, notes } => match index {
                0 => Some(name),
                1 => Some(notes),
                _ => None,
            },
            Modal::NewFolder { name } | Modal::NewTag { name } => (index == 0).then_some(name),
            Modal::Perspective { form } => (index == 0).then_some(&mut form.name),
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                radius,
                ..
            } => match index {
                0 => Some(review_days),
                1 => Some(completed_days),
                2 => Some(radius),
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
            Modal::NewFolder { .. } | Modal::NewTag { .. } => "Name",
            Modal::Perspective { .. } => "Name",
            Modal::Import { .. } => "Library text",
            Modal::Settings { .. } => match index {
                0 => "Default review days",
                1 => "Completed history (days)",
                2 => "Nearby radius (km)",
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
        let is_help = matches!(modal, Modal::Help);
        let is_perspective = matches!(modal, Modal::Perspective { .. });
        let is_import = matches!(modal, Modal::Import { .. });
        let is_settings = matches!(modal, Modal::Settings { .. });
        let is_tag_location = matches!(modal, Modal::TagLocation { .. });
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
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .child(
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
                    )
                    .child(
                        div()
                            .id("modal-close")
                            .cursor_pointer()
                            .text_color(theme.text_faint)
                            .hover(|style| style.text_color(theme.text))
                            .tooltip(widgets::tip(theme, "Close (Esc)"))
                            .child("×")
                            .on_click(cx.listener(|this, _, _, cx| this.close_modal(cx))),
                    ),
            );

        if is_help {
            card = card.child(help_body(theme));
        } else {
            for index in 0..field_count {
                card = card.child(self.render_modal_field(index, cx));
            }
            if is_perspective {
                card = card.child(self.render_perspective_controls(cx));
            }
            if is_import {
                card = card.child(self.render_import_controls(cx));
            }
            if is_settings {
                card = card.child(self.render_settings_controls(cx));
            }
            if is_tag_location {
                card = card.child(self.render_tag_location_controls(cx));
            }
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
                            .child("Esc close · ⇥ next field"),
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
                                    .child(widgets::button(theme, "Save", true))
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
            widgets::text_field(theme, buffer, "", focused).into_any_element()
        };
        div()
            .id(super::sid(format!("modal-field:{index}")))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child(label))
            .child(field)
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
                .tooltip(widgets::tip(theme, "Include completed tasks"))
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
                    .tooltip(widgets::tip(theme, "Match tasks tagged with a child tag"))
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
                .tooltip(widgets::tip(theme, "Remove coordinates from this tag"))
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

    fn render_settings_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let vim = matches!(&self.modal, Some(Modal::Settings { vim_mode: true, .. }));
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
            ))
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
                ("⌘T / ⇧T", "switch light / dark"),
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
                ("Click ⚑", "toggle flag"),
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
