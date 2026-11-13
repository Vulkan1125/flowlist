//! The main task list, project dashboards, review queue and nearby places.

use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate};
use gpui::{AnyElement, Context, IntoElement, MouseButton, Window, div, prelude::*, px};

use flowlist_core::command::Command;
use flowlist_core::ids::{ProjectId, TaskId};
use flowlist_core::model::{BuiltinPerspective, ProjectStatus, RepeatRule};
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::time::{Stamp, format_duration, human_date, local_date, now};

use super::drag::{DragTask, ghost};
use super::icons::{self, Icon};
use super::{EditField, FlowlistApp, FocusArea, View, group_label, sort_label, widgets};

impl FlowlistApp {
    pub(super) fn render_content(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let groups = self.current_groups();

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(theme.bg)
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.focus != FocusArea::Content {
                        this.focus = FocusArea::Content;
                        this.checklist_cursor = None;
                        this.vim.reset();
                        cx.notify();
                    }
                }),
            )
            .child(self.render_content_header(cx))
            .child(self.render_capture_bar(cx));

        if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)) {
            column = column.child(self.render_forecast_calendar(cx));
        }

        if let View::Project(id) = self.view.clone() {
            column = column.child(self.render_project_banner(&id, cx));
        }

        let mut list = div()
            .id("content-scroll")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll);

        let body = match &self.view {
            View::Builtin(BuiltinPerspective::Review) => self.render_review(cx).into_any_element(),
            View::Builtin(BuiltinPerspective::Nearby) => self.render_nearby(cx).into_any_element(),
            _ => self.render_groups(&groups, cx).into_any_element(),
        };
        list = list.child(body);
        column.child(list)
    }

    /// Dates in the library that carry a task, used for the Forecast calendar
    /// bars. Completed tasks and tasks in closed projects are left out, so the
    /// calendar agrees with the list below it.
    fn forecast_marked_dates(&self) -> BTreeSet<NaiveDate> {
        let db = self.store.db();
        let mut marked = BTreeSet::new();
        for task in db.tasks.values() {
            if task.completed {
                continue;
            }
            if let Some(project_id) = &task.project {
                match db.projects.get(project_id).map(|project| project.status) {
                    Some(ProjectStatus::Active) | None => {}
                    _ => continue,
                }
            }
            if let Some(due) = task.due {
                marked.insert(due.local_date());
            }
            if let Some(defer) = task.defer {
                marked.insert(defer.local_date());
            }
        }
        marked
    }

    /// Move the Forecast calendar one month forward or back.
    fn forecast_shift_month(&mut self, delta: i32) {
        let mut year = self.forecast_month.year();
        let mut month = self.forecast_month.month() as i32 + delta;
        while month < 1 {
            month += 12;
            year -= 1;
        }
        while month > 12 {
            month -= 12;
            year += 1;
        }
        if let Some(date) = NaiveDate::from_ymd_opt(year, month as u32, 1) {
            self.forecast_month = date;
        }
    }

    /// The month grid above the Forecast list. Monday-first, with an accent
    /// bar under every date that has a task; the list below stays unchanged.
    fn render_forecast_calendar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let today = local_date(now());
        let first = self.forecast_month;
        let marked = self.forecast_marked_dates();

        let (grid_start, _, weeks) = month_grid(first);

        let mut weekdays = div().flex().w_full().gap_1();
        for label in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
            weekdays = weekdays.child(
                div()
                    .flex_1()
                    .h(px(16.))
                    .text_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(label),
            );
        }

        let mut grid = div().flex().flex_col().gap_1().pt_1();
        for week in 0..weeks {
            let mut row = div().flex().w_full().gap_1();
            for weekday in 0..7 {
                let date = grid_start + Duration::days((week * 7 + weekday) as i64);
                let in_month = date.month() == first.month() && date.year() == first.year();
                let is_today = date == today;
                let has_tasks = marked.contains(&date);
                let number_color = if is_today {
                    theme.bg
                } else if has_tasks {
                    theme.text
                } else {
                    theme.text_dim
                };
                row = row.child(
                    div()
                        .flex_1()
                        .h(px(38.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .rounded_md()
                        .when(!in_month, |this| this.opacity(0.35))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(24.))
                                .rounded_full()
                                .when(is_today, |this| this.bg(theme.accent))
                                .text_xs()
                                .text_color(number_color)
                                .font_weight(if is_today {
                                    gpui::FontWeight::SEMIBOLD
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .child(date.day().to_string()),
                        )
                        .child(
                            div()
                                .h(px(3.))
                                .w(px(16.))
                                .rounded_full()
                                .when(has_tasks, |this| this.bg(theme.accent)),
                        ),
                );
            }
            grid = grid.child(row);
        }

        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_4()
            .pt_2()
            .pb_3()
            .border_b_1()
            .border_color(theme.border_soft)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pb_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(first.format("%B %Y").to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id("forecast-prev")
                                    .tooltip(widgets::tip(theme, "Previous month"))
                                    .child(widgets::button(theme, "‹", false))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.forecast_shift_month(-1);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("forecast-today")
                                    .tooltip(widgets::tip(theme, "Jump to this month"))
                                    .child(widgets::button(theme, "Today", false))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let today = local_date(now());
                                        if let Some(first) =
                                            NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                                        {
                                            this.forecast_month = first;
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(
                                div()
                                    .id("forecast-next")
                                    .tooltip(widgets::tip(theme, "Next month"))
                                    .child(widgets::button(theme, "›", false))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.forecast_shift_month(1);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(weekdays)
            .child(grid)
    }

    fn render_content_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let count = self
            .current_groups()
            .into_iter()
            .map(|group| group.rows.len())
            .sum::<usize>();
        let shows_completed =
            matches!(self.view, View::Project(_) | View::Folder(_) | View::Tag(_));
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .pt_4()
            .pb_2()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.text)
                                    .child(self.view_title()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_faint)
                                    .child(self.view_subtitle()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("search")
                                    .w(px(210.))
                                    .child(widgets::text_field(
                                        theme,
                                        &self.search,
                                        "Search tasks",
                                        self.focus == super::FocusArea::Search,
                                    ))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.focus = super::FocusArea::Search;
                                        this.search.select_all();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("sort")
                                    .tooltip(widgets::tip(theme, "Click to cycle the sort order"))
                                    .child(widgets::toggle_chip(
                                        theme,
                                        format!("Sort · {}", sort_label(self.sort)),
                                        false,
                                    ))
                                    .on_click(cx.listener(|this, _, _, cx| this.cycle_sort(cx))),
                            )
                            .child(
                                div()
                                    .id("group")
                                    .tooltip(widgets::tip(theme, "Click to cycle the grouping"))
                                    .child(widgets::toggle_chip(
                                        theme,
                                        format!("Group · {}", group_label(self.group)),
                                        false,
                                    ))
                                    .on_click(cx.listener(|this, _, _, cx| this.cycle_group(cx))),
                            )
                            .when(shows_completed, |this| {
                                this.child(
                                    div()
                                        .id("show-completed")
                                        .tooltip(widgets::tip(
                                            theme,
                                            "Show or hide completed tasks",
                                        ))
                                        .child(widgets::toggle_chip(
                                            theme,
                                            "Completed",
                                            self.show_completed,
                                        ))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_show_completed(cx)
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id("add-task")
                                    .child(widgets::button_with_icon(
                                        theme,
                                        Icon::Plus,
                                        "Task",
                                        true,
                                    ))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.focus = super::FocusArea::Capture;
                                        this.capture.end(false);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(format!("{count} task(s)"))
                    .when(self.store.can_undo(), |this| {
                        this.child(
                            div().child(
                                self.store
                                    .undo_label()
                                    .unwrap_or_else(|| "Undo".to_string()),
                            ),
                        )
                    }),
            )
    }

    fn render_capture_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div().px_4().pb_2().child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex().items_center().child(icons::icon(
                    Icon::Plus,
                    15.0,
                    theme.accent,
                )))
                .child(
                    div()
                        .id("capture-field")
                        .flex_1()
                        .child(widgets::text_field(
                            theme,
                            &self.capture,
                            "Add a task…  #tag  ::Project  !  due:tomorrow  est:15m",
                            self.focus == super::FocusArea::Capture,
                        ))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.focus = super::FocusArea::Capture;
                            this.capture.end(false);
                            cx.notify();
                        })),
                )
                .child(div().text_xs().text_color(theme.text_faint).child("⏎")),
        )
    }

    fn render_groups(&self, groups: &[Group], cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut list = div().flex().flex_col().w_full().py_1();
        let show_headers = groups.len() > 1
            || matches!(
                self.view,
                View::Builtin(BuiltinPerspective::Projects) | View::Folder(_) | View::Custom(_)
            );
        if groups.is_empty() {
            list = list.child(
                self.render_empty(
                    "Nothing here",
                    "Capture something above, or switch perspective from the sidebar.",
                )
                .into_any_element(),
            );
        }
        for group in groups {
            let open = self.is_open(&group.key, true);
            if show_headers {
                list = list.child(self.render_group_header(group, open, cx));
            }
            if open {
                for row in &group.rows {
                    if self.row_is_hidden(&row.id) {
                        continue;
                    }
                    list = list.child(self.render_task_row(row, cx));
                }
                if group.rows.is_empty() {
                    list = list.child(
                        div()
                            .px_4()
                            .py_2()
                            .text_sm()
                            .text_color(theme.text_faint)
                            .child("No tasks yet"),
                    );
                }
            }
        }
        list
    }

    fn row_is_hidden(&self, id: &TaskId) -> bool {
        let db = self.store.db();
        for ancestor in db.task_ancestors(id) {
            if !self.is_open(&format!("task:{ancestor}"), false) {
                return true;
            }
        }
        false
    }

    fn render_group_header(
        &self,
        group: &Group,
        open: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let key = group.key.clone();
        let open_key = key.clone();
        let project = project_id_from_key(&key);
        let mut row =
            div()
                .id(super::sid(format!("group:{}", group.key)))
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .pt_3()
                .pb_1()
                .border_b_1()
                .border_color(theme.border_soft)
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child(
                    div()
                        .w(px(12.))
                        .flex()
                        .items_center()
                        .text_color(theme.text_faint)
                        .child(icons::icon(
                            if open {
                                Icon::ChevronDown
                            } else {
                                Icon::ChevronRight
                            },
                            12.0,
                            theme.text_faint,
                        )),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_dim)
                        .child(group.title.clone()),
                )
                .child(widgets::count(theme, group.rows.len()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_group_collapsed(open_key.clone(), cx)
                }));
        if let Some(project) = project {
            let db = self.store.db();
            if let Some(entity) = db.projects.get(&project) {
                let (done, total) = db.project_progress(&project);
                row = row.child(
                    div()
                        .w(px(70.))
                        .child(widgets::progress(theme, done, total)),
                );
                if entity.status != ProjectStatus::Active {
                    row = row.child(widgets::chip(
                        theme,
                        entity.status.label(),
                        if entity.status == ProjectStatus::OnHold {
                            theme.warning
                        } else {
                            theme.text_faint
                        },
                    ));
                }
                if let Some(due) = entity.due {
                    row = row.child(widgets::chip(
                        theme,
                        format!("due {}", human_date(&due, now())),
                        theme.text_dim,
                    ));
                }
                let project_open = project.clone();
                row = row.child(
                    div()
                        .id("group-open-project")
                        .px_2()
                        .text_xs()
                        .text_color(theme.accent)
                        .cursor_pointer()
                        .hover(|style| style.text_color(theme.text))
                        .child("Open")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.open_view(View::Project(project_open.clone()), cx);
                        })),
                );
            }
        }
        row
    }

    fn render_task_row(&self, row: &TaskRow, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let Some(task) = db.tasks.get(&row.id) else {
            return div().into_any_element();
        };
        let id = row.id.clone();
        let selected = self.selected.contains(&id);
        let completed = task.completed;
        let flagged = task.flagged;
        let title = task.title.clone();
        let has_children = task.has_children();
        let expanded = self.is_open(&format!("task:{id}"), false);
        let estimate = task.estimate_min;
        let (checklist_done, checklist_total) = task.checklist_progress();
        let due = task.due;
        let defer = task.defer;
        let tags: Vec<String> = task
            .tags
            .iter()
            .filter_map(|tag| db.tags.get(tag).map(|tag| db.tag_path(&tag.id)))
            .take(3)
            .collect();
        let project = match &self.view {
            View::Builtin(BuiltinPerspective::Projects) | View::Project(_) => None,
            _ => task
                .project
                .as_ref()
                .map(|project| db.project_path(project)),
        };
        let availability = row.availability.clone();
        let short = id.short().to_string();
        let title_id = id.clone();
        let checkbox_id = id.clone();
        let flag_id = id.clone();
        let drop_id = id.clone();
        let disclosure_id = id.clone();
        let drag_payload = DragTask::task(id.clone(), title.clone());
        let due_color = due
            .map(|stamp| due_color(stamp, &theme))
            .unwrap_or(theme.text_faint);
        let title_color = if completed {
            theme.text_faint
        } else if availability.is_available() {
            theme.text
        } else {
            theme.text_dim
        };

        let base = div()
            .id(super::sid(format!("task:{}", id)))
            .flex()
            .items_center()
            .gap_2()
            .min_h(px(32.))
            .w_full()
            .pl(px(18.0 + row.depth as f32 * 18.0))
            .pr_4()
            .py_1()
            .border_b_1()
            .border_color(theme.border_soft)
            .bg(if selected { theme.selection } else { theme.bg })
            .text_sm()
            .cursor_pointer()
            .hover(|style| style.bg(theme.hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, _| window.focus(&this.focus_handle)),
            )
            .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
                let mods = event.modifiers();
                let double = match event {
                    gpui::ClickEvent::Mouse(mouse) => mouse.down.click_count >= 2,
                    _ => false,
                };
                if mods.shift {
                    this.select_range_to(title_id.clone());
                } else if mods.platform {
                    this.toggle_selection(title_id.clone());
                } else {
                    this.select_only(title_id.clone());
                    this.anchor = Some(title_id.clone());
                }
                if double {
                    this.show_inspector = true;
                    this.begin_edit(EditField::Title, cx);
                } else {
                    this.save_session();
                }
                cx.notify();
            }))
            .on_drag(drag_payload, move |payload, position, window, cx| {
                ghost(payload, position, window, cx, theme)
            })
            .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                this.drop_on_task(drop_id.clone(), payload, cx)
            }))
            .child(
                div()
                    .id(super::sid(format!("check:{}", checkbox_id)))
                    .tooltip(widgets::tip(
                        theme,
                        if completed {
                            "Reopen task (x)"
                        } else {
                            "Complete task (x)"
                        },
                    ))
                    .child(widgets::checkbox(
                        theme,
                        completed,
                        if flagged { theme.warning } else { theme.accent },
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_complete(checkbox_id.clone(), cx);
                    })),
            )
            .child(if has_children {
                div()
                    .id(super::sid(format!("disclosure:{}", disclosure_id)))
                    .w(px(12.))
                    .text_color(theme.text_faint)
                    .tooltip(widgets::tip(
                        theme,
                        if expanded {
                            "Collapse subtasks"
                        } else {
                            "Expand subtasks"
                        },
                    ))
                    .child(icons::icon(
                        if expanded {
                            Icon::ChevronDown
                        } else {
                            Icon::ChevronRight
                        },
                        12.0,
                        theme.text_faint,
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_expanded(format!("task:{}", disclosure_id), cx);
                    }))
                    .into_any_element()
            } else {
                div().w(px(12.)).into_any_element()
            })
            .child(
                div()
                    .id(super::sid(format!("flag:{}", flag_id)))
                    .w(px(14.))
                    .text_color(if flagged {
                        theme.danger
                    } else {
                        theme.text_faint
                    })
                    .opacity(if flagged { 1.0 } else { 0.35 })
                    .cursor_pointer()
                    .hover(|style| style.opacity(1.0))
                    .tooltip(widgets::tip(
                        theme,
                        if flagged {
                            "Unflag task (!)"
                        } else {
                            "Flag task (!)"
                        },
                    ))
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
                        cx.stop_propagation();
                        this.toggle_flag(flag_id.clone(), cx);
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(title_color)
                    .when(completed, |this| this.line_through())
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_shrink_0()
                    .children(tags.into_iter().map(|tag| widgets::tag_chip(theme, tag)))
                    .when_some(project, |this, project| {
                        this.child(div().text_xs().text_color(theme.text_faint).child(project))
                    })
                    .when_some(estimate, |this, minutes| {
                        this.child(widgets::chip(
                            theme,
                            format_duration(minutes),
                            theme.text_faint,
                        ))
                    })
                    .when(checklist_total > 0, |this| {
                        this.child(widgets::chip(
                            theme,
                            format!("☑ {checklist_done}/{checklist_total}"),
                            if checklist_done == checklist_total {
                                theme.success
                            } else {
                                theme.text_faint
                            },
                        ))
                    })
                    .when_some(defer.filter(|stamp| stamp.at > now()), |this, stamp| {
                        this.child(widgets::chip(
                            theme,
                            format!("⏳ {}", human_date(&stamp, now())),
                            theme.text_faint,
                        ))
                    })
                    .when_some(due, |this, stamp| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(due_color)
                                .child(human_date(&stamp, now())),
                        )
                    })
                    .when_some(repeat_label(db, &id), |this, label| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(format!("↻ {label}")),
                        )
                    })
                    .when(!availability.is_available() && !completed, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.warning)
                                .child(availability.label()),
                        )
                    })
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .opacity(0.0)
                            .child(short),
                    ),
            );
        base.into_any_element()
    }

    fn render_project_banner(&self, id: &ProjectId, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        let Some(project) = db.projects.get(id) else {
            return div();
        };
        let (done, total) = db.project_progress(id);
        let project_id = id.clone();
        let next_status = match project.status {
            ProjectStatus::Active => ProjectStatus::OnHold,
            ProjectStatus::OnHold => ProjectStatus::Active,
            ProjectStatus::Done => ProjectStatus::Active,
            ProjectStatus::Dropped => ProjectStatus::Active,
        };
        let notes = project.notes.clone();
        let next_review = project.next_review;
        let interval = project.review_interval_days;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .mx_4()
            .mb_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::progress(theme, done, total))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child(format!("{done}/{total}")),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.text_dim)
                            .child(format!(
                                "Review {} · every {} day(s)",
                                next_review.format("%b %-d"),
                                interval
                            )),
                    )
                    .child(
                        div()
                            .id("project-status")
                            .cursor_pointer()
                            .child(widgets::chip(
                                theme,
                                project.status.label(),
                                if project.status == ProjectStatus::Active {
                                    theme.success
                                } else {
                                    theme.warning
                                },
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.execute(Command::SetProjectStatus {
                                    id: project_id.clone(),
                                    status: next_status,
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("review-now")
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.accent)
                            .child("Review now")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if let View::Project(id) = this.view.clone() {
                                    this.execute(Command::CompleteReview {
                                        id,
                                        reviewed_on: local_date(now()),
                                    });
                                }
                                cx.notify();
                            })),
                    ),
            )
            .when(!notes.is_empty(), |this| {
                this.child(div().text_sm().text_color(theme.text_dim).child(notes))
            })
    }

    fn render_review(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        let at = now();
        let today = local_date(at);
        let mut projects: Vec<ProjectId> = db
            .projects
            .values()
            .filter(|project| project.status.is_open())
            .map(|project| project.id.clone())
            .collect();
        projects.sort_by_key(|id| db.projects[id].next_review);

        let mut list = div().flex().flex_col().gap_2().p_4();
        if projects.is_empty() {
            return list.child(self.render_empty(
                "No projects to review",
                "Create a project and it will show up here on its review date.",
            ));
        }
        for id in projects {
            let project = &db.projects[&id];
            let due = project.next_review <= today;
            let (done, total) = db.project_progress(&id);
            let open_id = id.clone();
            let review_id = id.clone();
            let drop_id = id.clone();
            list = list.child(
                div()
                    .id(super::sid(format!("review:{}", id)))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(if due { theme.warning } else { theme.border })
                    .bg(theme.raised)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_view(View::Project(open_id.clone()), cx)
                    }))
                    .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                        this.drop_on_project(drop_id.clone(), payload, cx)
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(project.name.clone()),
                            )
                            .child(widgets::chip(
                                theme,
                                if due {
                                    "Review due".to_string()
                                } else {
                                    format!("Review {}", project.next_review.format("%b %-d"))
                                },
                                if due { theme.warning } else { theme.text_faint },
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(160.))
                                    .child(widgets::progress(theme, done, total)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child(format!("{done}/{total} complete")),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.success)
                                    .child(project.status.label()),
                            )
                            .child(
                                div()
                                    .id("review-now")
                                    .ml_auto()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.accent)
                                    .text_xs()
                                    .text_color(theme.accent)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.selection))
                                    .child("Mark reviewed")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.execute(Command::CompleteReview {
                                            id: review_id.clone(),
                                            reviewed_on: local_date(now()),
                                        });
                                        cx.notify();
                                    })),
                            ),
                    ),
            );
        }
        list
    }

    fn render_nearby(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        let at = now();
        if db.settings.current_location.is_none() {
            let mut empty = div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .p_8()
                .child(self.render_empty(
                    "Set your location",
                    "Nearby ranks tagged places by distance. Enter a latitude and longitude and Flowlist will show what is close.",
                ));
            empty = empty.child(
                div()
                    .id("set-location")
                    .child(widgets::button(theme, "Set location", true))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_modal(super::modals::Modal::location(this), cx)
                    })),
            );
            return empty;
        }
        let hits = query::nearby(db, at);
        let mut list = div().flex().flex_col().p_4().gap_1();
        list = list.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb_2()
                .child(div().text_sm().text_color(theme.text_dim).child(format!(
                    "{} place-based task(s) within {} km",
                    hits.len(),
                    db.settings.nearby_radius_km
                )))
                .child(
                    div()
                        .id("set-location")
                        .text_xs()
                        .text_color(theme.accent)
                        .cursor_pointer()
                        .child("Change location")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.open_modal(super::modals::Modal::location(this), cx)
                        })),
                ),
        );
        if hits.is_empty() {
            list = list.child(self.render_empty(
                "Nothing nearby",
                "Tasks tagged with located contexts inside your radius will appear here.",
            ));
            return list;
        }
        for hit in hits {
            let row = query::row_for(db, &hit.task, at);
            let tag = db.tag_path(&hit.via_tag);
            list = list.child(
                div()
                    .id(super::sid(format!("nearby:{}", hit.task)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(theme.border_soft)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .w(px(110.))
                            .text_xs()
                            .text_color(theme.success)
                            .child(format!("{:.1} km", hit.distance_km))
                            .child(div().text_color(theme.text_faint).child(tag)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.render_task_row(&row, cx)),
                    ),
            );
        }
        list
    }
}

