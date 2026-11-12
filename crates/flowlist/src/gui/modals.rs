//! Modal sheets: new project/folder/tag/perspective, import, settings,
//! locations and the keyboard help card.

use gpui::{AnyElement, Context, IntoElement, Window, div, prelude::*, px, rgba};

use flowlist_core::model::GroupBy;
use flowlist_core::model::SortBy;

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
    NewPerspective {
        name: TextBuffer,
        group: GroupBy,
        sort: SortBy,
        flagged_only: bool,
    },
    Import {
        text: TextBuffer,
        merge: bool,
    },
    Settings {
        review_days: TextBuffer,
        completed_days: TextBuffer,
        radius: TextBuffer,
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
        Modal::NewPerspective {
            name: TextBuffer::empty(),
            group: GroupBy::Project,
            sort: SortBy::Due,
            flagged_only: false,
        }
    }

    pub fn settings(app: &FlowlistApp) -> Self {
        let settings = &app.store.db().settings;
        Modal::Settings {
            review_days: TextBuffer::new(settings.default_review_days.to_string()),
            completed_days: TextBuffer::new(settings.show_completed_days.to_string()),
            radius: TextBuffer::new(settings.nearby_radius_km.to_string()),
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
            Modal::NewPerspective { .. } => "New perspective",
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
            Modal::NewPerspective { .. } => "Saved filters with their own grouping and sort order.",
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
            Modal::NewFolder { .. } | Modal::NewTag { .. } | Modal::NewPerspective { .. } => 1,
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
            Modal::NewPerspective { name, .. } => (index == 0).then_some(name),
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                radius,
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
            Modal::NewPerspective { name, .. } => (index == 0).then_some(name),
            Modal::Import { text, .. } => (index == 0).then_some(text),
            Modal::Settings {
                review_days,
                completed_days,
                radius,
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
            Modal::NewPerspective { .. } => "Name",
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
            _ => 460.0,
        }
    }
}

fn next_group(group: GroupBy) -> GroupBy {
    match group {
        GroupBy::None => GroupBy::Project,
        GroupBy::Project => GroupBy::Tag,
        GroupBy::Tag => GroupBy::Folder,
        GroupBy::Folder => GroupBy::DueDate,
        GroupBy::DueDate => GroupBy::DeferDate,
        GroupBy::DeferDate => GroupBy::Flagged,
        GroupBy::Flagged => GroupBy::Estimate,
        GroupBy::Estimate => GroupBy::None,
    }
}

fn group_label(group: GroupBy) -> &'static str {
    match group {
        GroupBy::None => "None",
        GroupBy::Project => "Project",
        GroupBy::Tag => "Tag",
        GroupBy::Folder => "Folder",
        GroupBy::DueDate => "Due",
        GroupBy::DeferDate => "Defer",
        GroupBy::Flagged => "Flagged",
        GroupBy::Estimate => "Estimate",
    }
}

fn next_sort(sort: SortBy) -> SortBy {
    match sort {
        SortBy::Manual => SortBy::Due,
        SortBy::Due => SortBy::Defer,
        SortBy::Defer => SortBy::Name,
        SortBy::Name => SortBy::Flagged,
        SortBy::Flagged => SortBy::Estimate,
        SortBy::Estimate => SortBy::Created,
        SortBy::Created => SortBy::Manual,
        SortBy::CompletedAt => SortBy::Manual,
    }
}

fn sort_label(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Manual => "Manual",
        SortBy::Due => "Due",
        SortBy::Defer => "Defer",
        SortBy::Name => "Name",
        SortBy::Flagged => "Flagged",
        SortBy::Estimate => "Estimate",
        SortBy::Created => "Created",
        SortBy::CompletedAt => "Completed",
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
        let is_perspective = matches!(modal, Modal::NewPerspective { .. });
        let is_import = matches!(modal, Modal::Import { .. });
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
        let Some(Modal::NewPerspective {
            group,
            sort,
            flagged_only,
            ..
        }) = &self.modal
        else {
            return div();
        };
        let group = *group;
        let sort = *sort;
        let flagged = *flagged_only;
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("modal-group")
                    .child(widgets::button(
                        theme,
                        format!("Group · {}", group_label(group)),
                        false,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::NewPerspective { group, .. }) = &mut this.modal {
                            *group = next_group(*group);
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("modal-sort")
                    .child(widgets::button(
                        theme,
                        format!("Sort · {}", sort_label(sort)),
                        false,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::NewPerspective { sort, .. }) = &mut this.modal {
                            *sort = next_sort(*sort);
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("modal-flagged")
                    .child(widgets::button(
                        theme,
                        if flagged {
                            "Flagged only ✓"
                        } else {
                            "Flagged only"
                        },
                        flagged,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(Modal::NewPerspective { flagged_only, .. }) = &mut this.modal {
                            *flagged_only = !*flagged_only;
                        }
                        cx.notify();
                    })),
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
                ("⌘⌫", "delete selection"),
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
                ("1…8", "built-in perspectives"),
                ("n", "quick capture"),
                ("/", "search"),
                ("⌘F", "search"),
                ("i", "toggle inspector"),
                ("⌘1…⌘8", "jump to perspective"),
            ],
        ),
        (
            "Library",
            vec![
                ("⌘Z / ⇧⌘Z", "undo / redo"),
                ("⌘N", "quick capture"),
                ("⇧⌘N", "new project"),
                ("⌘,", "settings"),
                ("⌘C", "copy selected tasks"),
                ("⌘R", "rename current project/folder/tag/perspective"),
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
