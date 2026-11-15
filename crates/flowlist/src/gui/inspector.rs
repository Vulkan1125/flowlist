//! The right-hand inspector: edit one task (or project) without leaving the
//! list. Every field commits through the same command layer as the CLI.

use gpui::{AnyElement, Context, IntoElement, Window, div, prelude::*, px};

use flowlist_core::command::{Command, ProjectPatch};
use flowlist_core::ids::{ChecklistId, TaskId};
use flowlist_core::model::{Priority, ProjectIcon, ProjectStatus};
use flowlist_core::time::{format_duration, human_date, now};

use super::icons::Icon;
use super::widgets::TipExt;
use super::{EditField, FlowlistApp, View, icons, widgets};

impl FlowlistApp {
    pub(super) fn render_inspector(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let mut panel = div()
            .flex()
            .flex_col()
            .w(px(320.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.glass_fill(180.))
            .border_l_1()
            .border_color(theme.border)
            .text_sm()
            .overflow_hidden();

        panel = panel.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .h(px(38.))
                .border_b_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_faint)
                        .child("INSPECTOR"),
                )
                .child(
                    div()
                        .id("close-inspector")
                        .cursor_pointer()
                        .text_color(theme.text_faint)
                        .hover(|style| style.text_color(theme.text))
                        .flow_tip(self, cx, theme, "Close inspector (i)")
                        .child(icons::icon(Icon::Close, 14.0, theme.text_faint))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_inspector = false;
                            this.save_session();
                            cx.notify();
                        })),
                ),
        );

        let content = if let Some(id) = self.selected_one() {
            self.render_task_inspector(&id, cx).into_any_element()
        } else if let View::Project(id) = self.view.clone() {
            self.render_project_inspector(&id, cx).into_any_element()
        } else {
            self.render_empty_inspector().into_any_element()
        };

        panel.child(
            div()
                .id("inspector-scroll")
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(content),
        )
    }

    fn render_empty_inspector(&self) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .p_6()
            .text_center()
            .child(icons::icon(Icon::Info, 30.0, theme.text_faint))
            .child(
                div()
                    .text_color(theme.text_dim)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Nothing selected"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_faint)
                    .child("Select a task to see and edit its details."),
            )
    }

    fn render_task_inspector(&self, id: &TaskId, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let Some(task) = db.tasks.get(id) else {
            return self.render_empty_inspector().into_any_element();
        };
        let id = id.clone();
        let draft = self.draft_task.as_ref() == Some(&id);
        let title = task.title.clone();
        let notes = task.notes.clone();
        let completed = task.completed;
        let flagged = task.flagged;
        let priority = task.priority;
        let project = task
            .project
            .as_ref()
            .map(|project| db.project_path(project));
        let project_id = task.project.clone();
        let tags: Vec<(flowlist_core::ids::TagId, String)> = task
            .tags
            .iter()
            .filter_map(|tag_id| {
                db.tags
                    .get(tag_id)
                    .map(|tag| (tag_id.clone(), db.tag_path(&tag.id)))
            })
            .collect();
        let defer = task.defer.map(|stamp| human_date(&stamp, now()));
        let due = task.due.map(|stamp| human_date(&stamp, now()));
        let estimate = task.estimate_min.map(format_duration);
        let repeat = task.repeat.as_ref().map(|rule| rule.description());
        let checklist: Vec<(ChecklistId, String, bool)> = task
            .checklist
            .iter()
            .map(|item| (item.id.clone(), item.title.clone(), item.completed))
            .collect();
        let checklist_progress = task.checklist_progress();
        let attachments = task.attachments.clone();
        let sequential = task.sequential;
        let availability = flowlist_core::query::availability::availability(db, &id, now());
        let created = task
            .created_at
            .with_timezone(&chrono::Local)
            .format("%b %-d, %Y %H:%M")
            .to_string();
        let modified = task
            .modified_at
            .with_timezone(&chrono::Local)
            .format("%b %-d, %Y %H:%M")
            .to_string();
        let actions_id = id.clone();
        let delete_id = id.clone();
        let tag_open_id = project_id.clone();

        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("inspector-check")
                            .child(widgets::checkbox(theme, completed, theme.accent))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_complete(actions_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(if completed {
                                theme.text_faint
                            } else {
                                theme.text_dim
                            })
                            .child(if completed {
                                "Completed".to_string()
                            } else {
                                availability.label().to_string()
                            }),
                    )
                    .child(
                        div()
                            .id("inspector-flag")
                            .cursor_pointer()
                            .text_color(if flagged {
                                theme.danger
                            } else {
                                theme.text_faint
                            })
                            .flow_tip(
                                self,
                                cx,
                                theme,
                                if flagged {
                                    "Unflag task (!)"
                                } else {
                                    "Flag task (!)"
                                },
                            )
                            .child(icons::icon(
                                Icon::Flagged,
                                12.0,
                                if flagged {
                                    theme.danger
                                } else {
                                    theme.text_faint
                                },
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_flag(delete_id.clone(), cx)
                            })),
                    ),
            )
            .child(self.inspector_text_field("Title", EditField::Title, &title, cx))
            .child(self.inspector_priority(&id, priority, cx))
            .child(self.inspector_notes(&notes, cx))
            .child(self.inspector_checklist(&id, &checklist, checklist_progress, cx))
            .child(self.inspector_task_type(sequential, cx))
            .child(self.inspector_attachments(&attachments, cx))
            .child(self.inspector_tag_section(&tags, cx))
            .child(self.inspector_project_row(project, project_id, cx))
            .child(self.inspector_text_field(
                "Defer",
                EditField::Defer,
                &defer.unwrap_or_default(),
                cx,
            ))
            .child(self.inspector_text_field("Due", EditField::Due, &due.unwrap_or_default(), cx))
            .child(self.inspector_text_field(
                "Estimate",
                EditField::Estimate,
                &estimate.unwrap_or_default(),
                cx,
            ))
            .child(self.inspector_text_field(
                "Repeat",
                EditField::Repeat,
                &repeat.unwrap_or_default(),
                cx,
            ))
            .child(widgets::divider(theme))
            .when(!draft, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .id("convert-task")
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .text_xs()
                                        .text_color(theme.text_dim)
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.hover))
                                        .flow_tip(
                                            self,
                                            cx,
                                            theme,
                                            "Turn this task and its subtasks into a project",
                                        )
                                        .child("To project")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.convert_selected_to_project(cx)
                                        })),
                                )
                                .child(
                                    div()
                                        .id("delete-task")
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .text_xs()
                                        .text_color(theme.danger)
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.hover))
                                        .child("Delete")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.delete_selected(cx)),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("complete-task")
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(theme.accent)
                                        .text_xs()
                                        .text_color(theme.accent)
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.selection))
                                        .child(if completed { "Reopen" } else { "Complete" })
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Some(id) = this.selected_one() {
                                                this.toggle_complete(id, cx)
                                            }
                                        })),
                                ),
                        ),
                )
            })
            .when_some(tag_open_id, |this, project| {
                this.child(
                    div()
                        .id("open-project")
                        .text_xs()
                        .text_color(theme.accent)
                        .cursor_pointer()
                        .child("Open project →")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_view(View::Project(project.clone()), cx)
                        })),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pt_2()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(format!("Created {created}"))
                    .child(format!("Modified {modified}")),
            );
        if let Some(error) = &self.field_error {
            body = body.child(
                div()
                    .text_xs()
                    .text_color(theme.danger)
                    .child(error.clone()),
            );
        }
        body.into_any_element()
    }

    fn inspector_priority(
        &self,
        id: &TaskId,
        current: Option<Priority>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let mut chips = div().flex().items_center().gap_1();
        chips = chips.child(
            div()
                .id(super::sid(format!("priority-none:{id}")))
                .cursor_pointer()
                .child(widgets::toggle_chip(theme, "None", current.is_none()))
                .on_click(cx.listener({
                    let id = id.clone();
                    move |this, _, _, cx| this.set_task_priority(id.clone(), None, cx)
                })),
        );
        for priority in Priority::ALL {
            let active = current == Some(priority);
            chips = chips.child(
                div()
                    .id(super::sid(format!("priority-{priority:?}:{id}")))
                    .cursor_pointer()
                    .child(widgets::toggle_chip(theme, priority.label(), active))
                    .on_click(cx.listener({
                        let id = id.clone();
                        move |this, _, _, cx| this.set_task_priority(id.clone(), Some(priority), cx)
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Priority"),
            )
            .child(chips)
    }

    fn inspector_text_field(
        &self,
        label: &'static str,
        field: EditField,
        value: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing = self.editing == Some(field) && self.focus == super::FocusArea::Field;
        let placeholder = match field {
            EditField::Title => "Task title",
            EditField::Defer => "No defer date",
            EditField::Due => "No due date",
            EditField::Estimate => "No estimate (e.g. 45m)",
            EditField::Repeat => "Does not repeat (e.g. every week)",
            EditField::Project => "Inbox",
            EditField::Notes => "Notes",
        };
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child(label))
            .child(if editing {
                div()
                    .id("field-editor")
                    .child(widgets::text_field(theme, &self.field, placeholder, true))
                    .into_any_element()
            } else {
                div()
                    .id(super::sid(format!(
                        "field:{}",
                        label.to_lowercase().replace(' ', "-")
                    )))
                    .h(px(28.))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border_soft)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .text_color(if value.is_empty() {
                        theme.text_faint
                    } else {
                        theme.text
                    })
                    .child(if value.is_empty() {
                        placeholder.to_string()
                    } else {
                        value.to_string()
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.begin_edit(field, cx)))
                    .into_any_element()
            });
        if editing {
            row = row.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("⏎ save · ⇥ next · esc cancel"),
            );
        }
        row
    }

    fn inspector_notes(&self, notes: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let editing =
            self.editing == Some(EditField::Notes) && self.focus == super::FocusArea::Field;
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child("Notes"));
        if editing {
            row = row.child(widgets::text_area(
                theme,
                &self.field,
                "Add notes",
                true,
                140.0,
            ));
        } else {
            row = row.child(
                div()
                    .id("field:notes")
                    .w_full()
                    .min_w_0()
                    .whitespace_normal()
                    .min_h(px(52.))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border_soft)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .text_sm()
                    .text_color(if notes.is_empty() {
                        theme.text_faint
                    } else {
                        theme.text
                    })
                    .child(if notes.is_empty() {
                        "Add notes".to_string()
                    } else {
                        notes.to_string()
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.begin_edit(EditField::Notes, cx))),
            );
        }
        row
    }

    fn inspector_checklist(
        &self,
        task_id: &TaskId,
        items: &[(ChecklistId, String, bool)],
        progress: (usize, usize),
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing_item = self
            .checklist_editing
            .as_ref()
            .filter(|_| self.focus == super::FocusArea::Checklist);
        let mut section = div().flex().flex_col().gap_1().child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("Checklist"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if progress.1 > 0 && progress.0 == progress.1 {
                            theme.success
                        } else {
                            theme.text_faint
                        })
                        .child(format!("{}/{}", progress.0, progress.1)),
                ),
        );

        for (index, (item_id, title, completed)) in items.iter().enumerate() {
            let is_editing = editing_item == Some(item_id);
            let cursor = self.focus == super::FocusArea::ChecklistList
                && self.checklist_cursor == Some(index);
            let toggle_task = task_id.clone();
            let toggle_item = item_id.clone();
            let edit_item = item_id.clone();
            let edit_title = title.clone();
            let move_task = task_id.clone();
            let move_item = item_id.clone();
            let delete_task = task_id.clone();
            let delete_item = item_id.clone();

            let mut row = div()
                .id(super::sid(format!("checklist:{item_id}")))
                .flex()
                .items_center()
                .gap_2()
                .min_h(px(26.))
                .px_1()
                .rounded_sm()
                .hover(|style| style.bg(theme.hover))
                .when(cursor, |this| this.bg(theme.selection))
                .child(
                    div()
                        .id(super::sid(format!("checklist-toggle:{item_id}")))
                        .flex_shrink_0()
                        .flow_tip(
                            self,
                            cx,
                            theme,
                            if *completed {
                                "Mark step incomplete"
                            } else {
                                "Mark step complete"
                            },
                        )
                        .child(widgets::checkbox(theme, *completed, theme.accent))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_checklist_item(
                                toggle_task.clone(),
                                toggle_item.clone(),
                                cx,
                            );
                        })),
                );

            if is_editing {
                row = row.child(
                    div()
                        .id("checklist-editor")
                        .flex_1()
                        .child(widgets::text_field(
                            theme,
                            &self.checklist_buffer,
                            "Checklist item",
                            true,
                        )),
                );
            } else {
                row = row.child(
                    div()
                        .id(super::sid(format!("checklist-title:{item_id}")))
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_sm()
                        .text_color(if *completed {
                            theme.text_faint
                        } else {
                            theme.text
                        })
                        .when(*completed, |this| this.line_through())
                        .child(title.clone())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.begin_checklist_edit(edit_item.clone(), edit_title.clone(), cx)
                        })),
                );
            }

            row = row.child(
                div()
                    .id(super::sid(format!("checklist-up:{item_id}")))
                    .w(px(16.))
                    .text_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .opacity(if index == 0 { 0.3 } else { 0.8 })
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.text).opacity(1.0))
                    .flow_tip(self, cx, theme, "Move step up")
                    .child(icons::icon(Icon::ArrowUp, 12.0, theme.text_faint))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.move_checklist_item(
                            move_task.clone(),
                            move_item.clone(),
                            index.saturating_sub(1),
                            cx,
                        );
                    })),
            );
            let down_task = task_id.clone();
            let down_item = item_id.clone();
            row = row.child(
                div()
                    .id(super::sid(format!("checklist-down:{item_id}")))
                    .w(px(16.))
                    .text_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .opacity(if index + 1 >= items.len() { 0.3 } else { 0.8 })
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.text).opacity(1.0))
                    .flow_tip(self, cx, theme, "Move step down")
                    .child(icons::icon(Icon::ArrowDown, 12.0, theme.text_faint))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.move_checklist_item(
                            down_task.clone(),
                            down_item.clone(),
                            index + 1,
                            cx,
                        );
                    })),
            );
            row = row.child(
                div()
                    .id(super::sid(format!("checklist-delete:{item_id}")))
                    .w(px(16.))
                    .text_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.danger))
                    .flow_tip(self, cx, theme, "Delete step")
                    .child(icons::icon(Icon::Close, 12.0, theme.text_faint))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.delete_checklist_item(delete_task.clone(), delete_item.clone(), cx);
                    })),
            );
            section = section.child(row);
        }

        if items.is_empty() && editing_item.is_none() {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No steps yet."),
            );
        }

        if editing_item.is_none() {
            section = section.child(
                div()
                    .id("checklist-add")
                    .child(widgets::text_field(
                        theme,
                        &self.checklist_buffer,
                        "Add a step…",
                        self.focus == super::FocusArea::Checklist,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.focus = super::FocusArea::Checklist;
                        this.checklist_editing = None;
                        this.checklist_buffer.clear();
                        cx.notify();
                    })),
            );
        } else {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("⏎ save · esc cancel"),
            );
        }

        section
    }

    /// A compact completion list under the project and tag fields. Rows are
    /// clickable and Tab accepts the highlighted one.
    fn inspector_suggestions(
        &self,
        suggestions: Vec<String>,
        tags: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let highlighted = (tags || self.suggestion_engaged).then(|| {
            self.suggestion_index
                .min(suggestions.len().saturating_sub(1))
        });
        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .bg(theme.raised);
        for (index, path) in suggestions.into_iter().enumerate() {
            let pick = path.clone();
            list = list.child(
                div()
                    .id(super::sid(format!("suggestion:{index}:{path}")))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if highlighted == Some(index) {
                        theme.accent
                    } else {
                        theme.text_dim
                    })
                    .bg(if highlighted == Some(index) {
                        theme.selection
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(path)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if tags {
                            this.choose_tag_suggestion(pick.clone(), cx);
                        } else {
                            this.choose_project_suggestion(pick.clone(), cx);
                        }
                    })),
            );
        }
        list
    }

    fn inspector_tag_section(
        &self,
        tags: &[(flowlist_core::ids::TagId, String)],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let has_tag = self.selected_one();
        let mut section =
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(theme.text_faint).child("Tags"))
                .child(div().flex().flex_wrap().gap_1().children(tags.iter().map(
                    |(tag_id, path)| {
                        let remove = tag_id.clone();
                        div()
                            .id(super::sid(format!("tag-chip:{tag_id}")))
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_0p5()
                            .rounded_sm()
                            .bg(theme.accent_soft)
                            .text_color(theme.accent)
                            .text_xs()
                            .child(path.clone())
                            .child(
                                div()
                                    .id(super::sid(format!("remove-tag:{}", remove)))
                                    .cursor_pointer()
                                    .hover(|style| style.text_color(theme.danger))
                                    .flow_tip(self, cx, theme, "Remove tag")
                                    .child(icons::icon(Icon::Close, 12.0, theme.text_faint))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(id) = this.selected_one() {
                                            this.execute(Command::SetTaskTags {
                                                ids: vec![id],
                                                add: Vec::new(),
                                                remove: vec![remove.clone()],
                                            });
                                            this.save_session();
                                            cx.notify();
                                        }
                                    })),
                            )
                    },
                )))
                .child(
                    div()
                        .id("tag-input")
                        .child(widgets::text_field(
                            theme,
                            &self.tag_buffer,
                            "Add tag…",
                            self.focus == super::FocusArea::TagInput,
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.focus = super::FocusArea::TagInput;
                            this.suggestion_index = 0;
                            this.tag_buffer.end(false);
                            cx.notify();
                        })),
                );
        let suggestions = self.tag_suggestions();
        if !suggestions.is_empty() {
            section = section.child(self.inspector_suggestions(suggestions, true, cx));
        }
        if has_tag.is_none() {
            section = section.opacity(0.6);
        }
        section
    }

    fn inspector_project_row(
        &self,
        project: Option<String>,
        project_id: Option<flowlist_core::ids::ProjectId>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing =
            self.editing == Some(EditField::Project) && self.focus == super::FocusArea::Field;
        let field = if editing {
            div()
                .id("project-editor")
                .child(widgets::text_field(
                    theme,
                    &self.field,
                    "Project path",
                    true,
                ))
                .into_any_element()
        } else {
            let value = project.clone().unwrap_or_else(|| "Inbox".to_string());
            div()
                .id("field:project")
                .h(px(28.))
                .px_2()
                .flex()
                .items_center()
                .justify_between()
                .rounded_md()
                .border_1()
                .border_color(theme.border_soft)
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child(
                    div()
                        .text_color(if project.is_some() {
                            theme.text
                        } else {
                            theme.text_faint
                        })
                        .child(value),
                )
                .when_some(project_id.clone(), |this, project| {
                    this.child(
                        div()
                            .id("inspector-open-project")
                            .text_xs()
                            .text_color(theme.accent)
                            .child("open")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.open_view(View::Project(project.clone()), cx);
                            })),
                    )
                })
                .on_click(cx.listener(|this, _, _, cx| this.begin_edit(EditField::Project, cx)))
                .into_any_element()
        };
        let suggestions = self.project_suggestions();
        let mut section = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Project"),
            )
            .child(field);
        if !suggestions.is_empty() {
            section = section.child(self.inspector_suggestions(suggestions, false, cx));
        }
        section
    }

    fn render_project_inspector(
        &self,
        id: &flowlist_core::ids::ProjectId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let Some(project) = db.projects.get(id) else {
            return self.render_empty_inspector().into_any_element();
        };
        let (done, total) = db.project_progress(id);
        let tags: Vec<String> = project
            .tags
            .iter()
            .filter_map(|tag| db.tags.get(tag).map(|tag| db.tag_path(&tag.id)))
            .collect();
        let status = project.status;
        let icon = project.icon;
        let attachments = project.attachments.clone();
        let project_id = id.clone();
        let rename_id = id.clone();
        let due = project.due.map(|stamp| human_date(&stamp, now()));
        let defer = project.defer.map(|stamp| human_date(&stamp, now()));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(project.name.clone()),
                    )
                    .child(self.rename_button(
                        format!("rename-project-inspector:{rename_id}"),
                        super::modals::EntityRef::Project(rename_id),
                        "inspector-project".to_string(),
                        true,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(theme.text_faint).child("Icon"))
                    .child(
                        div()
                            .id("project-icon-toggle")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(if self.project_icon_open {
                                theme.accent
                            } else {
                                theme.border_soft
                            })
                            .bg(if self.project_icon_open {
                                theme.selection
                            } else {
                                theme.clear
                            })
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, "Change the project icon")
                            .hover(|style| style.bg(theme.hover))
                            .child(widgets::icon_for_project(icon, 14.0, theme.text_dim))
                            .child(icons::icon(Icon::ChevronDown, 10.0, theme.text_faint))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.project_icon_open = !this.project_icon_open;
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
            .when(self.project_icon_open, |this| {
                this.child(div().flex().flex_wrap().items_center().gap_1().children(
                    ProjectIcon::ALL.into_iter().map(|candidate| {
                        let set_id = project_id.clone();
                        let active = icon == candidate;
                        div()
                            .id(super::sid(format!(
                                "project-inspector-icon:{}",
                                candidate.slug()
                            )))
                            .size(px(24.))
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
                            .child(widgets::icon_for_project(
                                candidate,
                                13.0,
                                if active {
                                    theme.accent
                                } else {
                                    theme.text_faint
                                },
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.execute(Command::UpdateProject {
                                    id: set_id.clone(),
                                    patch: ProjectPatch {
                                        icon: Some(candidate),
                                        ..Default::default()
                                    },
                                });
                                this.project_icon_open = false;
                                cx.notify();
                            }))
                    }),
                ))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(120.))
                            .child(widgets::progress(theme, done, total)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child(format!("{done}/{total} complete")),
                    ),
            )
            .child(
                div().flex().flex_wrap().items_center().gap_1().children(
                    [
                        ProjectStatus::Active,
                        ProjectStatus::OnHold,
                        ProjectStatus::Done,
                        ProjectStatus::Dropped,
                    ]
                    .into_iter()
                    .map(|candidate| {
                        let set_id = project_id.clone();
                        let color = match candidate {
                            ProjectStatus::Active => theme.success,
                            ProjectStatus::OnHold => theme.warning,
                            ProjectStatus::Done => theme.accent,
                            ProjectStatus::Dropped => theme.text_faint,
                        };
                        div()
                            .id(super::sid(format!("project-status:{}", candidate.label())))
                            .cursor_pointer()
                            .opacity(if candidate == status { 1.0 } else { 0.55 })
                            .hover(|style| style.opacity(1.0))
                            .child(widgets::chip(theme, candidate.label(), color))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.execute(Command::SetProjectStatus {
                                    id: set_id.clone(),
                                    status: candidate,
                                });
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child(div().text_xs().text_color(theme.text_faint).child(format!(
                "Review {} · every {} day(s)",
                project.next_review.format("%b %-d"),
                project.review_interval_days
            )))
            .when_some(defer, |this, defer| {
                this.child(self.inspector_readonly("Defer", &defer))
            })
            .when_some(due, |this, due| {
                this.child(self.inspector_readonly("Due", &due))
            })
            .when(!tags.is_empty(), |this| {
                this.child(self.inspector_readonly("Tags", &tags.join(", ")))
            })
            .child(self.inspector_attachments(&attachments, cx))
            .into_any_element()
    }

    /// Parallel/Sequential for a task's own subtasks. Same language as the
    /// project type chips so an action group reads the same everywhere.
    fn inspector_task_type(&self, sequential: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Subtasks"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("task-kind-parallel")
                            .cursor_pointer()
                            .child(widgets::toggle_chip(theme, "Parallel", !sequential))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.set_task_sequential(false, cx)),
                            ),
                    )
                    .child(
                        div()
                            .id("task-kind-sequential")
                            .cursor_pointer()
                            .child(widgets::toggle_chip(theme, "Sequential", sequential))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.set_task_sequential(true, cx)),
                            ),
                    ),
            )
    }

    fn inspector_attachments(
        &self,
        attachments: &[String],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let mut rows = div().flex().flex_col().gap_1();
        for (index, path) in attachments.iter().enumerate() {
            let name = std::path::Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            let open_path = path.clone();
            let remove_path = path.clone();
            rows = rows.child(
                div()
                    .id(super::sid(format!("attachment:{index}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(24.))
                    .px_2()
                    .rounded_md()
                    .bg(theme.raised)
                    .child(icons::icon(Icon::Book, 12.0, theme.text_faint))
                    .child(
                        div()
                            .id(super::sid(format!("attachment-name:{index}")))
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_xs()
                            .text_color(theme.text)
                            .flow_tip(self, cx, theme, path.clone())
                            .child(name),
                    )
                    .child(
                        div()
                            .id(super::sid(format!("attachment-open:{index}")))
                            .text_xs()
                            .text_color(theme.accent)
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, "Open this file")
                            .child("open")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_attachment(open_path.clone());
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id(super::sid(format!("attachment-remove:{index}")))
                            .text_xs()
                            .text_color(theme.text_faint)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.danger))
                            .flow_tip(self, cx, theme, "Remove this attachment")
                            .child("×")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_attachment(remove_path.clone(), cx)
                            })),
                    ),
            );
        }
        let field = div()
            .id("field:attachments")
            .h(px(26.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .rounded_md()
            .border_1()
            .border_color(theme.border_soft)
            .text_xs()
            .text_color(theme.text_dim)
            .cursor_pointer()
            .flow_tip(self, cx, theme, "Choose files with the macOS file picker")
            .hover(|style| style.bg(theme.hover))
            .child(icons::icon(Icon::Paperclip, 12.0, theme.text_faint))
            .child(if attachments.is_empty() {
                "Attach file…"
            } else {
                "Attach another file…"
            })
            .on_click(cx.listener(|this, _, _, cx| this.pick_attachments(cx)))
            .into_any_element();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("Attachments"),
            )
            .when(!attachments.is_empty(), |this| this.child(rows))
            .child(field)
    }

    fn inspector_readonly(&self, label: &'static str, value: &str) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(theme.text_faint).child(label))
            .child(
                div()
                    .p_2()
                    .rounded_md()
                    .bg(theme.raised)
                    .text_sm()
                    .text_color(theme.text_dim)
                    .child(value.to_string()),
            )
    }
}
