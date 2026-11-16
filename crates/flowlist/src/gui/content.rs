//! The main task list, project dashboards, review queue and nearby places.

use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate};
use gpui::{
    AnyElement, Context, IntoElement, MouseButton, SharedString, TextRun, Window, div, font,
    prelude::*, px,
};

use flowlist_core::command::{Command, ProjectPatch, SettingsPatch};
use flowlist_core::ids::{ProjectId, TaskId};
use flowlist_core::model::{
    BuiltinPerspective, GroupBy, Priority, ProjectStatus, RepeatRule, SortBy,
};
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::time::{Stamp, format_duration, human_date, local_date, now};

use super::drag::{DragTask, ghost};
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
        let groups = self.current_groups();
        // Rebuilt as rows paint, so the test state lists only what is on
        // screen, including the subtasks an expanded parent reveals.
        self.content_rows.borrow_mut().clear();

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
            .child(self.render_content_header(window, cx))
            .when(self.view.shows_capture(), |this| {
                this.child(self.render_capture_bar(cx))
            });

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
            if task.completed || task.is_trashed() {
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
        self.selected.clear();
        self.anchor = None;
        self.save_session();
    }

    /// Move the Forecast calendar one month forward or back.
    pub(super) fn forecast_shift_month(&mut self, delta: i32) {
        self.forecast_month = add_months(self.forecast_month, delta);
    }

    /// Cycle the calendar between Auto, one month and two months. The choice is
    /// a library setting, so the CLI and a second window see the same layout.
    pub(super) fn cycle_forecast_months(&mut self, cx: &mut Context<Self>) {
        let next = match self.store.db().settings.forecast_months.min(2) {
            0 => 1,
            1 => 2,
            _ => 0,
        };
        self.execute(Command::UpdateSettings(SettingsPatch {
            forecast_months: Some(next),
            ..Default::default()
        }));
        cx.notify();
    }

    /// Switch the calendar between calendar-sized cells and the full pane.
    pub(super) fn toggle_forecast_width(&mut self, cx: &mut Context<Self>) {
        let full = !self.store.db().settings.forecast_full_width;
        self.execute(Command::UpdateSettings(SettingsPatch {
            forecast_full_width: Some(full),
            ..Default::default()
        }));
        cx.notify();
    }

    /// The month grid above the Forecast list. Monday-first, with an accent
    /// bar under every date that has a task. The Months and Width chips pick
    /// how many grids paint and whether they fill the pane; in Fixed mode the
    /// block is capped at calendar size and centred rather than widened.
    fn render_forecast_calendar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let today = local_date(now());
        let first = self.forecast_month;
        let marked = self.forecast_marked_dates();
        let window_width = self.window_size.map_or(1254.0, |(width, _)| width);
        let pane_width = forecast_pane_width(window_width, self.show_inspector);
        let configured_months = self.store.db().settings.forecast_months.min(2);
        let months = forecast_month_count(window_width, self.show_inspector, configured_months);
        let full_width = self.store.db().settings.forecast_full_width;
        let compact = pane_width < 520.0;
        let calendar_anchor = self.forecast_calendar_anchor.clone();
        let months_anchor = self.forecast_months_anchor.clone();
        let width_anchor = self.forecast_width_anchor.clone();

        let mut panels = div().flex().w_full().gap_6();
        for slot in 0..months {
            let month = add_months(first, slot as i32);
            panels = panels.child(self.render_forecast_month(month, slot, &marked, today, cx));
        }

        let month_label = match (configured_months, compact) {
            (0, true) => "Auto".to_string(),
            (0, false) => format!("Auto · {months}"),
            (1, true) => "1 mo".to_string(),
            (1, false) => "1 month".to_string(),
            (_, true) => "2 mo".to_string(),
            (_, false) => "2 months".to_string(),
        };
        let month_tip = match configured_months {
            0 => "Calendar months: Auto (M)".to_string(),
            1 => "Calendar months: 1 (M)".to_string(),
            _ => "Calendar months: 2 (M)".to_string(),
        };
        let width_label = if full_width { "Full width" } else { "Fixed" };
        let width_tip = if full_width {
            "Calendar width: fills the pane (W)"
        } else {
            "Calendar width: calendar-sized cells (W)"
        };

        let display_controls = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            months_anchor.set(Some(*bounds));
                        }
                    })
                    .child(
                        div()
                            .id("forecast-months")
                            .flow_tip(self, cx, theme, month_tip)
                            .child(widgets::toggle_chip(
                                theme,
                                month_label,
                                configured_months != 0,
                            ))
                            .on_click(cx.listener(|this, _, _, cx| this.cycle_forecast_months(cx))),
                    ),
            )
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            width_anchor.set(Some(*bounds));
                        }
                    })
                    .child(
                        div()
                            .id("forecast-width")
                            .flow_tip(self, cx, theme, width_tip)
                            .child(widgets::toggle_chip(theme, width_label, full_width))
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_forecast_width(cx))),
                    ),
            );

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
                        cx.notify();
                    }))
                    .into_any_element()
            });
        }
        let controls = controls
            .child(
                div()
                    .id("forecast-prev")
                    .flow_tip(self, cx, theme, "Previous month")
                    .child(widgets::button(theme, "‹", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_shift_month(-1);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("forecast-today")
                    .flow_tip(self, cx, theme, "Jump to this month")
                    .child(widgets::button(theme, "Today", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let today = local_date(now());
                        if let Some(first) = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                        {
                            this.forecast_month = first;
                            cx.notify();
                        }
                    })),
            )
            .child(
                div()
                    .id("forecast-next")
                    .flow_tip(self, cx, theme, "Next month")
                    .child(widgets::button(theme, "›", false))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.forecast_shift_month(1);
                        cx.notify();
                    })),
            );

        let title = div()
            .text_sm()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child(forecast_title(first, months));
        let header = if compact {
            // A narrow pane cannot fit the title, both display chips and the
            // month controls on one line, so the chips take their own row.
            div()
                .flex()
                .flex_col()
                .gap_1()
                .pb_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(title)
                        .child(controls),
                )
                .child(display_controls)
        } else {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .pb_1()
                .child(title)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(display_controls)
                        .child(controls),
                )
        };

        let calendar = div()
            .flex()
            .flex_col()
            .w_full()
            .gap_1()
            .child(header)
            .child(panels);
        let calendar = if full_width {
            calendar
        } else {
            calendar.max_w(px(forecast_max_width(months)))
        };

        div()
            .flex()
            .flex_col()
            .items_center()
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
            .child(calendar)
    }

    /// One month's weekday rail and date grid. `slot` keeps element ids unique
    /// when the neighbouring month also paints the same trailing dates.
    fn render_forecast_month(
        &self,
        first: NaiveDate,
        slot: usize,
        marked: &BTreeSet<NaiveDate>,
        today: NaiveDate,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
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
                let is_selected = self.forecast_date == Some(date);
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
                        .id(super::sid(format!("forecast-date:{slot}:{date}")))
                        .flex_1()
                        .h(px(38.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .when(is_selected, |this| this.bg(theme.selection))
                        .flow_tip(
                            self,
                            cx,
                            theme,
                            format!("Show tasks on {}", date.format("%b %-d")),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.forecast_date == Some(date) {
                                this.forecast_date = None;
                            } else {
                                this.forecast_date = Some(date);
                                this.forecast_month = add_months(date, 0);
                            }
                            this.selected.clear();
                            this.anchor = None;
                            cx.notify();
                        }))
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
            .flex_1()
            .min_w_0()
            .flex_col()
            .gap_1()
            .child(weekdays)
            .child(grid)
    }

    fn render_content_header(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let title = self.view_title();
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
        let count = self
            .current_groups()
            .into_iter()
            .map(|group| group.rows.len())
            .sum::<usize>();
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
                            .when(!is_trash, |this| {
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
                    .child(self.view_subtitle()),
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
        let options: [Option<SortBy>; 9] = [
            None,
            Some(SortBy::Manual),
            Some(SortBy::Due),
            Some(SortBy::Defer),
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
        let options: [Option<GroupBy>; 9] = [
            None,
            Some(GroupBy::Project),
            Some(GroupBy::Folder),
            Some(GroupBy::Tag),
            Some(GroupBy::DueDate),
            Some(GroupBy::DeferDate),
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
            if !self.is_open(&format!("task:{ancestor}"), true) {
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
        let painted_title = title.clone();
        let has_children = task.has_children();
        // Subtasks sit under their parent from the start, like an outline;
        // the disclosure collapses a branch instead of hiding it until asked.
        let expanded = self.is_open(&format!("task:{id}"), true);
        let estimate = task.estimate_min;
        let attachment_count = task.attachments.len();
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
        let disclosure_anchor = self.task_disclosures.clone();
        let disclosure_key = id.to_string();
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
                        if flagged { theme.warning } else { theme.accent },
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_complete(checkbox_id.clone(), cx);
                    })),
            )
            .child(if has_children {
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
                            .w(px(12.))
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
                                this.toggle_expanded(format!("task:{}", disclosure_id), true, cx);
                            })),
                    )
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
                        cx.stop_propagation();
                        this.toggle_flag(flag_id.clone(), cx);
                    })),
            )
            .when_some(task.priority, |this, priority| {
                this.child(
                    div()
                        .id(super::sid(format!("priority-mark:{}", row.id)))
                        .flex_shrink_0()
                        .w(px(3.))
                        .h(px(15.))
                        .rounded_full()
                        .bg(super::sidebar::priority_color(theme, priority))
                        .flow_tip(self, cx, theme, format!("{} priority", priority.label())),
                )
            })
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
                                        "{attachment_count} attachment{}",
                                        if attachment_count == 1 { "" } else { "s" }
                                    ),
                                )
                                .child(icons::icon(Icon::Paperclip, 11.0, theme.text_faint))
                                .child(attachment_count.to_string()),
                        )
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
                            .child(widgets::icon_for_project(
                                project.icon,
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

/// Two months once the pane can give each one enough room for readable cells.
/// A pinned one- or two-month choice overrides the width check; 0 means auto.
pub(super) fn forecast_month_count(
    window_width: f32,
    show_inspector: bool,
    configured: u8,
) -> usize {
    const TWO_MONTH_MIN: f32 = 620.0;
    match configured {
        1 => 1,
        2 => 2,
        _ if forecast_pane_width(window_width, show_inspector) >= TWO_MONTH_MIN => 2,
        _ => 1,
    }
}

/// The fixed-width calendar never grows past a comfortable month size: 520px
/// per month, plus the 24px gap the panels already use. Anything wider is
/// centred, so a full-screen window keeps calendar-sized cells instead of wide
/// slabs. Full-width mode skips this cap.
fn forecast_max_width(months: usize) -> f32 {
    let months = months.max(1) as f32;
    months * 520.0 + (months - 1.0) * 24.0
}

/// The calendar heading: one month, or a range when two are shown.
fn forecast_title(first: NaiveDate, months: usize) -> String {
    if months <= 1 {
        return first.format("%B %Y").to_string();
    }
    let last = add_months(first, months as i32 - 1);
    if first.year() == last.year() {
        format!(
            "{} – {} {}",
            first.format("%B"),
            last.format("%B"),
            first.format("%Y")
        )
    } else {
        format!("{} – {}", first.format("%B %Y"), last.format("%B %Y"))
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
    fn two_months_need_a_wide_enough_pane() {
        // The working window at 1254 shows two months with or without the
        // inspector; the outer panes leave 694 or 1014 pixels.
        assert_eq!(forecast_month_count(1254.0, true, 0), 2);
        assert_eq!(forecast_month_count(1254.0, false, 0), 2);
        // The minimum window with the inspector keeps one readable grid.
        assert_eq!(forecast_month_count(880.0, true, 0), 1);
        // Without the inspector the pane is wide enough for two.
        assert_eq!(forecast_month_count(880.0, false, 0), 2);
        // A mid-size window with the inspector is still single.
        assert_eq!(forecast_month_count(1000.0, true, 0), 1);
    }

    #[test]
    fn a_pinned_month_count_overrides_the_pane_width() {
        // One month even in a wide pane.
        assert_eq!(forecast_month_count(1600.0, false, 1), 1);
        // Two months in a pane that would otherwise keep one.
        assert_eq!(forecast_month_count(880.0, true, 2), 2);
        // Values outside the three choices fall back to automatic.
        assert_eq!(forecast_month_count(1254.0, false, 9), 2);
    }

    #[test]
    fn forecast_calendar_stops_widening_at_a_calendar_size() {
        assert_eq!(forecast_max_width(1), 520.0);
        // Two months plus the gap between the panels.
        assert_eq!(forecast_max_width(2), 1064.0);
    }

    #[test]
    fn forecast_title_spans_the_shown_months() {
        let november = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        assert_eq!(forecast_title(november, 1), "November 2026");
        assert_eq!(forecast_title(november, 2), "November – December 2026");
        let december = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        assert_eq!(forecast_title(december, 2), "December 2026 – January 2027");
    }
}