/// The Monday-first grid for a month: the first visible date, how many days
/// the month has, and how many week rows the grid needs.
fn month_grid(month: NaiveDate) -> (NaiveDate, i64, usize) {
    let first = NaiveDate::from_ymd_opt(month.year(), month.month(), 1).unwrap_or(month);
    let offset = i64::from(first.weekday().num_days_from_monday());
    let days_in_month = if first.month() == 12 {
        NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
    }
    .and_then(|next| next.pred_opt())
    .map(|last| i64::from(last.day()))
    .unwrap_or(31);
    let grid_start = first - Duration::days(offset);
    let weeks = ((offset + days_in_month + 6) / 7) as usize;
    (grid_start, days_in_month, weeks)
}

fn due_color(stamp: Stamp, theme: &super::theme::Theme) -> gpui::Hsla {
    let today = local_date(now());
    let date = stamp.local_date();
    if date < today {
        theme.danger
    } else if date == today {
        theme.warning
    } else {
        theme.text_faint
    }
}

fn project_id_from_key(key: &str) -> Option<ProjectId> {
    key.strip_prefix("project:").map(ProjectId::from_string)
}

fn repeat_label(db: &flowlist_core::db::Db, id: &TaskId) -> Option<String> {
    let task = db.tasks.get(id)?;
    let rule: &RepeatRule = task.repeat.as_ref()?;
    Some(rule.description())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_grid_starts_on_monday_and_covers_the_month() {
        // February 2026 begins on a Sunday, so the grid opens on 26 January.
        let (start, days, weeks) = month_grid(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 1, 26).unwrap());
        assert_eq!(days, 28);
        assert_eq!(weeks, 5);

        // January 2026 begins on a Thursday.
        let (start, days, weeks) = month_grid(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(start, NaiveDate::from_ymd_opt(2025, 12, 29).unwrap());
        assert_eq!(days, 31);
        assert_eq!(weeks, 5);

        // August 2026 begins on a Saturday and needs six rows.
        let (start, days, weeks) = month_grid(NaiveDate::from_ymd_opt(2026, 8, 15).unwrap());
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 7, 27).unwrap());
        assert_eq!(days, 31);
        assert_eq!(weeks, 6);

        // December rolls the year over for the last day and the next grid.
        let (start, days, weeks) = month_grid(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 11, 30).unwrap());
        assert_eq!(days, 31);
        assert_eq!(weeks, 5);
    }

    #[test]
    fn month_grid_never_loses_a_day() {
        for year in 2024..=2028 {
            for month in 1..=12 {
                let (start, days, weeks) =
                    month_grid(NaiveDate::from_ymd_opt(year, month, 1).expect("valid month"));
                assert_eq!(start.weekday().num_days_from_monday(), 0);
                assert!(weeks >= 4 && weeks <= 6);
                let last = start + Duration::days((weeks * 7 - 1) as i64);
                let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
                assert!(last >= first + Duration::days(days - 1));
                assert!(start <= first);
                assert!((first - start).num_days() < 7);
            }
        }
    }
}
