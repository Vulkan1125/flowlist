//! The main task list, project dashboards, review queue and nearby places.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use chrono::{Datelike, Duration, NaiveDate};
use gpui::{
    AnyElement, Context, CursorStyle, DragMoveEvent, IntoElement, MouseButton, ScrollDelta,
    SharedString, TextRun, Window, div, font, prelude::*, px,
};

use flowlist_core::command::{Command, ProjectPatch};
use flowlist_core::db::Db;
use flowlist_core::ids::{ProjectId, TaskId};
use flowlist_core::model::{
    BuiltinPerspective, GroupBy, Priority, ProjectStatus, RepeatRule, SortBy,
};
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::time::{Stamp, format_duration, human_date, local_date, now};

use super::drag::{DragForecastResize, DragTask, ghost, resize_ghost};
use super::icons::{self, Icon};
use super::widgets::TipExt;
use super::{
    EditField, FlowlistApp, FocusArea, View, group_label, group_slug, sort_label, sort_slug,
    widgets,
};

impl FlowlistApp {
    pub(super) fn render_content(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        // Only soften the content when the appearance on screen actually has
        // a wallpaper. A dark-only wallpaper must not make the plain light
        // background translucent.
        let content_bg = if self.active_wallpaper().is_some() {
            theme.bg.opacity(0.80)
        } else {
            theme.bg
        };
        let groups = self.current_groups();
        // Rebuilt as rows paint, so the test state lists only what is on
        // screen, including the subtasks an expanded parent reveals.
        self.content_rows.borrow_mut().clear();
        self.empty_note.borrow_mut().take();

        let mut column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(content_bg)
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.focus != FocusArea::Content {
                        // Clicking away from an inspector field ends that field
                        // session, including a first-time task's draft state.
                        if this.focus == FocusArea::Field {
                            this.editing = None;
                            this.field_error = None;
                            this.draft_task = None;
                        }
                        this.focus = FocusArea::Content;
                        this.checklist_cursor = None;
                        this.vim.reset();
                        cx.notify();
                    }
                }),
            )
            .child(self.render_content_header(&groups, window, cx))
            .when(self.view.shows_capture(), |this| {
                this.child(self.render_capture_bar(cx))
            });

        if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast))
            && self.forecast_calendar_open
        {
            column = column.child(if self.forecast_calendar_expanded {
                self.render_forecast_calendar(cx).into_any_element()
            } else {
                self.render_forecast_week_strip(cx).into_any_element()
            });
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
            View::Builtin(BuiltinPerspective::Archive) => {
                self.render_archive(cx).into_any_element()
            }
            _ => self.render_groups(&groups, cx).into_any_element(),
        };
        list = list.child(body);
        column.child(list)
    }

    /// Open tasks with a due date, grouped by the local day they land on.
    /// Completed tasks, trashed tasks and tasks in closed projects are left
    /// out so the grid and the Forecast list agree.
    fn forecast_tasks_by_day(&self) -> BTreeMap<NaiveDate, Vec<TaskId>> {
        let db = self.store.db();
        let mut days: BTreeMap<NaiveDate, Vec<TaskId>> = BTreeMap::new();
        for (id, task) in &db.tasks {
            if task.completed || task.is_trashed() {
                continue;
            }
            if let Some(project_id) = &task.project {
                match db.projects.get(project_id).map(|project| project.status) {
                    Some(ProjectStatus::Active) | None => {}
                    _ => continue,
                }
            }
            let mut dates = vec![
                task.due.map(|stamp| stamp.local_date()),
                task.defer.map(|stamp| stamp.local_date()),
            ];
            if self.forecast_planned {
                dates.push(task.planned.map(|stamp| stamp.local_date()));
            }
            for date in dates.into_iter().flatten() {
                let ids = days.entry(date).or_default();
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
        for ids in days.values_mut() {
            ids.sort_by_cached_key(|id| {
                let task = &db.tasks[id];
                (
                    task.due
                        .or(task.planned)
                        .or(task.defer)
                        .map(|stamp| stamp.at),
                    task.title.to_lowercase(),
                )
            });
        }
        days
    }

    /// Move the selected Forecast day, creating one at today when none is set.
    pub(super) fn forecast_step_date(&mut self, delta: i64) {
        let today = local_date(now());
        let next = self
            .forecast_date
            .unwrap_or(today)
            .checked_add_signed(Duration::days(delta))
            .unwrap_or(today);
        self.forecast_date = Some(next);
        self.forecast_month = NaiveDate::from_ymd_opt(next.year(), next.month(), 1).unwrap_or(next);
        self.forecast_week = week_start(next);
        self.forecast_calendar_open = true;
        self.forecast_calendar_expanded = true;
        self.selected.clear();
        self.anchor = None;
        self.save_session();
    }

    /// Move the Forecast calendar one month forward or back.
    pub(super) fn forecast_shift_month(&mut self, delta: i32) {
        self.forecast_month = add_months(self.forecast_month, delta);
    }

    /// Bring Forecast back to today.
    pub(super) fn forecast_today(&mut self) {
        let today = local_date(now());
        self.forecast_month =
            NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
        self.forecast_date = Some(today);
        self.forecast_week = week_start(today);
        self.forecast_calendar_open = true;
        self.forecast_calendar_expanded = true;
        self.selected.clear();
        self.anchor = None;
        self.save_session();
    }

    /// Show or hide the Forecast calendar. The mode (week strip or full month)
    /// and the week are remembered, so reopening restores the last position
    /// instead of resetting to the strip. Hiding always drops a day filter so
    /// the list returns to the full agenda.
    pub(super) fn toggle_forecast_calendar(&mut self) {
        self.forecast_calendar_open = !self.forecast_calendar_open;
        if !self.forecast_calendar_open {
            self.forecast_date = None;
            self.selected.clear();
            self.anchor = None;
        }
        self.save_session();
    }

    /// Switch the open calendar between the compact week strip and the full
    /// month grid, keeping the selected day and week.
    pub(super) fn set_forecast_calendar_expanded(&mut self, expanded: bool) {
        self.forecast_calendar_open = true;
        self.forecast_calendar_expanded = expanded;
        self.save_session();
    }

    /// Move the collapsed Forecast strip one week forward or back. The strip
    /// never moves before the current week; the full month grid has no such
    /// limit. The selected weekday follows the week.
    pub(super) fn forecast_shift_week(&mut self, delta: i64) {
        let today = local_date(now());
        let base = week_start(today);
        let current = week_start(self.forecast_week).max(base);
        let next = (current + Duration::days(delta * 7)).max(base);
        let anchor = self.forecast_date.unwrap_or(today);
        let offset = anchor.weekday().num_days_from_monday() as i64;
        let date = next + Duration::days(offset);
        self.forecast_week = next;
        self.forecast_date = Some(date);
        self.forecast_month =
            NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
        self.selected.clear();
        self.anchor = None;
        self.save_session();
    }

    /// Pick a day from the collapsed week strip, or clear the filter when the
    /// same day is clicked again.
    pub(super) fn forecast_pick_day(&mut self, date: NaiveDate) {
        if self.forecast_date == Some(date) {
            self.forecast_date = None;
        } else {
            self.forecast_date = Some(date);
            self.forecast_month =
                NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
            self.forecast_week = week_start(date).max(week_start(local_date(now())));
        }
        self.selected.clear();
        self.anchor = None;
        self.save_session();
    }

    /// A dragged calendar height is spread across the month's week rows, so a
    /// five-week month and a six-week month keep the same overall block size.
    fn forecast_cell_height(&self, month: NaiveDate) -> f32 {
        let weeks = month_grid(month).2 as f32;
        match self.forecast_calendar_height {
            Some(height) => ((height - FORECAST_CALENDAR_CHROME) / weeks)
                .clamp(FORECAST_CELL_MIN, FORECAST_CELL_MAX),
            None => 68.0,
        }
    }

    /// Keyboard counterpart to the resize grip: step the calendar height by a
    /// row or so, starting from its natural size when the user has not dragged.
    pub(super) fn nudge_forecast_height(&mut self, delta: f32, cx: &mut Context<Self>) {
        let weeks = month_grid(self.forecast_month).2 as f32;
        let current = self
            .forecast_calendar_height
            .unwrap_or(weeks * 68.0 + FORECAST_CALENDAR_CHROME);
        self.set_forecast_calendar_height(current + delta, cx);
    }

    /// Set the calendar height from the resize grip, clamped so a month can
    /// never collapse below readable cells or swallow the whole pane. The
    /// session write is debounced, since a drag produces a move every frame.
    pub(super) fn set_forecast_calendar_height(&mut self, height: f32, cx: &mut Context<Self>) {
        let weeks = month_grid(self.forecast_month).2 as f32;
        let min = weeks * FORECAST_CELL_MIN + FORECAST_CALENDAR_CHROME;
        let max = weeks * FORECAST_CELL_MAX + FORECAST_CALENDAR_CHROME;
        self.forecast_calendar_height = Some(height.clamp(min, max));
        if !self.forecast_height_dirty {
            self.forecast_height_dirty = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(400))
                    .await;
                this.update(cx, |this, _cx| {
                    this.forecast_height_dirty = false;
                    this.save_session();
                })
                .ok();
            })
            .detach();
        }
        cx.notify();
    }

    /// The Forecast month grid. It reads like macOS Calendar: a quiet,
    /// continuous grid with separators, one accent circle for today and
    /// tinted event pills. One month, always full width, folded away by the
    /// header chip in the content pane when the user wants the list alone.
    fn render_forecast_calendar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let today = local_date(now());
        let first = self.forecast_month;
        let days = self.forecast_tasks_by_day();
        let window_width = self.window_size.map_or(1254.0, |(width, _)| width);
        let pane_width = forecast_pane_width(window_width, self.show_inspector);
        let compact = pane_width < 620.0;
        let calendar_anchor = self.forecast_calendar_anchor.clone();
        let grip_anchor = self.forecast_grip_anchor.clone();
        let collapse_anchor = self.forecast_calendar_collapse_anchor.clone();

        let mut controls = div().flex().flex_shrink_0().items_center().gap_1();
        if let Some(date) = self.forecast_date {
            controls = controls.child(if compact {
                div()
                    .id("forecast-all-dates")
                    .flow_tip(self, cx, theme, "Show all dates")
                    .child(widgets::icon_button(theme, Icon::Close, true))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_date = None;
                        this.selected.clear();
                        this.anchor = None;
                        this.save_session();
                        cx.notify();
                    }))
                    .into_any_element()
            } else {
                div()
                    .id("forecast-all-dates")
                    .child(widgets::button(
                        theme,
                        format!("All dates · {}", date.format("%b %-d")),
                        true,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_date = None;
                        this.selected.clear();
                        this.anchor = None;
                        this.save_session();
                        cx.notify();
                    }))
                    .into_any_element()
            });
        }
        let controls = controls
            .child(
                div()
                    .id("forecast-prev")
                    .flow_tip(self, cx, theme, "Previous month (⇧{ )")
                    .child(widgets::button(theme, "‹", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_shift_month(-1);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("forecast-today")
                    .flow_tip(self, cx, theme, "Jump to today (t)")
                    .child(widgets::button(theme, "Today", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_today();
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("forecast-next")
                    .flow_tip(self, cx, theme, "Next month (⇧} )")
                    .child(widgets::button(theme, "›", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_shift_month(1);
                        cx.notify();
                    })),
            );

        let title = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(first.format("%B %Y").to_string()),
            )
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            collapse_anchor.set(Some(*bounds));
                        }
                    })
                    .id("forecast-calendar-collapse")
                    .flow_tip(self, cx, theme, "Show the week strip")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(20.))
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child(icons::icon(Icon::ChevronUp, 12.0, theme.text_faint)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_forecast_calendar_expanded(false);
                        cx.notify();
                    })),
            );
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .pb_1()
            .child(title)
            .child(controls);

        div()
            .flex()
            .flex_col()
            .w_full()
            .px_4()
            .pt_2()
            .pb_3()
            .border_b_1()
            .border_color(theme.border_soft)
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(bounds) = bounds.first() {
                    calendar_anchor.set(Some(*bounds));
                }
            })
            // A drag on the bottom grip resizes the month grid. The pointer's
            // offset from the calendar's top edge is the height the user is
            // asking for, so the block tracks the cursor exactly.
            .on_drag_move::<DragForecastResize>(cx.listener(
                |this, event: &DragMoveEvent<DragForecastResize>, _, cx| {
                    let height = f32::from(event.event.position.y - event.bounds.origin.y);
                    this.set_forecast_calendar_height(height, cx);
                },
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .gap_1()
                    .child(header)
                    .child(self.render_forecast_month(first, today, &days, cx))
                    .child(
                        div()
                            .on_children_prepainted(move |bounds, _, _| {
                                if let Some(bounds) = bounds.first() {
                                    grip_anchor.set(Some(*bounds));
                                }
                            })
                            .child(
                                div()
                                    .id("forecast-resize")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w_full()
                                    .h(px(10.))
                                    .cursor(CursorStyle::ResizeUpDown)
                                    .hover(|style| style.bg(theme.hover))
                                    .flow_tip(
                                        self,
                                        cx,
                                        theme,
                                        "Drag to resize the calendar (- / =)",
                                    )
                                    .child(
                                        div().w(px(38.)).h(px(3.)).rounded_full().bg(theme.border),
                                    )
                                    .on_drag(DragForecastResize, resize_ghost),
                            ),
                    ),
            )
    }

    /// The collapsed Forecast calendar: a compact week strip under a month
    /// heading, with a bottom pill that opens the full month grid. The full
    /// calendar itself stays exactly as it was.
    fn render_forecast_week_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let today = local_date(now());
        let base = week_start(today);
        let week = week_start(self.forecast_week).max(base);
        let anchor = self.forecast_date.unwrap_or(week);
        let days = self.forecast_tasks_by_day();
        let at_base = week <= base;
        let labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let title = anchor.format("%B %Y").to_string();
        let strip_boxes = self.forecast_strip_boxes.clone();
        let next_anchor = self.forecast_strip_next_anchor.clone();
        let expand_anchor = self.forecast_strip_expand_anchor.clone();

        let mut header = div()
            .flex()
            .items_center()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            expand_anchor.set(Some(*bounds));
                        }
                    })
                    .id("forecast-week-expand")
                    .flow_tip(self, cx, theme, "Show the full month")
                    .child(div().flex().items_center().justify_center().size(px(20.)).rounded_md().cursor_pointer().hover(|style| style.bg(theme.hover)).child(icons::icon(Icon::ChevronDown, 12.0, theme.text_faint)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.set_forecast_calendar_expanded(true);
                        cx.notify();
                    })),
            )
            .child(div().flex_grow());
        header = header.child(
            div()
                .id("forecast-week-prev")
                .flow_tip(self, cx, theme, "Previous week")
                .when(at_base, |this| this.opacity(0.35))
                .child(widgets::icon_button(theme, Icon::ArrowLeft, false))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !at_base {
                        this.forecast_shift_week(-1);
                        cx.notify();
                    }
                })),
        );
        header = header.child(
            div()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        next_anchor.set(Some(*bounds));
                    }
                })
                .id("forecast-week-next")
                .flow_tip(self, cx, theme, "Next week")
                .child(widgets::icon_button(theme, Icon::ArrowRight, false))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.forecast_shift_week(1);
                    cx.notify();
                })),
        );

        let mut row = div().flex().w_full().gap_1().pt_2();
        for (index, label) in labels.iter().enumerate() {
            let date = week + Duration::days(index as i64);
            let count = days.get(&date).map_or(0, Vec::len);
            let selected = self.forecast_date == Some(date);
            let is_today = date == today;
            let drop_date = date;
            let box_sink = strip_boxes.clone();
            let box_key = date.to_string();
            row = row.child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        record_box(&box_sink, &box_key, bounds.first().copied());
                    })
                    .id(super::sid(format!("forecast-week-date:{date}")))
                    .flex_1()
                    .min_w_0()
                    .h(px(80.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .rounded_md()
                    .bg(if selected {
                        theme.selection
                    } else if is_today {
                        theme.field
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child(*label),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(22.))
                            .rounded_full()
                            .when(is_today, |this| this.bg(theme.accent))
                            .text_sm()
                            .font_weight(if is_today {
                                gpui::FontWeight::SEMIBOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if is_today {
                                theme.bg
                            } else if selected {
                                theme.accent
                            } else {
                                theme.text
                            })
                            .child(date.day().to_string()),
                    )
                    .child(
                        div()
                            .h(px(12.))
                            .text_xs()
                            .text_color(if selected { theme.accent } else { theme.text_faint })
                            .child(if count > 0 { count.to_string() } else { String::new() }),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.forecast_pick_day(drop_date);
                        cx.notify();
                    }))
                    .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                        this.drop_on_forecast_day(drop_date, payload, cx)
                    })),
            );
        }

        div()
            .flex()
            .flex_col()
            .w_full()
            .px_4()
            .pt_2()
            .pb_1()
            .border_b_1()
            .border_color(theme.border_soft)
            .child(header)
            .child(
                div()
                    .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                        let dx = match event.delta {
                            ScrollDelta::Pixels(delta) => f32::from(delta.x),
                            ScrollDelta::Lines(delta) => delta.x,
                        };
                        if dx > 0.5 {
                            this.forecast_shift_week(-1);
                            cx.notify();
                        } else if dx < -0.5 {
                            this.forecast_shift_week(1);
                            cx.notify();
                        }
                    }))
                    .child(row),
            )
    }

    /// One month's macOS-style grid. Each day is a cell in a single continuous
    /// surface: separators instead of cards, the day number in the corner, and
    /// up to two event pills.
    fn render_forecast_month(
        &self,
        first: NaiveDate,
        today: NaiveDate,
        days: &BTreeMap<NaiveDate, Vec<TaskId>>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        let cell_height = self.forecast_cell_height(first);
        let (grid_start, _, weeks) = month_grid(first);
        let day_boxes = self.forecast_day_boxes.clone();
        let chip_boxes = self.forecast_chip_boxes.clone();

        let mut weekdays = div().flex().w_full();
        for label in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
            weekdays = weekdays.child(
                div()
                    .flex_1()
                    .h(px(18.))
                    .text_center()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(label),
            );
        }

        let mut grid = div()
            .flex()
            .flex_col()
            .w_full()
            .rounded_lg()
            .border_1()
            .border_color(theme.border_soft)
            .overflow_hidden();

        for week in 0..weeks {
            let mut row = div().flex().w_full().when(week + 1 < weeks, |this| {
                this.border_b_1().border_color(theme.border_soft)
            });
            for weekday in 0..7 {
                let date = grid_start + Duration::days((week * 7 + weekday) as i64);
                let in_month = date.month() == first.month() && date.year() == first.year();
                let is_today = date == today;
                let is_selected = self.forecast_date == Some(date);
                let ids = days.get(&date).cloned().unwrap_or_default();
                let count = ids.len();
                let cell_key = date.to_string();
                let box_sink = day_boxes.clone();
                let drop_date = date;
                let select_date = date;
                let select_month =
                    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);

                let mut cell = div()
                    .id(super::sid(format!("forecast-date:{date}")))
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h(px(cell_height))
                    .gap_1()
                    .px_1()
                    .py_1()
                    .when(weekday < 6, |this| {
                        this.border_r_1().border_color(theme.border_soft)
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .when(is_selected, |this| this.bg(theme.selection))
                    .when(!in_month, |this| this.opacity(0.42))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.forecast_date == Some(select_date) {
                            this.forecast_date = None;
                        } else {
                            this.forecast_date = Some(select_date);
                            this.forecast_month = select_month;
                            this.forecast_week = week_start(select_date);
                        }
                        this.selected.clear();
                        this.anchor = None;
                        this.save_session();
                        cx.notify();
                    }))
                    .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                        this.drop_on_forecast_day(drop_date, payload, cx)
                    }));

                cell = cell.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(20.))
                                .rounded_full()
                                .when(is_today, |this| this.bg(theme.accent))
                                .text_xs()
                                .font_weight(if is_today {
                                    gpui::FontWeight::SEMIBOLD
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .text_color(if is_today {
                                    theme.bg
                                } else if in_month {
                                    theme.text
                                } else {
                                    theme.text_dim
                                })
                                .child(date.day().to_string()),
                        )
                        .when(count > 0, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child(count.to_string()),
                            )
                        }),
                );

                for id in ids.iter().take(2) {
                    let Some(task) = db.tasks.get(id) else {
                        continue;
                    };
                    let title = task.title.clone();
                    let payload = DragTask::task(id.clone(), title.clone());
                    let click_id = id.clone();
                    let chip_key = id.to_string();
                    let chip_sink = chip_boxes.clone();
                    cell = cell.child(
                        div()
                            .on_children_prepainted(move |bounds, _, _| {
                                record_box(&chip_sink, &chip_key, bounds.first().copied());
                            })
                            .child(
                                div()
                                    .id(super::sid(format!("forecast-chip:{}", click_id)))
                                    .flex()
                                    .items_center()
                                    .h(px(15.))
                                    .w_full()
                                    .min_w_0()
                                    .px_1()
                                    .rounded_sm()
                                    .bg(theme.accent_soft)
                                    .text_xs()
                                    .text_color(theme.text)
                                    .truncate()
                                    .text_ellipsis()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.forecast_date = Some(date);
                                        this.forecast_month = select_month;
                                        this.forecast_week = week_start(date);
                                        this.select_only(click_id.clone());
                                        this.anchor = Some(click_id.clone());
                                        this.focus = FocusArea::Content;
                                        cx.notify();
                                    }))
                                    .on_drag(payload, move |payload, position, window, cx| {
                                        ghost(payload, position, window, cx, theme)
                                    })
                                    .child(title),
                            ),
                    );
                }
                if count > 2 {
                    cell = cell.child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child(format!("+{} more", count - 2)),
                    );
                }

                row = row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .on_children_prepainted(move |bounds, _, _| {
                            record_box(&box_sink, &cell_key, bounds.first().copied());
                        })
                        .child(cell),
                );
            }
            grid = grid.child(row);
        }

        div()
            .flex()
            .flex_1()
            .min_w_0()
            .flex_col()
            .gap_1()
            .child(weekdays)
            .child(grid)
    }

    fn render_content_header(
        &self,
        groups: &[Group],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        // A library-wide search is its own place: the view name would be a lie
        // when the rows come from everywhere.
        let library_search =
            self.search_library && !self.search.is_empty() && self.view.searches_tasks();
        let title = if library_search {
            "Search".to_string()
        } else {
            self.view_title()
        };
        // The title shares its row with the chips, leaving it no room to wrap.
        // Shape it up front and paint it only when every letter fits: a name
        // clipped to a few characters reads as a bug, so it is left out.
        let shaped = window.text_system().shape_line(
            SharedString::from(title.clone()),
            px(24.),
            &[TextRun {
                len: title.len(),
                font: font("SF Pro Text").bold(),
                color: theme.text,
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        );
        let title_width = f32::from(shaped.width);
        let title_fits = self
            .title_space
            .get()
            .is_none_or(|space| title_width + 1.0 <= space);
        self.title_visible.set(title_fits);
        let title_block = div().min_w_0().flex_1().when(title_fits, |this| {
            this.child(
                div()
                    .text_2xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.text)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(title),
            )
        });
        let is_archive = matches!(self.view, View::Builtin(BuiltinPerspective::Archive));
        // A Forecast day filter names the day in the header count, so the
        // header reads "1 task for 23 Nov" while a date is picked.
        let forecast_day = if matches!(self.view, View::Builtin(BuiltinPerspective::Forecast))
            && self.forecast_calendar_open
        {
            self.forecast_date
        } else {
            None
        };
        // The caller already ran the query for the list; reuse it instead of
        // running the whole view a second time just to count the rows.
        let count = if is_archive {
            query::archived_projects(self.store.db()).len()
        } else {
            groups.iter().map(|group| group.rows.len()).sum::<usize>()
        };
        let shows_completed =
            matches!(self.view, View::Project(_) | View::Folder(_) | View::Tag(_));
        let is_trash = matches!(self.view, View::Builtin(BuiltinPerspective::Trash));
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
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(title_block)
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .items_center()
                            .gap_2()
                            .when(
                                matches!(self.view, View::Builtin(BuiltinPerspective::Today)),
                                |this| {
                                    let anchor = self.hide_past_anchor.clone();
                                    this.child(
                                        div()
                                            .flex_shrink_0()
                                            .child(
                                                div()
                                                    .id("hide-past")
                                                    .flow_tip(
                                                        self,
                                                        cx,
                                                        theme,
                                                        "Show or hide past plans and overdue tasks in Today",
                                                    )
                                                    .child(widgets::toggle_chip(
                                                        theme,
                                                        "Hide past",
                                                        self.hide_past,
                                                    ))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.hide_past = !this.hide_past;
                                                        this.remember_view_options();
                                                        this.save_session();
                                                        cx.notify();
                                                    })),
                                            )
                                            .on_children_prepainted(move |bounds, _, _| {
                                                if let Some(bounds) = bounds.first() {
                                                    anchor.set(Some(*bounds));
                                                }
                                            }),
                                    )
                                },
                            )
                            .when(
                                matches!(self.view, View::Builtin(BuiltinPerspective::Forecast)),
                                |this| {
                                    let anchor = self.forecast_toggle_anchor.clone();
                                    this.child(
                                        div()
                                            .flex_shrink_0()
                                            .child(
                                                div()
                                                    .id("forecast-calendar-toggle")
                                                    .flow_tip(
                                                        self,
                                                        cx,
                                                        theme,
                                                        "Show or hide the Forecast calendar",
                                                    )
                                                    .child(widgets::toggle_chip(
                                                        theme,
                                                        "Calendar",
                                                        self.forecast_calendar_open,
                                                    ))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_forecast_calendar();
                                                        cx.notify();
                                                    })),
                                            )
                                            .on_children_prepainted(move |bounds, _, _| {
                                                if let Some(bounds) = bounds.first() {
                                                    anchor.set(Some(*bounds));
                                                }
                                            })
                                            .child(
                                                div()
                                                    .on_children_prepainted(self.theme_probe(
                                                        "forecast-planned-toggle".to_string(),
                                                    ))
                                                    .id("forecast-planned-toggle")
                                                    .flex_shrink_0()
                                                    .flow_tip(
                                                        self,
                                                        cx,
                                                        theme,
                                                        "Show or hide items on their planned date",
                                                    )
                                                    .child(widgets::toggle_chip(
                                                        theme,
                                                        "Planned",
                                                        self.forecast_planned,
                                                    ))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.forecast_planned =
                                                            !this.forecast_planned;
                                                        this.save_session();
                                                        cx.notify();
                                                    })),
                                            ),
                                    )
                                },
                            )
                            .when(!is_trash && !is_archive, |this| {
                                this.child({
                                    let anchor = self.sort_anchor.clone();
                                    div()
                                        .flex_shrink_0()
                                        .child(
                                            div()
                                                .id("sort")
                                                .flow_tip(self, cx, theme, "Pick a sort order")
                                                .child(widgets::toggle_chip(
                                                    theme,
                                                    format!("Sort · {}", sort_label(self.sort)),
                                                    self.sort_menu_open,
                                                ))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let suppress =
                                                        std::mem::take(&mut this.sort_closed_on_up);
                                                    this.sort_menu_open =
                                                        !this.sort_menu_open && !suppress;
                                                    this.group_menu_open = false;
                                                    this.priority_menu_open = false;
                                                    this.sidebar_menu = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .on_children_prepainted(move |bounds, _, _| {
                                            if let Some(bounds) = bounds.first() {
                                                anchor.set(Some(*bounds));
                                            }
                                        })
                                })
                                .child({
                                    let anchor = self.group_anchor.clone();
                                    div()
                                        .flex_shrink_0()
                                        .child(
                                            div()
                                                .id("group")
                                                .flow_tip(self, cx, theme, "Pick a grouping")
                                                .child(widgets::toggle_chip(
                                                    theme,
                                                    format!("Group · {}", group_label(self.group)),
                                                    self.group_menu_open,
                                                ))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let suppress = std::mem::take(
                                                        &mut this.group_closed_on_up,
                                                    );
                                                    this.group_menu_open =
                                                        !this.group_menu_open && !suppress;
                                                    this.sort_menu_open = false;
                                                    this.priority_menu_open = false;
                                                    this.sidebar_menu = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .on_children_prepainted(move |bounds, _, _| {
                                            if let Some(bounds) = bounds.first() {
                                                anchor.set(Some(*bounds));
                                            }
                                        })
                                })
                                .child(
                                    div()
                                        .id("only-available")
                                        .flex_shrink_0()
                                        .flow_tip(
                                            self,
                                            cx,
                                            theme,
                                            "Show only the next available task (First Available)",
                                        )
                                        .child(widgets::toggle_chip(
                                            theme,
                                            "Available",
                                            self.only_available,
                                        ))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_only_available(cx)
                                        })),
                                )
                                .child({
                                    let anchor = self.priority_anchor.clone();
                                    div()
                                        .flex_shrink_0()
                                        .child(
                                            div()
                                                .id("priority-filter")
                                                .flow_tip(
                                                    self,
                                                    cx,
                                                    theme,
                                                    "Filter every view by priority",
                                                )
                                                .child(widgets::toggle_chip(
                                                    theme,
                                                    format!(
                                                        "Priority · {}",
                                                        self.priority_filter
                                                            .map(|priority| priority
                                                                .label()
                                                                .to_string())
                                                            .unwrap_or_else(|| "All".to_string())
                                                    ),
                                                    self.priority_filter.is_some(),
                                                ))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let suppress = std::mem::take(
                                                        &mut this.priority_closed_on_up,
                                                    );
                                                    this.priority_menu_open =
                                                        !this.priority_menu_open && !suppress;
                                                    this.sort_menu_open = false;
                                                    this.group_menu_open = false;
                                                    this.sidebar_menu = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .on_children_prepainted(move |bounds, _, _| {
                                            if let Some(bounds) = bounds.first() {
                                                anchor.set(Some(*bounds));
                                            }
                                        })
                                })
                            })
                            .when(shows_completed && !is_trash, |this| {
                                this.child(
                                    div()
                                        .id("show-completed")
                                        .flex_shrink_0()
                                        .flow_tip(self, cx, theme, "Show or hide completed tasks")
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
                            .when(is_trash, |this| {
                                this.child(
                                    div()
                                        .id("trash-select-all")
                                        .flex_shrink_0()
                                        .flow_tip(self, cx, theme, "Select every task in Trash")
                                        .child(widgets::button(theme, "Select all", false))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.select_all_trash(cx)),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("trash-restore")
                                        .flex_shrink_0()
                                        .flow_tip(self, cx, theme, "Restore selected tasks")
                                        .child(widgets::button(theme, "Restore", false))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.restore_selected(cx)),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("trash-purge")
                                        .flex_shrink_0()
                                        .flow_tip(
                                            self,
                                            cx,
                                            theme,
                                            "Permanently delete selected tasks",
                                        )
                                        .child(widgets::button(theme, "Delete forever", false))
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.purge_selected(cx)),
                                        ),
                                )
                            }),
                    )
                    .on_children_prepainted({
                        // The first child is the title slot. Its laid-out width
                        // is exactly what is left beside the chips, so record it
                        // and ask for one more frame when the window or the
                        // labels resize.
                        let title_space = self.title_space.clone();
                        move |bounds, window, _| {
                            if let Some(slot) = bounds.first() {
                                let width = f32::from(slot.size.width);
                                if title_space.get() != Some(width) {
                                    title_space.set(Some(width));
                                    window.request_animation_frame();
                                }
                            }
                        }
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .text_sm()
                    .text_color(theme.text_faint)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(if library_search {
                        "Matches from the whole library".to_string()
                    } else {
                        self.view_subtitle()
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .when(
                        self.view.searches_tasks() && !self.search.is_empty(),
                        |this| {
                            this.child(
                                div()
                                    .id("search-scope")
                                    .flex_shrink_0()
                                    .flow_tip(
                                        self,
                                        cx,
                                        theme,
                                        if self.search_library {
                                            "Searching the whole library; click for this view (⌘⇧F)"
                                        } else {
                                            "Searching this view; click for the whole library (⌘⇧F)"
                                        },
                                    )
                                    .child(widgets::toggle_chip(
                                        theme,
                                        if self.search_library {
                                            "Library"
                                        } else {
                                            "This view"
                                        },
                                        self.search_library,
                                    ))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.search_library = !this.search_library;
                                        cx.notify();
                                    })),
                            )
                        },
                    )
                    .child(view_count_label(is_archive, count, forecast_day)),
            )
    }

    /// A real list of sort choices: the chip opens a menu instead of
    /// silently cycling, so the user can jump straight to the order they want.
    pub(super) fn render_sort_menu(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = self.theme;
        let mut menu = div()
            .min_w(px(150.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg();
        let options: [Option<SortBy>; 10] = [
            None,
            Some(SortBy::Manual),
            Some(SortBy::Due),
            Some(SortBy::Defer),
            Some(SortBy::Planned),
            Some(SortBy::Name),
            Some(SortBy::Flagged),
            Some(SortBy::Estimate),
            Some(SortBy::Created),
            Some(SortBy::CompletedAt),
        ];
        for option in options {
            let selected = self.sort == option;
            let slug = option.map(sort_slug).unwrap_or("default");
            menu = menu.child(
                div()
                    .id(super::sid(format!("sort-option:{slug}")))
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
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(sort_label(option))
                    .when(selected, |this| {
                        this.child(icons::icon(Icon::Check, 11.0, theme.accent))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_sort(option, cx))),
            );
        }
        menu
    }

    pub(super) fn render_group_menu(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = self.theme;
        let mut menu = div()
            .min_w(px(150.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg();
        let options: [Option<GroupBy>; 10] = [
            None,
            Some(GroupBy::Project),
            Some(GroupBy::Folder),
            Some(GroupBy::Tag),
            Some(GroupBy::DueDate),
            Some(GroupBy::DeferDate),
            Some(GroupBy::PlannedDate),
            Some(GroupBy::Flagged),
            Some(GroupBy::Estimate),
            Some(GroupBy::None),
        ];
        for option in options {
            let selected = self.group == option;
            let slug = option.map(group_slug).unwrap_or("default");
            menu = menu.child(
                div()
                    .id(super::sid(format!("group-option:{slug}")))
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
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(group_label(option))
                    .when(selected, |this| {
                        this.child(icons::icon(Icon::Check, 11.0, theme.accent))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_group(option, cx))),
            );
        }
        menu
    }

    pub(super) fn render_priority_menu(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = self.theme;
        let mut menu = div()
            .min_w(px(140.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg();
        let options: [Option<Priority>; 4] = [
            None,
            Some(Priority::High),
            Some(Priority::Medium),
            Some(Priority::Low),
        ];
        for option in options {
            let selected = self.priority_filter == option;
            let (slug, label) = match option {
                None => ("all", "All priorities".to_string()),
                Some(priority) => (priority.slug(), priority.label().to_string()),
            };
            menu = menu.child(
                div()
                    .id(super::sid(format!("priority-option:{slug}")))
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
                    } else {
                        theme.clear
                    })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(label)
                    .when(selected, |this| {
                        this.child(icons::icon(Icon::Check, 11.0, theme.accent))
                    })
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.choose_priority_filter(option, cx)),
                    ),
            );
        }
        menu
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
                            "Add a task…  #tag  ::Project  fsr:1h  due:tomorrow  est:15m",
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

    fn render_groups(&self, groups: &[Group], cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let mut list = div().flex().flex_col().w_full().py_1();
        let show_headers = groups.len() > 1
            || matches!(
                self.view,
                View::Builtin(BuiltinPerspective::Projects) | View::Folder(_) | View::Custom(_)
            );
        // Gaps before root rows accept a dragged subtask in any list: the
        // task lands in the same project (or Inbox) as the row below the gap.
        // The end gap only makes sense in a project or Inbox, where "the end"
        // is well defined.
        let end_gap = matches!(
            self.view,
            View::Project(_) | View::Builtin(BuiltinPerspective::Inbox)
        );
        // A view with a single empty group is really an empty view: give it
        // the same illustrated empty state instead of a lonely caption.
        let entirely_empty = groups.len() == 1 && groups[0].rows.is_empty() && !show_headers;
        if groups.is_empty() || entirely_empty {
            if !self.search.is_empty() {
                let (title, detail) = if self.search_library {
                    (
                        "No matches",
                        "Nothing in the library matches that. ⌘⇧F searches this view instead.",
                    )
                } else {
                    (
                        "No matches",
                        "Nothing in this view matches. ⌘⇧F searches the whole library.",
                    )
                };
                return self.render_empty(title, detail).into_any_element();
            }
            let (title, detail) = match &self.view {
                View::Builtin(BuiltinPerspective::Inbox) => (
                    "Inbox zero",
                    "A clean slate. Capture something above and it lands here.",
                ),
                View::Builtin(BuiltinPerspective::Today) => (
                    "Nothing planned or due today",
                    if self.hide_past {
                        "Past plans and overdue tasks are hidden."
                    } else {
                        "A clear day. Capture something above when it comes up."
                    },
                ),
                View::Builtin(BuiltinPerspective::Flagged) => (
                    "Nothing flagged",
                    "Flag a task with ! and it will wait here.",
                ),
                View::Builtin(BuiltinPerspective::Tags) => (
                    "No tags yet",
                    "Add #tag to a task and the tag appears here.",
                ),
                View::Builtin(BuiltinPerspective::Nearby) => (
                    "Nothing nearby",
                    "Give a tag a location and tasks around it show up here.",
                ),
                View::Builtin(BuiltinPerspective::Archive) => (
                    "No archived projects",
                    "Finished projects can be archived from the project menu.",
                ),
                View::Builtin(BuiltinPerspective::Changed) => (
                    "Nothing changed recently",
                    "Edits from the last few days collect here.",
                ),
                View::Builtin(BuiltinPerspective::Attachments) => (
                    "No attachments",
                    "Add a file or link from the inspector and it will appear here.",
                ),
                View::Builtin(BuiltinPerspective::Trash) => (
                    "Trash is empty",
                    "Deleted tasks wait here until you restore or empty them.",
                ),
                View::Builtin(BuiltinPerspective::Completed) => (
                    "Nothing completed yet",
                    "Finish a task with x and it will wait here.",
                ),
                _ => (
                    "Nothing here",
                    "Capture something above, or switch perspective from the sidebar.",
                ),
            };
            return self.render_empty(title, detail).into_any_element();
        }
        for group in groups {
            let open = self.is_open(&group.key, super::group_default_open(&group.key));
            if show_headers {
                list = list.child(self.render_group_header(group, open, cx));
            }
            if open {
                // One root drop target before the first top-level row. Rows
                // themselves handle "after this row", so no spacer is needed
                // between every pair and the rows stay visually joined.
                let mut first_root = true;
                for row in self.expanded_rows(&group.rows) {
                    if self.row_is_hidden(&row.id) {
                        continue;
                    }
                    if row.depth == 0 && first_root {
                        list = list.child(
                            self.render_root_drop_zone(Some(row.id.clone()), cx),
                        );
                        first_root = false;
                    }
                    list = list.child(self.render_task_row(&row, cx));
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
        if end_gap {
            list = list.child(self.render_root_drop_zone(None, cx));
        }
        list.into_any_element()
    }

    /// The rows a group paints. A parent that matches the view but whose
    /// children do not (Today, Flagged, a search) still reveals those children
    /// when its disclosure is open, so the arrow always does something.
    fn expanded_rows(&self, rows: &[TaskRow]) -> Vec<TaskRow> {
        fn add(
            app: &FlowlistApp,
            db: &Db,
            at: chrono::DateTime<chrono::Utc>,
            id: &TaskId,
            seeded: bool,
            out: &mut Vec<TaskRow>,
            seen: &mut BTreeSet<TaskId>,
        ) {
            if !seen.insert(id.clone()) {
                return;
            }
            let Some(task) = db.tasks.get(id) else {
                return;
            };
            // Rows handed in by the view already passed its filter: the
            // Completed and Trash perspectives put completed or trashed tasks
            // there on purpose. Only children revealed by expanding a branch
            // need the completed-work filter.
            if !seeded {
                let keep_completed =
                    app.show_completed || app.keep_completed_subtasks;
                if task.is_trashed() || (task.completed && !keep_completed) {
                    return;
                }
            }
            out.push(query::row_for(db, id, at));
            if app.is_open(&format!("task:{id}"), false) {
                for child in &task.child_order {
                    add(app, db, at, child, false, out, seen);
                }
            }
        }

        let db = self.store.db();
        let at = now();
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for row in rows {
            add(self, db, at, &row.id, true, &mut out, &mut seen);
        }
        out
    }

    /// A thin root-level gap that accepts a dragged task. It only shows a
    /// line while the pointer is over it, so it costs almost no space.
    fn render_root_drop_zone(
        &self,
        before: Option<TaskId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let key = match &before {
            Some(id) => format!("root-before:{id}"),
            None => "root-end".to_string(),
        };
        let active = self.content_drop_target.as_deref() == Some(key.as_str());
        let hover_key = key.clone();
        let move_before = before.clone();
        div()
            .id(super::sid(key))
            .h(px(7.))
            .w_full()
            .flex()
            .items_center()
            .child(
                div()
                    .h(px(2.))
                    .w_full()
                    .when(active, |this| this.bg(theme.accent)),
            )
            .on_drag_move::<DragTask>(cx.listener(
                move |this, event: &DragMoveEvent<DragTask>, _, cx| {
                    if event.bounds.contains(&event.event.position) {
                        this.set_content_drop_target(Some(hover_key.clone()), cx);
                    }
                },
            ))
            .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                this.drop_on_root(move_before.clone(), payload, cx)
            }))
            .into_any_element()
    }

    /// Record where an Archive row or its Reopen button painted, so a probe
    /// can open or revive a finished project without pixel guessing.
    fn archive_probe(
        &self,
        key: String,
    ) -> impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.archive_boxes.clone();
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

    /// The Archive lists finished and dropped projects rather than tasks,
    /// grouped by the folder they lived in. Each row opens the project, and
    /// its Reopen button sets the project back to Active through the normal
    /// command, so the change is undoable.
    fn render_archive(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let db = self.store.db();
        if query::archived_projects(db).is_empty() {
            return self
                .render_empty(
                    "No finished projects",
                    "Mark a project Done or Dropped and it will wait here.",
                )
                .into_any_element();
        }
        let mut list = div().flex().flex_col().w_full().py_1();
        for (key, title, projects) in self.archive_sections() {
            let open = self.is_open(&key, true);
            let toggle_key = key.clone();
            let header = div()
                .id(super::sid(format!("archive-group:{key}")))
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
                .child(icons::icon(
                    if open {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    },
                    12.0,
                    theme.text_faint,
                ))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_dim)
                        .child(title),
                )
                .child(widgets::count(theme, projects.len()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_group_collapsed(toggle_key.clone(), cx)
                }));
            list = list.child(header);
            if open {
                for id in projects {
                    list = list.child(self.archive_project_row(&id, cx));
                }
            }
        }
        list.into_any_element()
    }

    /// Finished projects grouped by their folder, in library order, with root
    /// projects under "Library". Shared by the Archive surface and the test
    /// state, so what a probe sees is what the pane paints.
    pub(super) fn archive_sections(&self) -> Vec<(String, String, Vec<ProjectId>)> {
        let db = self.store.db();
        let mut groups: Vec<(String, String, Vec<ProjectId>)> = Vec::new();
        for id in query::archived_projects(db) {
            let Some(project) = db.projects.get(&id) else {
                continue;
            };
            let (key, title) = match &project.folder {
                Some(folder) => (format!("archive:folder:{folder}"), db.folder_path(folder)),
                None => ("archive:root".to_string(), "Library".to_string()),
            };
            match groups
                .iter_mut()
                .find(|(candidate, _, _)| candidate == &key)
            {
                Some((_, _, projects)) => projects.push(id),
                None => groups.push((key, title, vec![id])),
            }
        }
        groups
    }

    fn archive_project_row(&self, id: &ProjectId, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let Some(project) = db.projects.get(id) else {
            return div().into_any_element();
        };
        let (done, total) = db.project_progress(id);
        let status = project.status;
        let status_color = match status {
            ProjectStatus::Done => theme.success,
            _ => theme.text_faint,
        };
        let open_id = id.clone();
        let reopen_id = id.clone();
        let delete_id = id.clone();
        let row_key = format!("project:{id}");
        let reopen_key = format!("reopen:{id}");
        let delete_key = format!("delete:{id}");
        div()
            .on_children_prepainted(self.archive_probe(row_key))
            .child(
                div()
                    .id(super::sid(format!("archive-project:{id}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .h(px(38.))
                    .mx_2()
                    .px_3()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(widgets::icon_spec(
                        &project.icon,
                        15.0,
                        theme.text_faint,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_sm()
                            .text_color(theme.text)
                            .child(project.name.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child(format!("{done} of {total} done")),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(status_color)
                            .child(status.label()),
                    )
                    .child(
                        div()
                            .on_children_prepainted(self.archive_probe(reopen_key))
                            .child(
                                div()
                                    .id(super::sid(format!("archive-reopen:{id}")))
                                    .flex_shrink_0()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .text_xs()
                                    .text_color(theme.accent)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.selection))
                                    .flow_tip(self, cx, theme, "Reopen this project (undoable)")
                                    .child("Reopen")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        // Reopen sits inside the row's click
                                        // target; keep it from also opening
                                        // the project.
                                        cx.stop_propagation();
                                        this.execute(Command::SetProjectStatus {
                                            id: reopen_id.clone(),
                                            status: ProjectStatus::Active,
                                        });
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .on_children_prepainted(self.archive_probe(delete_key))
                            .child(
                                div()
                                    .id(super::sid(format!("archive-delete:{id}")))
                                    .flex_shrink_0()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .cursor_pointer()
                                    .hover(|style| style.text_color(theme.danger).bg(theme.hover))
                                    .flow_tip(
                                        self,
                                        cx,
                                        theme,
                                        "Delete this project and its tasks (undoable)",
                                    )
                                    .child("Delete")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        // Reopen and Delete sit inside the row's
                                        // click target; keep them from also
                                        // opening the project.
                                        cx.stop_propagation();
                                        let modal = super::modals::Modal::delete_project(
                                            this,
                                            delete_id.clone(),
                                        );
                                        this.open_modal(modal, cx);
                                    })),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_view(View::Project(open_id.clone()), cx)
                    })),
            )
            .into_any_element()
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
        let group_boxes = self.content_group_boxes.clone();
        let group_key = group.key.clone();
        let mut row =
            div()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        group_boxes.borrow_mut().insert(
                            group_key.clone(),
                            [
                                f32::from(bounds.origin.x),
                                f32::from(bounds.origin.y),
                                f32::from(bounds.size.width),
                                f32::from(bounds.size.height),
                            ],
                        );
                    }
                })
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
        let painted_title = title.clone();
        let subtask_total = task
            .child_order
            .iter()
            .filter(|child| {
                db.tasks
                    .get(*child)
                    .is_some_and(|task| !task.is_trashed())
            })
            .count();
        let subtask_completed = task
            .child_order
            .iter()
            .filter(|child| {
                db.tasks
                    .get(*child)
                    .is_some_and(|task| !task.is_trashed() && task.completed)
            })
            .count();
        // The badge only counts children the list will actually paint. Done
        // subtasks count once they are kept visible; while they are filtered
        // out they must not turn an empty branch into "[2/0]".
        let shows_done = self.show_completed || self.keep_completed_subtasks;
        let subtask_done = if shows_done { subtask_completed } else { 0 };
        let subtask_open = subtask_total - subtask_completed;
        // [done/total] over the rows that are actually shown, so two visible
        // finished subtasks read "[2/2]", never "[2/0]".
        let subtask_visible = subtask_done + subtask_open;
        let has_children = subtask_visible > 0;
        // Subtasks start collapsed; the disclosure opens a branch. Expanding
        // stays view state, so sorting or grouping never changes it.
        let expanded = self.is_open(&format!("task:{id}"), false);
        let estimate = task.estimate_min;
        let attachment_count = task.attachments.len();
        // A task whose attachments are all links reads better with a chain
        // than a paperclip; a mixed list keeps the paperclip as the umbrella.
        let attachment_links = attachment_count > 0
            && task
                .attachments
                .iter()
                .all(|value| flowlist_core::model::attachment::is_web_link(value));
        let due = task.due;
        let defer = task.defer;
        let planned = task.planned;
        let tags: Vec<String> = task
            .tags
            .iter()
            .filter_map(|tag| db.tags.get(tag).map(|tag| db.tag_path(&tag.id)))
            .take(3)
            .collect();
        let project = row_project_label(db, &self.view, task.project.as_ref());
        let availability = row.availability.clone();
        let title_id = id.clone();
        let checkbox_id = id.clone();
        let flag_id = id.clone();
        let drop_id = id.clone();
        let disclosure_id = id.clone();
        let drag_payload = DragTask::task(id.clone(), title.clone());
        let disclosure_anchor = self.task_disclosures.clone();
        let disclosure_key = id.to_string();
        let due_color = due
            .map(|stamp| due_color(stamp, &theme))
            .unwrap_or(theme.text_faint);
        // Hover reveals the quiet row actions without leaving a ghost icon
        // in every line of the list.
        let row_group = format!("task-row:{id}");
        let title_color = if completed {
            theme.text_faint
        } else if availability.is_available() {
            theme.text
        } else {
            theme.text_dim
        };

        let root_key = format!("root-after:{id}");
        let root_after = self.content_drop_target.as_deref() == Some(root_key.as_str());
        let root_id = id.clone();
        let drop_root_id = id.clone();
        let row_depth = row.depth;
        let title_line = div()
            .flex_shrink()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_color(title_color)
            .when(completed, |this| this.line_through())
            .child(title);
        let meta_line = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    // Right-edge metadata reads in a fixed order: what is
                    // attached, where it lives, what it is tagged, how long it
                    // takes, when it is due, then the softer signals.
                    .when(subtask_visible > 0, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(format!("[{subtask_done}/{subtask_visible}]")),
                        )
                    })
                    .when(attachment_count > 0, |this| {
                        this.child(
                            div()
                                .id(super::sid(format!("attachments:{}", row.id)))
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .flow_tip(
                                    self,
                                    cx,
                                    theme,
                                    format!(
                                        "{attachment_count} {}{}",
                                        if attachment_links {
                                            "link"
                                        } else {
                                            "attachment"
                                        },
                                        if attachment_count == 1 { "" } else { "s" }
                                    ),
                                )
                                .child(icons::icon(
                                    if attachment_links {
                                        Icon::Link
                                    } else {
                                        Icon::Paperclip
                                    },
                                    11.0,
                                    theme.text_faint,
                                ))
                                .child(attachment_count.to_string()),
                        )
                    })
                    .when_some(project, |this, project| {
                        this.child(div().text_xs().text_color(theme.text_faint).child(project))
                    })
                    .children(tags.into_iter().map(|tag| widgets::tag_chip(theme, tag)))
                    .when_some(estimate, |this, minutes| {
                        this.child(widgets::chip(
                            theme,
                            format_duration(minutes),
                            theme.text_faint,
                        ))
                    })
                    .when_some(planned, |this, stamp| {
                        let label = human_date(&stamp, now());
                        this.child(
                            div()
                                .id(super::sid(format!("planned:{}", row.id)))
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .flow_tip(
                                    self,
                                    cx,
                                    theme,
                                    format!("Planned {label}"),
                                )
                                .child(icons::icon(
                                    Icon::PlannedDate,
                                    11.0,
                                    theme.text_faint,
                                ))
                                .child(label),
                        )
                    })
                    .when_some(due, |this, stamp| {
                        let label = human_date(&stamp, now());
                        this.child(
                            div()
                                .id(super::sid(format!("due:{}", row.id)))
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_xs()
                                .text_color(due_color)
                                .flow_tip(self, cx, theme, format!("Due {label}"))
                                .child(icons::icon(Icon::DueDate, 11.0, due_color))
                                .child(label),
                        )
                    })
                    .when_some(defer.filter(|stamp| stamp.at > now()), |this, stamp| {
                        this.child(widgets::chip_with_icon(
                            theme,
                            Icon::DeferDate,
                            human_date(&stamp, now()),
                            theme.text_faint,
                        ))
                    })
                    .when_some(repeat_label(db, &id), |this, label| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(icons::icon(Icon::Repeat, 11.0, theme.text_faint))
                                .child(label),
                        )
                    })
                    .when(!availability.is_available() && !completed, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.warning)
                                .child(availability.label()),
                        )
                    });
        let flag_color = if flagged {
            super::theme::priority_color(theme, Priority::High)
        } else {
            theme.text_faint
        };
        let flag = div()
            .id(super::sid(format!("flag:{}", flag_id)))
            .w(px(12.))
            // With a chevron, its hit area carries the one-circle gap. Without
            // one, keep the same gap before the flag.
            .when(!has_children, |this| this.ml_2())
            .text_color(flag_color)
            .opacity(if flagged { 1.0 } else { 0.0 })
            .cursor_pointer()
            .group_hover(row_group.clone(), |style| style.opacity(1.0))
            .hover(|style| style.opacity(1.0))
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
            .child(icons::icon(Icon::Flagged, 12.0, flag_color))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.toggle_flag(flag_id.clone(), cx);
            }));
        let disclosure = has_children.then(|| {
            div()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        disclosure_anchor.borrow_mut().insert(
                            disclosure_key.clone(),
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
                        .id(super::sid(format!("disclosure:{}", disclosure_id)))
                        .flex()
                        .items_center()
                        .gap_1()
                        // The gap before the flag is part of the toggle target,
                        // so a small chevron is not a pixel hunt.
                        .pr(px(16.))
                        .text_color(theme.text_faint)
                        .flow_tip(
                            self,
                            cx,
                            theme,
                            if expanded {
                                "Collapse subtasks"
                            } else {
                                "Expand subtasks"
                            },
                        )
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
                            this.set_branch_open(
                                format!("task:{}", disclosure_id),
                                !expanded,
                            );
                            cx.notify();
                        })),
                )
                .into_any_element()
        });
        let first_line = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_grow()
            .flex_shrink()
            .min_w_0()
            .overflow_hidden()
            .child(
                div()
                    .id(super::sid(format!("check:{}", checkbox_id)))
                    .flow_tip(
                        self,
                        cx,
                        theme,
                        if completed {
                            "Reopen task (x)"
                        } else {
                            "Complete task (x)"
                        },
                    )
                    .child(widgets::checkbox(
                        theme,
                        completed,
                        theme.accent,
                        task.priority
                            .map(|priority| super::theme::priority_color(theme, priority)),
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_complete(checkbox_id.clone(), cx);
                    })),
            )
            .child(title_line)
            // The chevron belongs to the title, so it comes first; the flag
            // follows it before the row's spare space.
            .when_some(disclosure, |this, disclosure| this.child(disclosure))
            .child(flag)
            .child(div().flex_grow());
        let content = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap_2()
            .child(first_line)
            .child(meta_line);
        let base = div()
            .id(super::sid(format!("task:{}", id)))
            .group(row_group.clone())
            .flex()
            .items_start()
            .gap_2()
            .min_h(px(32.))
            .w_full()
            .pl(px(18.0 + row.depth as f32 * 18.0))
            .pr_4()
            .py_1()
            .border_b_1()
            .border_color(theme.border_soft)
            .bg(if root_after {
                theme.accent_soft
            } else if selected {
                theme.selection
            } else {
                theme.bg
            })
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
            .on_drag_move::<DragTask>(cx.listener(
                move |this, event: &DragMoveEvent<DragTask>, _, cx| {
                    // The bottom of a root row is a large, forgiving zone for
                    // moving a subtask back out to the project root.
                    let bottom = event.bounds.origin.y + event.bounds.size.height;
                    let near_bottom = row_depth == 0
                        && event.event.position.y > bottom - event.bounds.size.height * 0.30;
                    let target = near_bottom.then(|| format!("root-after:{root_id}"));
                    this.set_content_drop_target(target, cx);
                },
            ))
            .on_drop::<DragTask>(cx.listener(move |this, payload: &DragTask, _, cx| {
                let key = format!("root-after:{drop_root_id}");
                if this.content_drop_target.as_deref() == Some(key.as_str()) {
                    this.drop_on_root(Some(drop_root_id.clone()), payload, cx);
                } else {
                    this.drop_on_task(drop_id.clone(), payload, cx);
                }
            }))
            .child(content);
        self.content_rows.borrow_mut().push((
            id.to_string(),
            row.depth,
            painted_title,
            has_children,
        ));
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
                            .child(widgets::icon_spec(
                                &project.icon,
                                15.0,
                                theme.text_dim,
                            ))
                            .child(widgets::progress(theme, done, total))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child(format!("{done}/{total}")),
                            ),
                    )
                    .child({
                        let edit_id = id.clone();
                        div()
                            .id("banner-review-interval")
                            .flex()
                            .flex_1()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .text_color(theme.text_dim)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .flow_tip(self, cx, theme, "Change how often this project is reviewed")
                            .child(format!(
                                "Review {} · every {} day(s)",
                                next_review.format("%b %-d"),
                                interval
                            ))
                            .child(icons::icon(Icon::ChevronRight, 10.0, theme.text_faint))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let modal =
                                    super::modals::Modal::review_interval(this, edit_id.clone());
                                this.open_modal(modal, cx);
                            }))
                    })
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
                            .id("project-kind")
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, "Click to change the project type")
                            .child(widgets::chip(
                                theme,
                                project.effective_kind().label(),
                                theme.text_dim,
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let View::Project(id) = this.view.clone() {
                                    if let Some(project) = this.store.db().projects.get(&id) {
                                        let kind = project.effective_kind().next();
                                        this.execute(Command::UpdateProject {
                                            id: id.clone(),
                                            patch: ProjectPatch {
                                                kind: Some(kind),
                                                ..Default::default()
                                            },
                                        });
                                    }
                                }
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
                                    .min_w_0()
                                    .line_clamp(1)
                                    .text_ellipsis()
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
                            ))
                            .child(
                                div()
                                    .id("review-now")
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

/// The content pane's width in logical pixels. The Forecast calendar uses it
/// to choose between one and two month grids.
/// Record a painted box for the GUI probe, keyed by the thing it represents.
fn record_box(
    sink: &Rc<RefCell<BTreeMap<String, [f32; 4]>>>,
    key: &str,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) {
    let Some(bounds) = bounds else {
        return;
    };
    sink.borrow_mut().insert(
        key.to_string(),
        [
            f32::from(bounds.origin.x),
            f32::from(bounds.origin.y),
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
        ],
    );
}

/// Everything a month grid paints around its week rows: the heading, the
/// weekday labels and the resize grip. Used to turn a dragged block height into
/// a per-row height.
const FORECAST_CALENDAR_CHROME: f32 = 62.0;
const FORECAST_CELL_MIN: f32 = 44.0;
const FORECAST_CELL_MAX: f32 = 160.0;

fn forecast_pane_width(window_width: f32, show_inspector: bool) -> f32 {
    (window_width
        - super::SIDEBAR_WIDTH
        - if show_inspector {
            super::INSPECTOR_WIDTH
        } else {
            0.0
        })
    .max(0.0)
}

/// The count under a view title. A picked Forecast day names the day, so the
/// label reads "1 task for 23 Nov"; with no day picked it stays "3 tasks".
fn view_count_label(
    is_archive: bool,
    count: usize,
    forecast_day: Option<NaiveDate>,
) -> String {
    if is_archive {
        if count == 0 {
            "No projects".to_string()
        } else {
            super::plural(count, "project", "projects")
        }
    } else if let Some(date) = forecast_day {
        if count == 0 {
            format!("No tasks for {}", date.format("%-d %b"))
        } else {
            format!(
                "{} for {}",
                super::plural(count, "task", "tasks"),
                date.format("%-d %b")
            )
        }
    } else if count == 0 {
        "No tasks".to_string()
    } else {
        super::plural(count, "task", "tasks")
    }
}

/// The first day of the month `delta` months after `month`. The Forecast month
/// is always the first of a month, so this only needs to wrap the year.
pub(super) fn add_months(month: NaiveDate, delta: i32) -> NaiveDate {
    let mut year = month.year();
    let mut month_number = month.month() as i32 + delta;
    while month_number < 1 {
        month_number += 12;
        year -= 1;
    }
    while month_number > 12 {
        month_number -= 12;
        year += 1;
    }
    NaiveDate::from_ymd_opt(year, month_number as u32, 1).unwrap_or(month)
}

/// Monday of the week containing `date`. Forecast's strip and month grid both
/// use a Monday-first week.
pub(super) fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

/// The Monday-first grid for a month: the first visible date, how many days
/// the month has, and how many week rows the grid needs.
pub(super) fn month_grid(month: NaiveDate) -> (NaiveDate, i64, usize) {
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

/// The project label a row shows. A project view already names its project,
/// and a folder view already names its folder, so only the remaining path is
/// worth repeating there.
fn row_project_label(db: &Db, view: &View, project: Option<&ProjectId>) -> Option<String> {
    let project = project?;
    match view {
        View::Builtin(BuiltinPerspective::Projects) | View::Project(_) => None,
        View::Folder(folder) => {
            let path = db.project_path(project);
            let prefix = format!("{} / ", db.folder_path(folder));
            Some(path.strip_prefix(&prefix).unwrap_or(&path).to_string())
        }
        _ => Some(db.project_path(project)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_rows_drop_the_folder_prefix() {
        let mut db = Db::new();
        let project = db.ensure_project_path("Personal/Shopping").unwrap();
        let folder = db.ensure_folder_path("Personal").unwrap();
        assert_eq!(
            row_project_label(&db, &View::Folder(folder), Some(&project)).as_deref(),
            Some("Shopping")
        );
        assert_eq!(
            row_project_label(
                &db,
                &View::Builtin(BuiltinPerspective::Today),
                Some(&project)
            )
            .as_deref(),
            Some("Personal / Shopping")
        );
        assert_eq!(
            row_project_label(&db, &View::Project(project.clone()), Some(&project)),
            None
        );
    }

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

    #[test]
    fn add_months_wraps_the_year() {
        let november = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
        assert_eq!(
            add_months(november, 1),
            NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()
        );
        assert_eq!(
            add_months(november, 3),
            NaiveDate::from_ymd_opt(2027, 2, 1).unwrap()
        );
        assert_eq!(
            add_months(november, -11),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap()
        );
        assert_eq!(
            add_months(november, 0),
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
        );
    }

    #[test]
    fn view_count_label_names_a_picked_forecast_day() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        assert_eq!(view_count_label(false, 1, Some(day)), "1 task for 23 Nov");
        assert_eq!(view_count_label(false, 3, Some(day)), "3 tasks for 23 Nov");
        assert_eq!(view_count_label(false, 0, Some(day)), "No tasks for 23 Nov");
        assert_eq!(view_count_label(false, 3, None), "3 tasks");
        assert_eq!(view_count_label(false, 0, None), "No tasks");
        assert_eq!(view_count_label(true, 2, Some(day)), "2 projects");
    }
}
