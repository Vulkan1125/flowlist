//! The right-hand inspector: edit one task (or project) without leaving the
//! list. Every field commits through the same command layer as the CLI.

use chrono::{Datelike, Duration, Timelike};
use gpui::{AnyElement, Context, CursorStyle, IntoElement, Window, div, prelude::*, px};

use flowlist_core::command::{Command, ProjectPatch};
use flowlist_core::ids::{ChecklistId, TaskId};
use flowlist_core::model::{Priority, ProjectIcon, ProjectStatus};
use flowlist_core::time::{format_duration, human_date, local_date, now};

use super::content::month_grid;
use super::drag::{DragChecklist, DragTask, checklist_ghost};
use super::icons::Icon;
use super::widgets::TipExt;
use super::{ClockUnit, DateField, DatePickerStep, EditField, FlowlistApp, View, icons, widgets};

impl FlowlistApp {
    pub(super) fn render_inspector(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let mut panel = div()
            .flex()
            .flex_col()
            .w(px(super::INSPECTOR_WIDTH))
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
            self.render_task_inspector(&id, window, cx)
                .into_any_element()
        } else if let View::Project(id) = self.view.clone() {
            self.render_project_inspector(&id, window, cx)
                .into_any_element()
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
            .gap_3()
            .p_6()
            .text_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(54.))
                    .rounded_full()
                    .bg(theme.field)
                    .border_1()
                    .border_color(theme.border_soft)
                    .child(icons::icon(Icon::Info, 22.0, theme.text_faint)),
            )
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

    fn render_task_inspector(
        &self,
        id: &TaskId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
            .child(self.inspector_task_type(&id, sequential, cx))
            .child(self.inspector_attachments(
                &attachments,
                super::modals::AttachmentTarget::Task(id.clone()),
                window,
                cx,
            ))
            .child(self.inspector_tag_section(&tags, cx))
            .child(self.inspector_project_row(project, project_id, cx))
            .child(self.inspector_date_field(DateField::Defer, &defer.unwrap_or_default(), cx))
            .child(self.inspector_date_field(DateField::Due, &due.unwrap_or_default(), cx))
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
                    move |this, _, _, cx| this.choose_task_priority(id.clone(), None, cx)
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
                        move |this, _, _, cx| {
                            this.choose_task_priority(id.clone(), Some(priority), cx)
                        }
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

    fn field_text_hit(&mut self, hit: &widgets::TextHit, cx: &mut Context<Self>) {
        self.focus = super::FocusArea::Field;
        apply_text_hit(&mut self.field, hit);
        cx.notify();
    }

    fn tag_text_hit(&mut self, hit: &widgets::TextHit, cx: &mut Context<Self>) {
        self.focus = super::FocusArea::TagInput;
        self.suggestion_engaged = false;
        apply_text_hit(&mut self.tag_buffer, hit);
        cx.notify();
    }

    fn checklist_text_hit(&mut self, hit: &widgets::TextHit, cx: &mut Context<Self>) {
        self.focus = super::FocusArea::Checklist;
        apply_text_hit(&mut self.checklist_buffer, hit);
        cx.notify();
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
        let is_title = field == EditField::Title;
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
                let editor = if is_title {
                    // The read-only title clamps to two lines; editing must
                    // wrap too, or a long title puts the caret outside the
                    // single-line field and it disappears.
                    widgets::text_area_with_size_and_cursor(
                        theme,
                        &self.field,
                        placeholder,
                        true,
                        56.0,
                        14.0,
                        20.0,
                        cx.listener(|this, hit: &widgets::TextHit, _, cx| {
                            this.field_text_hit(hit, cx)
                        }),
                    )
                    .into_any_element()
                } else {
                    widgets::text_field_with_cursor(
                        theme,
                        &self.field,
                        placeholder,
                        true,
                        cx.listener(|this, hit: &widgets::TextHit, _, cx| {
                            this.field_text_hit(hit, cx)
                        }),
                    )
                    .into_any_element()
                };
                div()
                    .on_children_prepainted(self.field_editor_probe())
                    .id("field-editor")
                    .child(editor)
                    .into_any_element()
            } else {
                let field_box = div()
                    .id(super::sid(format!(
                        "field:{}",
                        label.to_lowercase().replace(' ', "-")
                    )))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border)
                    // A readonly field is still a box: give it a surface so it
                    // reads as one solid object on the glass, and a border
                    // strong enough that its rounded corners stay visible.
                    .bg(theme.field)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .text_color(if value.is_empty() {
                        theme.text_faint
                    } else {
                        theme.text
                    })
                    .child(
                        div()
                            .when(is_title, |this| this.line_clamp(2))
                            .child(if value.is_empty() {
                                placeholder.to_string()
                            } else {
                                value.to_string()
                            }),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.begin_edit(field, cx)));
                // A task title gets two lines in the inspector instead of
                // running off the card; every other field stays one line tall.
                if is_title {
                    field_box.min_h(px(28.)).py_1().into_any_element()
                } else {
                    field_box.h(px(28.)).into_any_element()
                }
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

    /// A Due or Defer field. The value still opens the text editor; the
    /// calendar icon on the right opens a small month picker instead.
    fn inspector_date_field(
        &self,
        field: DateField,
        value: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing =
            self.editing == Some(field.edit_field()) && self.focus == super::FocusArea::Field;
        let open = self.date_picker == Some(field);
        let anchor = self.date_anchor(field);
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(field.label()),
            )
            .child(if editing {
                div()
                    .id("field-editor")
                    .child(widgets::text_field_with_cursor(
                        theme,
                        &self.field,
                        field.placeholder(),
                        true,
                        cx.listener(|this, hit: &widgets::TextHit, _, cx| {
                            this.field_text_hit(hit, cx)
                        }),
                    ))
                    .into_any_element()
            } else {
                div()
                    .id(super::sid(format!("field:{}", field.slug())))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .rounded_lg()
                    .border_1()
                    .border_color(if open { theme.accent } else { theme.border })
                    .bg(theme.field)
                    .child(
                        div()
                            .id(super::sid(format!("date-value:{}", field.slug())))
                            .flex_1()
                            .h_full()
                            .flex()
                            .items_center()
                            .px_2()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .text_color(if value.is_empty() {
                                theme.text_faint
                            } else {
                                theme.text
                            })
                            .child(if value.is_empty() {
                                field.placeholder().to_string()
                            } else {
                                value.to_string()
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.begin_edit(field.edit_field(), cx)
                            })),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .child(
                                div()
                                    .id(super::sid(format!("date-picker:{}", field.slug())))
                                    .h(px(28.))
                                    .w(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.hover))
                                    .flow_tip(
                                        self,
                                        cx,
                                        theme,
                                        format!("Pick a {} date", field.label().to_lowercase()),
                                    )
                                    .child(icons::icon(
                                        Icon::Calendar,
                                        14.0,
                                        if open { theme.accent } else { theme.text_faint },
                                    ))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_date_picker(field, cx)
                                    })),
                            )
                            .on_children_prepainted(move |bounds, _, _| {
                                if let Some(bounds) = bounds.first() {
                                    anchor.set(Some(*bounds));
                                }
                            }),
                    )
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

    /// Record the open field editor's painted bounds for the GUI probe.
    fn field_editor_probe(
        &self,
    ) -> impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let anchor = self.field_editor_anchor.clone();
        move |bounds, _, _| {
            if let Some(bounds) = bounds.first() {
                anchor.set(Some(*bounds));
            }
        }
    }

    /// Record one control's painted bounds in the date picker's probe map.
    /// The map outlives a frame because boxes fill in during paint, after the
    /// test state for that frame has already been written.
    fn probe_box(
        &self,
        key: String,
    ) -> impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.date_picker_boxes.clone();
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

    /// Wrap a control so its painted bounds land in the probe map. The click
    /// itself stays on the wrapper's stateful parent.
    fn probed(&self, key: &'static str, child: impl IntoElement) -> impl IntoElement {
        div()
            .on_children_prepainted(self.probe_box(key.to_string()))
            .child(child)
    }

    /// The date popover. It is one card with two steps: a month calendar, then
    /// a clock face when the chosen day carries a time.
    pub(super) fn render_date_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        if self.date_picker.is_none() {
            return div().into_any_element();
        }
        let body = match self.date_picker_step {
            DatePickerStep::Calendar => self.render_date_calendar(cx),
            DatePickerStep::Clock => self.render_time_clock(cx),
        };
        div()
            .flex()
            .flex_col()
            .p_3()
            .w(px(super::DATE_PICKER_WIDTH))
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg()
            .child(body)
            .into_any_element()
    }

    /// Step one: one month, Monday-first, today marked, the stored date filled
    /// in and the keyboard cursor outlined. Picking a day with a time selected
    /// moves on to the clock instead of closing.
    fn render_date_calendar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(field) = self.date_picker else {
            return div().into_any_element();
        };
        let today = local_date(now());
        let month = self.date_picker_month;
        let cursor = self.date_picker_cursor;
        let stored = self.date_picker_value(field);
        let selected = stored.map(|stamp| stamp.local_date());
        let timed = self.date_picker_time.is_some();
        let (grid_start, _, weeks) = month_grid(month);

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(month.format("%B %Y").to_string()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("date-picker-prev")
                            .flow_tip(self, cx, theme, "Previous month")
                            .child(widgets::button(theme, "‹", false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.shift_date_picker_month(-1);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("date-picker-next")
                            .flow_tip(self, cx, theme, "Next month")
                            .child(widgets::button(theme, "›", false))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.shift_date_picker_month(1);
                                cx.notify();
                            })),
                    ),
            );

        let weekdays = div()
            .flex()
            .gap(px(2.))
            .children(["M", "T", "W", "T", "F", "S", "S"].map(|label| {
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(label)
            }));

        let mut grid = div().flex().flex_col().gap(px(2.));
        for week in 0..weeks {
            let mut row = div().flex().gap(px(2.));
            for weekday in 0..7 {
                let date = grid_start + Duration::days((week * 7 + weekday) as i64);
                let in_month = date.month() == month.month() && date.year() == month.year();
                let is_today = date == today;
                let is_selected = selected == Some(date);
                let is_cursor = cursor == date;
                let mut cell = div()
                    .id(super::sid(format!("date-picker-day:{date}")))
                    .flex_1()
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_xs()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(
                        div()
                            .on_children_prepainted(self.probe_box(format!("day:{date}")))
                            .child(date.day().to_string()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.choose_date_picker_date(date, cx);
                    }));
                if is_selected {
                    cell = cell
                        .bg(theme.accent)
                        .text_color(theme.bg)
                        .font_weight(gpui::FontWeight::SEMIBOLD);
                } else if is_today {
                    cell = cell.bg(theme.accent_soft).text_color(theme.accent);
                } else {
                    cell = cell.text_color(if in_month { theme.text } else { theme.text_dim });
                }
                if is_cursor {
                    cell = cell.border_1().border_color(theme.accent);
                }
                if !in_month {
                    cell = cell.opacity(0.45);
                }
                row = row.child(cell);
            }
            grid = grid.child(row);
        }

        let mode =
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_xs().text_color(theme.text_faint).child("Time"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .id("date-picker-all-day")
                                .flow_tip(self, cx, theme, "Keep the day all-day")
                                .child(self.probed(
                                    "all-day",
                                    widgets::toggle_chip(theme, "All day", !timed),
                                ))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_date_picker_all_day(true, cx)
                                })),
                        )
                        .child(
                            div()
                                .id("date-picker-at-time")
                                .flow_tip(self, cx, theme, "Set a time on the clock")
                                .child(self.probed(
                                    "at-time",
                                    widgets::toggle_chip(theme, "At time", timed),
                                ))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_date_picker_all_day(false, cx)
                                })),
                        ),
                );

        let footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("date-picker-today")
                            .flow_tip(self, cx, theme, "Today (t)")
                            .child(self.probed("today", widgets::button(theme, "Today", false)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.choose_date_picker_date(local_date(now()), cx);
                            })),
                    )
                    .child(
                        div()
                            .id("date-picker-clear")
                            .flow_tip(self, cx, theme, "Clear the date (⌫)")
                            .child(self.probed("clear", widgets::button(theme, "Clear", false)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.clear_date_picker(cx);
                            })),
                    ),
            )
            .child(div().text_xs().text_color(theme.text_faint).child("⏎ pick"));

        let hint = if timed {
            "←↑↓→ day · ⇞⇟ month · a all-day · ⏎ clock"
        } else {
            "←↑↓→ day · ⇞⇟ month · a time · ⏎ pick"
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(weekdays)
            .child(grid)
            .child(widgets::divider(theme))
            .child(mode)
            .child(footer)
            .child(div().text_xs().text_color(theme.text_faint).child(hint))
            .into_any_element()
    }

    /// Step two: an Android-style clock face. The hour comes first; choosing
    /// one advances to minutes. The dial is a square with the numbers placed
    /// on a circle and a hand drawn from the centre to the active number.
    fn render_time_clock(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let Some(base) = self.date_picker_clock_base() else {
            return div().into_any_element();
        };
        let hour12 = super::clock_hour_12(base.hour());
        let minute = base.minute();
        let pm = base.hour() >= 12;
        let unit = self.date_picker_unit;
        let cursor = self.date_picker_cursor;

        let hour_color = if unit == ClockUnit::Hour {
            theme.accent
        } else {
            theme.text_dim
        };
        let minute_color = if unit == ClockUnit::Minute {
            theme.accent
        } else {
            theme.text_dim
        };
        let display =
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap_1()
                .child(
                    div()
                        .id("clock-hour")
                        .px_2()
                        .rounded_md()
                        .cursor_pointer()
                        .text_3xl()
                        .text_color(hour_color)
                        .hover(|style| style.bg(theme.hover))
                        .child(format!("{hour12:02}"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_date_picker_unit(ClockUnit::Hour, cx)
                        })),
                )
                .child(div().text_3xl().text_color(theme.text_faint).child(":"))
                .child(
                    div()
                        .id("clock-minute")
                        .px_2()
                        .rounded_md()
                        .cursor_pointer()
                        .text_3xl()
                        .text_color(minute_color)
                        .hover(|style| style.bg(theme.hover))
                        .child(format!("{minute:02}"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_date_picker_unit(ClockUnit::Minute, cx)
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .ml_2()
                        .child(
                            div()
                                .id("clock-am")
                                .flow_tip(self, cx, theme, "Morning")
                                .child(widgets::toggle_chip(theme, "AM", !pm))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_date_picker_half(false, cx)
                                })),
                        )
                        .child(
                            div()
                                .id("clock-pm")
                                .flow_tip(self, cx, theme, "Afternoon and evening")
                                .child(widgets::toggle_chip(theme, "PM", pm))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_date_picker_half(true, cx)
                                })),
                        ),
                );

        let dial_size = 236.0;
        let center = dial_size / 2.0;
        let radius = 92.0;
        let cell = 36.0;
        let active = match unit {
            ClockUnit::Hour => (hour12 % 12) as usize,
            ClockUnit::Minute => ((minute as f32 / 5.0).round() as usize) % 12,
        };
        let (tx, ty) = super::clock_dial_position(active, center, radius);
        let hand_color = theme.accent;
        let hand = gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let origin = bounds.center();
                let end = gpui::point(bounds.origin.x + px(tx), bounds.origin.y + px(ty));
                // A short tail on the far side, the way the Android dial reads.
                let tail = gpui::point(
                    bounds.origin.x + px(center - (tx - center) * 0.16),
                    bounds.origin.y + px(center - (ty - center) * 0.16),
                );
                let mut path = gpui::PathBuilder::stroke(px(2.));
                path.move_to(origin);
                path.line_to(end);
                path.move_to(origin);
                path.line_to(tail);
                if let Ok(path) = path.build() {
                    window.paint_path(path, hand_color);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size(px(dial_size));

        let mut dial = div()
            .relative()
            .w(px(dial_size))
            .h(px(dial_size))
            .child(hand)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size(px(dial_size))
                    .rounded_full()
                    .border_1()
                    .border_color(theme.border_soft),
            );
        for index in 0..12usize {
            let (x, y) = super::clock_dial_position(index, center, radius);
            let (label, key) = match unit {
                ClockUnit::Hour => {
                    let hour = if index == 0 { 12 } else { index };
                    (format!("{hour}"), format!("hour:{hour}"))
                }
                ClockUnit::Minute => {
                    let minute = index * 5;
                    (format!("{minute:02}"), format!("minute:{minute}"))
                }
            };
            let hour_value = if index == 0 { 12 } else { index as u32 };
            let minute_value = (index * 5) as u32;
            let mut number = div()
                .id(super::sid(format!("date-picker-{key}")))
                .absolute()
                .left(px(x - cell / 2.0))
                .top(px(y - cell / 2.0))
                .size(px(cell))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .text_sm()
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child(
                    div()
                        .on_children_prepainted(self.probe_box(key))
                        .child(label),
                )
                .on_click(cx.listener(move |this, _, _, cx| match unit {
                    ClockUnit::Hour => this.set_date_picker_hour(hour_value, cx),
                    ClockUnit::Minute => this.set_date_picker_minute(minute_value, cx),
                }));
            if index == active {
                number = number
                    .bg(theme.accent)
                    .text_color(theme.bg)
                    .font_weight(gpui::FontWeight::SEMIBOLD);
            } else {
                number = number.text_color(theme.text);
            }
            dial = dial.child(number);
        }

        let footer =
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .id("date-picker-back")
                                .flow_tip(self, cx, theme, "Back to the calendar (esc)")
                                .child(self.probed("back", widgets::button(theme, "Back", false)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.date_picker_back_to_calendar(cx)
                                })),
                        )
                        .child(
                            div()
                                .id("date-picker-clock-all-day")
                                .flow_tip(self, cx, theme, "Clear the time (a)")
                                .child(self.probed(
                                    "clock-all-day",
                                    widgets::toggle_chip(theme, "All day", false),
                                ))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_date_picker_all_day(true, cx)
                                })),
                        ),
                )
                .child(
                    div()
                        .id("date-picker-done")
                        .flow_tip(self, cx, theme, "Set the date and time (⏎)")
                        .child(self.probed("done", widgets::button(theme, "Done", true)))
                        .on_click(cx.listener(|this, _, _, cx| {
                            let date = this.date_picker_cursor;
                            this.apply_date_picker(date, cx);
                        })),
                );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(cursor.format("%a, %b %-d").to_string()),
            )
            .child(display)
            .child(div().flex().justify_center().child(dial))
            .child(footer)
            .child(
                div()
                    .flex()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("←↑↓→ set · ⇥ hour/min · ⏎ done"),
            )
            .into_any_element()
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
            row = row.child(widgets::text_area_with_cursor(
                theme,
                &self.field,
                "Add notes",
                true,
                140.0,
                cx.listener(|this, hit: &widgets::TextHit, _, cx| this.field_text_hit(hit, cx)),
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
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.field)
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
                        .child(widgets::text_field_with_cursor(
                            theme,
                            &self.tag_buffer,
                            "Add tag…",
                            self.focus == super::FocusArea::TagInput,
                            cx.listener(|this, hit: &widgets::TextHit, _, cx| {
                                this.tag_text_hit(hit, cx)
                            }),
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.focus = super::FocusArea::TagInput;
                            this.suggestion_index = 0;
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
                .child(widgets::text_field_with_cursor(
                    theme,
                    &self.field,
                    "Project path",
                    true,
                    cx.listener(|this, hit: &widgets::TextHit, _, cx| {
                        this.field_text_hit(hit, cx)
                    }),
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
                .rounded_lg()
                .border_1()
                .border_color(theme.border)
                .bg(theme.field)
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
        window: &mut Window,
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
                            .min_w_0()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(project.name.clone()),
                    )
                    .child(self.rename_button(
                        format!("rename-project-inspector:{rename_id}"),
                        super::modals::EntityRef::Project(rename_id),
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
                                theme.border
                            })
                            .bg(if self.project_icon_open {
                                theme.selection
                            } else {
                                theme.field
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
                            .border_color(if active { theme.accent } else { theme.border })
                            .bg(if active { theme.selection } else { theme.field })
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
            .child({
                let edit_id = id.clone();
                div()
                    .id("project-review-interval")
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme.text))
                    .flow_tip(self, cx, theme, "Change how often this project is reviewed")
                    .child(format!(
                        "Review {} · every {} day(s)",
                        project.next_review.format("%b %-d"),
                        project.review_interval_days
                    ))
                    .child(icons::icon(Icon::ChevronRight, 10.0, theme.text_faint))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let modal = super::modals::Modal::review_interval(this, edit_id.clone());
                        this.open_modal(modal, cx);
                    }))
            })
            .when_some(defer, |this, defer| {
                this.child(self.inspector_readonly("Defer", &defer))
            })
            .when_some(due, |this, due| {
                this.child(self.inspector_readonly("Due", &due))
            })
            .when(!tags.is_empty(), |this| {
                this.child(self.inspector_readonly("Tags", &tags.join(", ")))
            })
            .child(self.inspector_attachments(
                &attachments,
                super::modals::AttachmentTarget::Project(id.clone()),
                window,
                cx,
            ))
            .into_any_element()
    }

    /// Subtasks as an inline checklist-style list: tick, click to select,
    /// drag to reorder, or type in the field at the bottom to add one.
    fn inspector_task_type(
        &self,
        id: &TaskId,
        sequential: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let parent = id.clone();
        let db = self.store.db();
        let children: Vec<(TaskId, String, bool, Option<Priority>)> = db
            .tasks
            .get(id)
            .map(|task| {
                task.child_order
                    .iter()
                    .filter_map(|child_id| {
                        db.tasks
                            .get(child_id)
                            .filter(|child| !child.is_trashed())
                            .map(|child| {
                                (
                                    child.id.clone(),
                                    child.title.clone(),
                                    child.completed,
                                    child.priority,
                                )
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let one_off = db.tasks.get(id).is_some_and(|task| task.steps_one_off);
        let checklist: Vec<(ChecklistId, String, bool)> = db
            .tasks
            .get(id)
            .map(|task| {
                task.checklist
                    .iter()
                    .map(|item| (item.id.clone(), item.title.clone(), item.completed))
                    .collect()
            })
            .unwrap_or_default();
        // Both lists are always visible. The mode only changes the rules and
        // what the add field creates.
        let done = children.iter().filter(|child| child.2).count()
            + checklist.iter().filter(|item| item.2).count();
        let total = children.len() + checklist.len();

        let mut section = div().flex().flex_col().gap_1().child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .child("Subtasks"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if total > 0 && done == total {
                            theme.success
                        } else {
                            theme.text_faint
                        })
                        .child(format!("{done}/{total}")),
                ),
        );

        let step_mode = if one_off {
            super::TaskStepMode::OneOff
        } else if sequential {
            super::TaskStepMode::Sequential
        } else {
            super::TaskStepMode::Parallel
        };
        let mode_id = parent.clone();
        let mode_boxes = self.inspector_boxes.clone();
        let probe = |key: &'static str| {
            let boxes = mode_boxes.clone();
            move |bounds: Vec<gpui::Bounds<gpui::Pixels>>, _: &mut Window, _: &mut gpui::App| {
                if let Some(bounds) = bounds.first() {
                    boxes.borrow_mut().insert(
                        key.to_string(),
                        [
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        ],
                    );
                }
            }
        };
        section = section.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .on_children_prepainted(probe("step:parallel"))
                        .id("task-kind-parallel")
                        .cursor_pointer()
                        .child(widgets::toggle_chip(
                            theme,
                            "Parallel",
                            step_mode == super::TaskStepMode::Parallel,
                        ))
                        .on_click(cx.listener({
                            let mode_id = mode_id.clone();
                            move |this, _, _, cx| {
                                this.set_task_step_mode(
                                    mode_id.clone(),
                                    super::TaskStepMode::Parallel,
                                    cx,
                                )
                            }
                        })),
                )
                .child(
                    div()
                        .on_children_prepainted(probe("step:sequential"))
                        .id("task-kind-sequential")
                        .cursor_pointer()
                        .child(widgets::toggle_chip(
                            theme,
                            "Sequential",
                            step_mode == super::TaskStepMode::Sequential,
                        ))
                        .on_click(cx.listener({
                            let mode_id = mode_id.clone();
                            move |this, _, _, cx| {
                                this.set_task_step_mode(
                                    mode_id.clone(),
                                    super::TaskStepMode::Sequential,
                                    cx,
                                )
                            }
                        })),
                )
                .child(
                    div()
                        .on_children_prepainted(probe("step:one-off"))
                        .id("task-kind-one-off")
                        .cursor_pointer()
                        .child(widgets::toggle_chip(
                            theme,
                            "One-off",
                            step_mode == super::TaskStepMode::OneOff,
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_task_step_mode(
                                mode_id.clone(),
                                super::TaskStepMode::OneOff,
                                cx,
                            )
                        })),
                ),
        );

        // Give the mode chips room to breathe above the subtask list.
        section = section.child(div().h(px(6.)));

        for (child_id, title, completed, priority) in children {
            let select_id = child_id.clone();
            let toggle_id = child_id.clone();
            let separate_id = child_id.clone();
            let trash_id = child_id.clone();
            let drag_id = child_id.clone();
            let drag_title = title.clone();
            let drop_parent = parent.clone();
            let drop_target = child_id.clone();
            let box_id = format!("subtask:{child_id}");
            let boxes = self.inspector_boxes.clone();
            let row = div()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        boxes.borrow_mut().insert(
                            box_id.clone(),
                            [
                                f32::from(bounds.origin.x),
                                f32::from(bounds.origin.y),
                                f32::from(bounds.size.width),
                                f32::from(bounds.size.height),
                            ],
                        );
                    }
                })
                .id(super::sid(format!("subtask:{child_id}")))
                .flex()
                .items_center()
                .gap_2()
                .min_h(px(26.))
                .px_1()
                .rounded_sm()
                .hover(|style| style.bg(theme.hover))
                .child(
                    div()
                        .id(super::sid(format!("subtask-toggle:{child_id}")))
                        .flex_shrink_0()
                        .flow_tip(
                            self,
                            cx,
                            theme,
                            if completed {
                                "Mark subtask incomplete"
                            } else {
                                "Mark subtask complete"
                            },
                        )
                        .child(widgets::checkbox(
                            theme,
                            completed,
                            theme.accent,
                            priority.map(|priority| {
                                super::theme::priority_color(theme, priority)
                            }),
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_complete(toggle_id.clone(), cx);
                        })),
                )
                .child(
                    div()
                        .id(super::sid(format!("subtask-title:{child_id}")))
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_sm()
                        .text_color(if completed {
                            theme.text_faint
                        } else {
                            theme.text
                        })
                        .when(completed, |this| this.line_through())
                        .child(title)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_only(select_id.clone());
                            this.show_inspector = true;
                            cx.notify();
                        })),
                )
                .when(!one_off, |this| {
                    this.child(
                        div()
                            .id(super::sid(format!("separate-subtask:{child_id}")))
                            .text_xs()
                            .text_color(theme.text_faint)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .flow_tip(
                                self,
                                cx,
                                theme,
                                "Move this task out from under its parent",
                            )
                            .child("Separate")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.promote_subtask(separate_id.clone(), cx);
                            })),
                    )
                })
                .child(
                    div()
                        .id(super::sid(format!("delete-subtask:{child_id}")))
                        .ml(px(10.))
                        .w(px(18.))
                        .text_center()
                        .text_xs()
                        .text_color(theme.text_faint)
                        .cursor_pointer()
                        .hover(|style| style.text_color(theme.danger))
                        .flow_tip(self, cx, theme, "Move subtask to Trash")
                        .child(icons::icon(Icon::Trash, 12.0, theme.text_faint))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.trash_subtask(trash_id.clone(), cx);
                        })),
                )
                .child(
                    div()
                        .id(super::sid(format!("subtask-grip:{child_id}")))
                        .w(px(22.))
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::OpenHand)
                        .text_xs()
                        .text_color(theme.text_faint)
                        .flow_tip(self, cx, theme, "Drag to reorder")
                        .child("⠿"),
                )
                .on_drag(
                    DragTask::task(drag_id, drag_title),
                    move |payload, position, window, cx| {
                        super::drag::ghost(payload, position, window, cx, theme)
                    },
                )
                .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                    this.drop_subtask_beside(
                        drop_parent.clone(),
                        drop_target.clone(),
                        payload,
                        cx,
                    )
                }));
            section = section.child(row);
        }

        if total == 0 {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No subtasks yet."),
            );
        }

        for (item_id, title, completed) in &checklist {
                let toggle_task = parent.clone();
                let toggle_item = item_id.clone();
                let edit_task = parent.clone();
                let edit_item = item_id.clone();
                let edit_title = title.clone();
                let delete_task = parent.clone();
                let delete_item = item_id.clone();
                let drag_task = parent.clone();
                let drag_item = item_id.clone();
                let drag_title = title.clone();
                let drop_task = parent.clone();
                let drop_item = item_id.clone();
                section = section.child(
                    div()
                        .id(super::sid(format!("one-off:{item_id}")))
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_h(px(26.))
                        .px_1()
                        .rounded_sm()
                        .hover(|style| style.bg(theme.hover))
                        .child(
                            div()
                                .id(super::sid(format!("one-off-toggle:{item_id}")))
                                .flex_shrink_0()
                                .flow_tip(
                                    self,
                                    cx,
                                    theme,
                                    if *completed {
                                        "Mark one-off incomplete"
                                    } else {
                                        "Mark one-off complete"
                                    },
                                )
                                .child(widgets::checkbox(
                                    theme,
                                    *completed,
                                    theme.accent,
                                    None,
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.toggle_checklist_item(
                                        toggle_task.clone(),
                                        toggle_item.clone(),
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            div()
                                .id(super::sid(format!("one-off-title:{item_id}")))
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
                                    this.begin_checklist_edit(
                                        edit_item.clone(),
                                        edit_title.clone(),
                                        cx,
                                    );
                                    this.select_only(edit_task.clone());
                                    this.show_inspector = true;
                                    cx.notify();
                                })),
                        )
                        .child(
                            div()
                                .id(super::sid(format!("one-off-delete:{item_id}")))
                                .w(px(16.))
                                .text_center()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.danger))
                                .flow_tip(self, cx, theme, "Delete one-off")
                                .child(icons::icon(Icon::Close, 12.0, theme.text_faint))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.delete_checklist_item(
                                        delete_task.clone(),
                                        delete_item.clone(),
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            div()
                                .id(super::sid(format!("one-off-grip:{item_id}")))
                                .w(px(22.))
                                .h(px(22.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor(CursorStyle::OpenHand)
                                .text_xs()
                                .text_color(theme.text_faint)
                                .flow_tip(self, cx, theme, "Drag to reorder")
                                .child("⠿"),
                        )
                        .on_drag(
                            DragChecklist {
                                task: drag_task,
                                item: drag_item,
                                label: drag_title,
                            },
                            move |payload, position, window, cx| {
                                checklist_ghost(payload, position, window, cx, theme)
                            },
                        )
                        .on_drop::<DragChecklist>(cx.listener(
                            move |this, payload: &DragChecklist, _, cx| {
                                this.drop_checklist_item(
                                    drop_task.clone(),
                                    drop_item.clone(),
                                    payload,
                                    cx,
                                )
                            },
                        )),
                );
            }

        // And separate the last subtask from the add field.
        section = section.child(div().h(px(6.)));

        let add_anchor = self.checklist_add_anchor.clone();
        section.child(
            div()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        add_anchor.set(Some(*bounds));
                    }
                })
                .id("subtask-add")
                .child(widgets::text_field_with_cursor(
                    theme,
                    &self.checklist_buffer,
                    if one_off {
                        "Add a one-off…"
                    } else {
                        "Add a subtask…"
                    },
                    self.focus == super::FocusArea::Checklist,
                    cx.listener(|this, hit: &widgets::TextHit, _, cx| {
                        this.checklist_text_hit(hit, cx)
                    }),
                ))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.focus = super::FocusArea::Checklist;
                    cx.notify();
                })),
        )
    }

    /// A link's favicon once it has loaded. Fetching starts on first sight of
    /// the host and the chain icon stands in until the bytes arrive.
    fn favicon(&self, path: &str, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let host = super::favicon::host(flowlist_core::model::attachment::attachment_url(path))?;
        if let Some(image) = self.favicons.borrow().get(&host).cloned() {
            return Some(
                gpui::img(gpui::ImageSource::Render(image))
                    .w(px(14.))
                    .h(px(14.))
                    .rounded_sm()
                    .into_any_element(),
            );
        }
        if self.favicon_misses.borrow().contains(&host)
            || self.favicon_requests.borrow().contains(&host)
        {
            return None;
        }
        self.favicon_requests.borrow_mut().insert(host.clone());
        let fetch_host = host.clone();
        cx.spawn(async move |this, cx| {
            let bytes = cx
                .background_executor()
                .spawn(async move { super::favicon::fetch(&fetch_host) })
                .await;
            let image = bytes.and_then(|bytes| super::favicon::decode(&bytes));
            this.update(cx, |this, cx| {
                this.favicon_requests.borrow_mut().remove(&host);
                match image {
                    Some(image) => {
                        this.favicons.borrow_mut().insert(
                            host.clone(),
                            std::sync::Arc::new(image),
                        );
                    }
                    None => {
                        this.favicon_misses.borrow_mut().insert(host.clone());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        None
    }

    fn inspector_attachments(
        &self,
        attachments: &[String],
        target: super::modals::AttachmentTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let mut rows = div().flex().flex_col().gap_1();
        for (index, path) in attachments.iter().enumerate() {
            let link = flowlist_core::model::attachment::is_web_link(path);
            let name = flowlist_core::model::attachment::attachment_name(path);
            // The name shares its row with the open and remove controls. The
            // inspector is a fixed 320 wide with 12 of padding and an 8 inset
            // on the row, so 204 points is what the text can claim.
            let name = widgets::fit_head(
                &name,
                204.0,
                12.0,
                gpui::FontWeight::NORMAL,
                theme.text,
                window,
            );
            let open_path = path.clone();
            let remove_path = path.clone();
            let preview_path = path.clone();
            let can_preview = super::preview::previewable(path);
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
                    .child(if link {
                        self.favicon(path, cx).unwrap_or_else(|| {
                            icons::icon(Icon::Link, 12.0, theme.text_faint).into_any_element()
                        })
                    } else {
                        icons::icon(Icon::Book, 12.0, theme.text_faint).into_any_element()
                    })
                    .child(
                        div()
                            .id(super::sid(format!("attachment-name:{index}")))
                            .flex_1()
                            .min_w_0()
                            // A pasted URL or deep path is one long token: it
                            // must end in an ellipsis, never wrap out of the row.
                            .truncate()
                            .text_xs()
                            .text_color(theme.text)
                            .flow_tip(
                                self,
                                cx,
                                theme,
                                if can_preview {
                                    format!("Preview {path}")
                                } else if link {
                                    path.clone()
                                } else {
                                    format!("Open {path}")
                                },
                            )
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.accent))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if can_preview {
                                    this.preview_attachment(preview_path.clone(), cx);
                                } else {
                                    this.open_attachment(preview_path.clone());
                                    cx.notify();
                                }
                            }))
                            .child(name),
                    )
                    .when(link, |this| {
                        let rename_target = target.clone();
                        let rename_path = path.clone();
                        this.child(
                            div()
                                .id(super::sid(format!("attachment-rename:{index}")))
                                .flex_shrink_0()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .cursor_pointer()
                                .hover(|style| style.text_color(theme.text))
                                .flow_tip(self, cx, theme, "Rename this link")
                                .child("rename")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let modal = super::modals::Modal::rename_link(
                                        rename_target.clone(),
                                        &rename_path,
                                    );
                                    this.open_modal(modal, cx);
                                })),
                        )
                    })
                    .child(
                        div()
                            .id(super::sid(format!("attachment-open:{index}")))
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(theme.accent)
                            .cursor_pointer()
                            .flow_tip(
                                self,
                                cx,
                                theme,
                                if link {
                                    "Open this link in the browser"
                                } else {
                                    "Open this file"
                                },
                            )
                            .child("open")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_attachment(open_path.clone());
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id(super::sid(format!("attachment-remove:{index}")))
                            .flex_shrink_0()
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
        let file_button = div()
            .id("attach-file")
            .h(px(26.))
            .px_2()
            .flex()
            .items_center()
            .justify_center()
            .gap_1()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field)
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
            .on_click(cx.listener(|this, _, _, cx| this.pick_attachments(cx)));
        let link_button = div()
            .id("attach-link")
            .h(px(26.))
            .px_2()
            .flex()
            .items_center()
            .justify_center()
            .gap_1()
            .flex_shrink_0()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field)
            .text_xs()
            .text_color(theme.text_dim)
            .cursor_pointer()
            .flow_tip(self, cx, theme, "Attach a website link")
            .hover(|style| style.bg(theme.hover))
            .child(icons::icon(Icon::Link, 12.0, theme.text_faint))
            .child("Add link…")
            .on_click(cx.listener(|this, _, _, cx| this.attach_link(cx)));
        let field = div()
            .id("field:attachments")
            .flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .on_children_prepainted(self.view_probe("attach-file".to_string()))
                    .child(file_button),
            )
            .child(
                div()
                    .on_children_prepainted(self.view_probe("attach-link".to_string()))
                    .child(link_button),
            )
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

/// Turn a mouse hit into a caret move, an extended selection, or the word /
/// whole-text selection a double- or triple-click asks for.
fn apply_text_hit(buffer: &mut super::text::TextBuffer, hit: &widgets::TextHit) {
    if hit.extend {
        buffer.set_cursor(hit.index, true);
    } else if hit.click_count >= 3 {
        buffer.select_all();
    } else if hit.click_count == 2 {
        buffer.select_word_at(hit.index);
    } else {
        buffer.set_cursor(hit.index, false);
    }
}
