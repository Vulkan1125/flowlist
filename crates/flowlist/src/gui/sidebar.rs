//! The left rail: built-in perspectives, saved perspectives, the project tree
//! and the tag tree. Projects and tags are drop targets for task drags.

use gpui::{AnyElement, Context, IntoElement, Window, div, prelude::*, px};

use flowlist_core::ids::{ContainerId, FolderId, TagId};
use flowlist_core::model::BuiltinPerspective;

use super::drag::DragTask;
use super::modals;
use super::{FlowlistApp, View, widgets};

fn glyph(perspective: BuiltinPerspective) -> &'static str {
    match perspective {
        BuiltinPerspective::Inbox => "▤",
        BuiltinPerspective::Projects => "▦",
        BuiltinPerspective::Forecast => "◷",
        BuiltinPerspective::Flagged => "⚑",
        BuiltinPerspective::Review => "↻",
        BuiltinPerspective::Tags => "◇",
        BuiltinPerspective::Completed => "✓",
        BuiltinPerspective::Nearby => "◎",
    }
}

impl FlowlistApp {
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
            .opacity(0.0)
            .text_color(theme.text_faint)
            .cursor_pointer()
            .hover(|style| style.opacity(1.0).text_color(theme.text).bg(theme.hover))
            .child("✎")
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                let modal = modals::Modal::rename(this, entity.clone());
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
            .opacity(0.0)
            .text_color(theme.text_faint)
            .cursor_pointer()
            .hover(|style| style.opacity(1.0).text_color(theme.danger).bg(theme.hover))
            .child("×")
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.execute(command.clone());
                if this.view == view {
                    this.view = View::Builtin(BuiltinPerspective::Inbox);
                }
                this.save_session();
                cx.notify();
            }))
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
            .w(px(240.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.border)
            .text_sm()
            .overflow_hidden();

        rail = rail.child(widgets::section_label(theme, "PERSPECTIVES"));
        for perspective in BuiltinPerspective::ALL {
            rail = rail.child(self.perspective_row(perspective, cx));
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
                        .child(widgets::button(theme, "＋ Project", false))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_project(this), cx)
                        })),
                )
                .child(
                    div()
                        .id("new-folder")
                        .flex_1()
                        .child(widgets::button(theme, "＋ Folder", false))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_folder(this), cx)
                        })),
                )
                .child(
                    div()
                        .id("new-tag")
                        .flex_1()
                        .child(widgets::button(theme, "＋ Tag", false))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::new_tag(this), cx)
                        })),
                )
                .child(
                    div()
                        .id("new-perspective")
                        .flex_1()
                        .child(widgets::button(theme, "＋ View", false))
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
        div()
            .id(super::sid(format!("perspective:{}", perspective.slug())))
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .mx_2()
            .px_2()
            .rounded_md()
            .bg(if active {
                theme.selection
            } else {
                theme.sidebar
            })
            .text_color(if active { theme.accent } else { theme.text_dim })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .child(div().w(px(16.)).text_center().child(glyph(perspective)))
            .child(div().flex_1().min_w_0().child(perspective.label()))
            .when(count > 0, |this| this.child(widgets::count(theme, count)))
            .on_click(
                cx.listener(move |this, _, _, cx| this.open_view(View::Builtin(perspective), cx)),
            )
    }

    fn custom_perspective_row(
        &self,
        row: PerspectiveRow,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let active = self.view == View::Custom(row.id.clone());
        let id = row.id.clone();
        let rename_id = row.id.clone();
        let delete_id = row.id.clone();
        let delete_view = View::Custom(row.id.clone());
        div()
            .id(super::sid(format!("custom:{}", row.id)))
            .flex()
            .items_center()
            .gap_2()
            .h(px(28.))
            .mx_2()
            .px_2()
            .rounded_md()
            .bg(if active {
                theme.selection
            } else {
                theme.sidebar
            })
            .text_color(if active { theme.accent } else { theme.text_dim })
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .child(div().w(px(16.)).text_center().child("◆"))
            .child(div().flex_1().min_w_0().child(row.name))
            .child(self.rename_button(
                format!("rename-perspective:{rename_id}"),
                modals::EntityRef::Perspective(rename_id),
                cx,
            ))
            .child(self.delete_entity_button(
                format!("delete-perspective:{delete_id}"),
                flowlist_core::command::Command::DeletePerspective { id: delete_id },
                delete_view,
                cx,
            ))
            .on_click(
                cx.listener(move |this, _, _, cx| this.open_view(View::Custom(id.clone()), cx)),
            )
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
                    let count = db.project_count(&id);
                    let view_id = id.clone();
                    let toggle_key = key.clone();
                    let rename_id = id.clone();
                    let delete_id = id.clone();
                    let delete_view = View::Folder(id.clone());
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
                        .child(
                            div()
                                .id(super::sid(format!("folder-toggle:{id}")))
                                .w(px(14.))
                                .text_color(self.theme.text_faint)
                                .child(if open { "▾" } else { "▸" })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.toggle_expanded(toggle_key.clone(), cx);
                                })),
                        )
                        .child(div().w(px(14.)).child("▱"))
                        .child(div().flex_1().min_w_0().overflow_hidden().child(name))
                        .child(self.rename_button(
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
                        .when(count > 0, |this| {
                            this.child(widgets::count(self.theme, count))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_view(View::Folder(view_id.clone()), cx)
                        }));
                    if self.view == View::Folder(id.clone()) {
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
                    let view_id = id.clone();
                    let drop_id = id.clone();
                    let rename_id = id.clone();
                    let active = self.view == View::Project(id.clone());
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
                            self.theme.sidebar
                        })
                        .text_color(if active {
                            self.theme.accent
                        } else {
                            self.theme.text
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(self.theme.hover))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .child(project.name.clone()),
                        )
                        .child(self.rename_button(
                            format!("rename-project:{rename_id}"),
                            modals::EntityRef::Project(rename_id),
                            cx,
                        ))
                        .child(
                            div()
                                .text_xs()
                                .text_color(self.theme.text_faint)
                                .child(format!("{done}/{total}")),
                        )
                        .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                            this.drop_on_project(drop_id.clone(), payload, cx)
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_view(View::Project(view_id.clone()), cx)
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
            let drop_id = tag.clone();
            let rename_id = tag.clone();
            let delete_id = tag.clone();
            let delete_view = View::Tag(tag.clone());
            let active = self.view == View::Tag(tag.clone());
            let toggle_key = key.clone();
            let depth_pad = if parent.is_some() { 26.0 } else { 18.0 };
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
                    self.theme.sidebar
                })
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
                        .child(if open { "▾" } else { "▸" })
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
                        .w(px(12.))
                        .text_color(if located {
                            self.theme.success
                        } else {
                            self.theme.text_faint
                        })
                        .child(if located { "◉" } else { "#" }),
                )
                .child(div().flex_1().min_w_0().overflow_hidden().child(name))
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
                .child(self.rename_button(
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
                .when(usage > 0, |this| {
                    this.child(widgets::count(self.theme, usage))
                })
                .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                    this.drop_on_tag(drop_id.clone(), payload, cx)
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.open_view(View::Tag(view_id.clone()), cx)
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
