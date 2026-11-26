//! The right-hand inspector: edit one task (or project) without leaving the
//! list. Every field commits through the same command layer as the CLI.

use chrono::{Datelike, Duration, Timelike};
use gpui::{
    AnyElement, Context, CursorStyle, IntoElement, MouseButton, Window, div, prelude::*, px,
};
use std::collections::BTreeSet;

use flowlist_core::command::{Command, TaskPatch};
use flowlist_core::ids::{ChecklistId, TaskId};
use flowlist_core::model::{
    AlertEvent, DefaultNotification, IconSpec, Priority, ProjectStatus, RepeatMode, RepeatRule,
    RepeatSchedule,
};
use flowlist_core::parse::{parse_duration, parse_stamp};
use flowlist_core::time::{IntervalUnit, Stamp, format_duration, human_date, local_date, now};

use super::content::month_grid;
use super::drag::{
    DragChecklist, DragInspectorScroll, DragTask, checklist_ghost, inspector_scroll_ghost,
};
use super::icons::Icon;
use super::widgets::TipExt;
use super::{
    BatchDraft, BatchField, ClockUnit, DateField, DatePickerStep, EditField, FlowlistApp,
    InspectorTab, NotificationTarget, View, icons, widgets,
};

/// A two-or-more choice row that keeps the OmniFocus segmented look without
/// introducing a second set of menu state.
fn segmented_row(
    theme: super::theme::Theme,
    id_prefix: &'static str,
    options: &[(&'static str, &'static str)],
    selected: usize,
    boxes: std::rc::Rc<std::cell::RefCell<std::collections::BTreeMap<String, [f32; 4]>>>,
    on_choose: impl Fn(&mut FlowlistApp, usize, &mut Context<FlowlistApp>) + 'static,
    cx: &mut Context<FlowlistApp>,
) -> AnyElement {
    let mut row = div().flex().items_center().gap_1();
    let on_choose = std::rc::Rc::new(on_choose);
    for (index, (slug, label)) in options.iter().enumerate() {
        let on_choose = on_choose.clone();
        let box_key = format!("{id_prefix}:{slug}");
        let boxes = boxes.clone();
        row = row.child(
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
                .id(super::sid(format!("{id_prefix}:{slug}")))
                .flex_1()
                .h(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .border_1()
                .border_color(if index == selected {
                    theme.accent
                } else {
                    theme.border
                })
                .bg(if index == selected {
                    theme.selection
                } else {
                    theme.field
                })
                .text_xs()
                .text_color(if index == selected {
                    theme.accent
                } else {
                    theme.text_dim
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child(*label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    on_choose(this, index, cx);
                })),
        );
    }
    row.into_any_element()
}

fn interval_unit_label(unit: IntervalUnit) -> &'static str {
    match unit {
        IntervalUnit::Days => "Days",
        IntervalUnit::Weeks => "Weeks",
        IntervalUnit::Months => "Months",
        IntervalUnit::Years => "Years",
    }
}

fn interval_unit_slug(unit: IntervalUnit) -> &'static str {
    match unit {
        IntervalUnit::Days => "days",
        IntervalUnit::Weeks => "weeks",
        IntervalUnit::Months => "months",
        IntervalUnit::Years => "years",
    }
}

fn review_step_days(days: u32) -> u32 {
    if days >= 7 && days % 7 == 0 {
        7
    } else {
        1
    }
}

fn review_interval_label(days: u32) -> String {
    if days >= 7 && days % 7 == 0 {
        let weeks = days / 7;
        if weeks == 1 {
            "1 week".to_string()
        } else {
            format!("{weeks} weeks")
        }
    } else if days == 1 {
        "1 day".to_string()
    } else {
        format!("{days} days")
    }
}

fn format_notification_time(at: chrono::DateTime<chrono::Utc>) -> String {
    at.with_timezone(&chrono::Local)
        .format("%b %-d, %Y, %-I:%M %p")
        .to_string()
}

fn relative_notification_label(
    alert: &flowlist_core::model::TaskAlert,
) -> (String, Option<String>) {
    let offset = alert.offset_minutes;
    let detail = if offset == 0 {
        None
    } else {
        let minutes = offset.unsigned_abs();
        let amount = if minutes % 60 == 0 {
            format!("{}h", minutes / 60)
        } else if minutes < 60 {
            format!("{minutes}m")
        } else {
            format!("{}h {}m", minutes / 60, minutes % 60)
        };
        Some(if offset < 0 {
            format!("{amount} before")
        } else {
            format!("{amount} after")
        })
    };
    let label = match alert.event {
        flowlist_core::model::AlertEvent::Due => {
            if offset < 0 {
                "Before Due"
            } else if offset == 0 {
                "Due"
            } else {
                "After Due"
            }
        }
        flowlist_core::model::AlertEvent::Defer => "Deferred",
        flowlist_core::model::AlertEvent::Planned => "Planned",
    };
    (label.to_string(), detail)
}

fn notification_menu_item(
    theme: super::theme::Theme,
    label: String,
    id: String,
    probe: impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .on_children_prepainted(probe)
        .id(super::sid(id))
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .text_color(theme.text_dim)
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover))
        .child(label)
        .on_click(on_click)
        .into_any_element()
}

fn notification_checkbox(
    theme: super::theme::Theme,
    checked: bool,
    id: String,
    probe: impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> AnyElement {
    div()
        .on_children_prepainted(probe)
        .id(super::sid(id))
        .size(px(15.))
        .flex_shrink_0()
        .rounded_sm()
        .border_1()
        .border_color(if checked {
            theme.accent
        } else {
            theme.border
        })
        .bg(if checked { theme.accent } else { theme.field })
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(checked, |this| {
            this.child(icons::icon(Icon::Check, 10.0, theme.bg))
        })
        .when(!checked, |this| this.child(div().size(px(1.))))
        .on_click(on_click)
        .into_any_element()
}

fn notification_row(
    theme: super::theme::Theme,
    check: Option<AnyElement>,
    label: &str,
    detail: Option<&str>,
    date: &str,
    remove: Option<AnyElement>,
    probe: impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static,
) -> AnyElement {
    let mut inner = div()
        .flex()
        .items_center()
        .gap_2()
        .flex_1()
        .min_w_0()
        .min_h(px(28.))
        .px_2()
        .py_1()
        .rounded_md()
        .bg(theme.field)
        .child(div().flex_shrink_0().text_xs().child(format!("{label}:")))
        .when_some(detail, |this, detail| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(detail.to_string()),
            )
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_xs()
                .text_color(theme.text_dim)
                .child(date.to_string()),
        );
    if let Some(remove) = remove {
        inner = inner.child(remove);
    }
    let mut row = div()
        .on_children_prepainted(probe)
        .flex()
        .items_center()
        .gap_2()
        .child(inner);
    if let Some(check) = check {
        row = row.child(check);
    }
    row.into_any_element()
}

fn batch_choice(
    theme: super::theme::Theme,
    label: impl Into<String>,
    active: bool,
    id: &str,
    boxes: std::rc::Rc<std::cell::RefCell<std::collections::BTreeMap<String, [f32; 4]>>>,
    cx: &mut Context<FlowlistApp>,
    on_click: impl Fn(&mut FlowlistApp, &mut Context<FlowlistApp>) + 'static,
) -> AnyElement {
    let key = id.to_string();
    div()
        .on_children_prepainted(move |bounds, _, _| {
            if let Some(bounds) = bounds.first() {
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
        })
        .id(super::sid(id.to_string()))
        .child(widgets::toggle_chip(theme, label, active))
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .into_any_element()
}

fn parse_batch_date(value: &str) -> Result<Option<Option<Stamp>>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if matches!(value.to_lowercase().as_str(), "none" | "clear" | "-") {
        return Ok(Some(None));
    }
    parse_stamp(value, now())
        .map(|stamp| Some(Some(stamp)))
        .map_err(|error| error.to_string())
}

fn parse_batch_estimate(value: &str) -> Result<Option<u32>, String> {
    let value = value.trim();
    if matches!(value.to_lowercase().as_str(), "none" | "clear" | "-") {
        return Ok(None);
    }
    parse_duration(value)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn parse_batch_tags(value: &str) -> (Vec<String>, Vec<String>) {
    let mut add = Vec::new();
    let mut remove = Vec::new();
    for token in value.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(tag) = token.strip_prefix('-') {
            let tag = tag.trim();
            if !tag.is_empty() {
                remove.push(tag.to_string());
            }
        } else {
            let tag = token.strip_prefix('+').unwrap_or(token).trim();
            if !tag.is_empty() {
                add.push(tag.to_string());
            }
        }
    }
    (add, remove)
}

/// Where an effective date comes from when the task itself has no value.
fn inherited_date_suffix(
    db: &flowlist_core::db::Db,
    task: &flowlist_core::model::Task,
    field: DateField,
) -> Option<&'static str> {
    let own = match field {
        DateField::Defer => task.defer,
        DateField::Planned => task.planned,
        DateField::Due => task.due,
        DateField::Reminder => return None,
    };
    if own.is_some() {
        return None;
    }
    let mut parent = task.parent.clone();
    let mut guard = 0;
    while let Some(id) = parent {
        guard += 1;
        if guard > 64 {
            break;
        }
        let Some(ancestor) = db.tasks.get(&id) else {
            break;
        };
        let value = match field {
            DateField::Defer => ancestor.defer,
            DateField::Planned => ancestor.planned,
            DateField::Due => ancestor.due,
            DateField::Reminder => None,
        };
        if value.is_some() {
            return Some("with parent");
        }
        parent = ancestor.parent.clone();
    }
    let project = db
        .effective_project(&task.id)
        .and_then(|id| db.projects.get(&id));
    let value = project.and_then(|project| match field {
        DateField::Defer => project.defer,
        DateField::Planned => project.planned,
        DateField::Due => project.due,
        DateField::Reminder => None,
    });
    value.is_some().then_some("with project")
}

impl FlowlistApp {
    fn inspector_section_open(&self, key: &str, default_open: bool) -> bool {
        self.inspector_sections
            .get(key)
            .map(|collapsed| !*collapsed)
            .unwrap_or(default_open)
    }

    fn toggle_inspector_section(
        &mut self,
        key: &'static str,
        default_open: bool,
        cx: &mut Context<Self>,
    ) {
        let now_open = self.inspector_section_open(key, default_open);
        self.inspector_sections
            .insert(key.to_string(), now_open);
        self.save_session();
        cx.notify();
    }

    /// One OmniFocus-style disclosure section. The header is the click target;
    /// its content is only built when open, so a collapsed inspector is short
    /// and cheap.
    fn inspector_section(
        &self,
        key: &'static str,
        title: &'static str,
        default_open: bool,
        content: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let open = self.inspector_section_open(key, default_open);
        let boxes = self.inspector_section_boxes.clone();
        let probe_key = key.to_string();
        let mut section = div()
            .flex()
            .flex_col()
            .gap_2()
            .border_b_1()
            .border_color(theme.border_soft)
            .pb_2()
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            boxes.borrow_mut().insert(
                                probe_key.clone(),
                                [
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ],
                            );
                        }
                    })
                    .id(super::sid(format!("inspector-section:{key}")))
                    .flex()
                    .items_center()
                    .gap_1()
                    .py_1()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(icons::icon(
                        if open {
                            Icon::ChevronDown
                        } else {
                            Icon::ChevronRight
                        },
                        10.0,
                        theme.text_faint,
                    ))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text_dim)
                            .child(title),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_inspector_section(key, default_open, cx)
                    })),
            );
        if open {
            section = section.child(content);
        }
        section.into_any_element()
    }

    pub(super) fn render_inspector(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        if self.batch_selection != self.selected {
            self.batch_selection = self.selected.clone();
            self.batch_draft = BatchDraft::default();
            self.batch_editing = None;
        }
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
                ),
        );

        if self.selected.len() == 1 && self.selected_one().is_some() {
            panel = panel.child(self.inspector_tab_bar(cx));
        }

        panel = panel.on_drag_move::<DragInspectorScroll>(cx.listener(
            |this, event: &gpui::DragMoveEvent<DragInspectorScroll>, _, cx| {
                let Some(drag_offset) = this.inspector_scroll_drag_grab.get() else {
                    return;
                };
                let bounds = this.inspector_scroll.bounds();
                let viewport = f32::from(bounds.size.height);
                let max_offset = f32::from(this.inspector_scroll.max_offset().height);
                if viewport <= 0.0 || max_offset <= 0.0 {
                    return;
                }
                let thumb_height =
                    (viewport * viewport / (viewport + max_offset)).clamp(24.0, viewport);
                let travel = (viewport - thumb_height).max(1.0);
                let pointer = f32::from(event.event.position.y - bounds.origin.y);
                let thumb_top = (pointer - drag_offset).clamp(0.0, travel);
                let scroll = (thumb_top / travel) * max_offset;
                this.inspector_scroll
                    .set_offset(gpui::point(px(0.), px(-scroll)));
                cx.notify();
            },
        ));

        let content = if self.selected.len() > 1 {
            self.render_batch_inspector(cx).into_any_element()
        } else if let Some(id) = self.selected_one() {
            match self.inspector_tab {
                InspectorTab::Properties => self
                    .render_task_properties(&id, window, cx)
                    .into_any_element(),
                InspectorTab::Subtasks => self.render_task_subtasks(&id, cx),
            }
        } else if let View::Project(id) = self.view.clone() {
            self.render_project_inspector(&id, window, cx)
                .into_any_element()
        } else {
            self.render_empty_inspector().into_any_element()
        };

        panel.child(
            div()
                .relative()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(
                    div()
                        .id("inspector-scroll")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.inspector_scroll)
                        .child(content),
                )
                .child(self.inspector_scrollbar(theme, cx)),
        )
    }

    fn inspector_scrollbar(
        &self,
        theme: super::theme::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        const THUMB_WIDTH: f32 = 6.0;
        const TRACK_WIDTH: f32 = 14.0;
        const PADDING: f32 = 4.0;
        const MIN_THUMB: f32 = 24.0;

        let handle = &self.inspector_scroll;
        let bounds = handle.bounds();
        let viewport = f32::from(bounds.size.height);
        let max_offset = f32::from(handle.max_offset().height);
        if viewport <= 0.0 || max_offset <= 1.0 {
            return div().into_any_element();
        }
        let scrolled = (-f32::from(handle.offset().y)).clamp(0.0, max_offset);
        let thumb_height =
            (viewport * viewport / (viewport + max_offset)).clamp(MIN_THUMB, viewport);
        let travel = (viewport - thumb_height).max(1.0);
        let thumb_top = (scrolled / max_offset) * travel;
        let thumb_top_window = f32::from(bounds.origin.y) + thumb_top;
        let grab = self.inspector_scroll_drag_grab.clone();
        let thumb = div()
            .id("inspector-scroll-thumb")
            .absolute()
            .top(px(thumb_top))
            .right(px(PADDING))
            .w(px(THUMB_WIDTH))
            .h(px(thumb_height))
            .rounded_full()
            .bg(theme.border)
            .hover(|style| style.bg(theme.text_faint))
            .on_drag(DragInspectorScroll, move |payload, position, window, cx| {
                let pointer = f32::from(position.y);
                let drag_offset = (pointer - thumb_top_window).clamp(0.0, thumb_height);
                grab.set(Some(drag_offset));
                inspector_scroll_ghost(payload, position, window, cx)
            });
        div()
            .on_children_prepainted(self.theme_probe("inspector-scrollbar".to_string()))
            .id("inspector-scroll-track")
            .absolute()
            .top_0()
            .right_0()
            .h_full()
            .w(px(TRACK_WIDTH))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    let pointer = f32::from(event.position.y);
                    if pointer >= thumb_top_window && pointer <= thumb_top_window + thumb_height {
                        return;
                    }
                    let bounds = this.inspector_scroll.bounds();
                    let viewport = f32::from(bounds.size.height);
                    let max_offset = f32::from(this.inspector_scroll.max_offset().height);
                    if viewport <= 0.0 || max_offset <= 0.0 {
                        return;
                    }
                    let thumb_height =
                        (viewport * viewport / (viewport + max_offset)).clamp(MIN_THUMB, viewport);
                    let travel = (viewport - thumb_height).max(1.0);
                    let pointer = f32::from(event.position.y - bounds.origin.y);
                    let thumb_top = (pointer - thumb_height / 2.0).clamp(0.0, travel);
                    let scroll = (thumb_top / travel) * max_offset;
                    this.inspector_scroll
                        .set_offset(gpui::point(px(0.), px(-scroll)));
                    cx.notify();
                }),
            )
            .child(thumb)
            .into_any_element()
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

    fn render_batch_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let count = self.selected.len();
        let mut body =
            div()
                .flex()
                .flex_col()
                .gap_3()
                .p_3()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("{count} tasks selected")),
                )
                .child(div().text_xs().text_color(theme.text_faint).child(
                    "Set a field to change every selected task. Empty fields stay unchanged.",
                ))
                .child(self.batch_text_field(BatchField::Project, &self.batch_draft.project, cx))
                .child(self.batch_text_field(BatchField::Tags, &self.batch_draft.tags, cx))
                .child(self.batch_priority(cx))
                .child(self.batch_text_field(BatchField::Defer, &self.batch_draft.defer, cx))
                .child(self.batch_text_field(BatchField::Planned, &self.batch_draft.planned, cx))
                .child(self.batch_text_field(BatchField::Due, &self.batch_draft.due, cx))
                .child(self.batch_flag(cx))
                .child(self.batch_text_field(BatchField::Estimate, &self.batch_draft.estimate, cx));
        if let Some(error) = &self.batch_draft.error {
            body = body.child(
                div()
                    .text_xs()
                    .text_color(theme.warning)
                    .whitespace_normal()
                    .child(error.clone()),
            );
        }
        body.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .id("batch-clear")
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_xs()
                        .text_color(theme.text_dim)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.hover))
                        .child("Clear")
                        .on_click(cx.listener(|this, _, _, cx| this.clear_batch(cx))),
                )
                .child({
                    let boxes = self.batch_boxes.clone();
                    div()
                        .on_children_prepainted(move |bounds, _, _| {
                            if let Some(bounds) = bounds.first() {
                                boxes.borrow_mut().insert(
                                    "batch-apply".to_string(),
                                    [
                                        f32::from(bounds.origin.x),
                                        f32::from(bounds.origin.y),
                                        f32::from(bounds.size.width),
                                        f32::from(bounds.size.height),
                                    ],
                                );
                            }
                        })
                        .id("batch-apply")
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.accent)
                        .text_xs()
                        .text_color(theme.accent)
                        .cursor_pointer()
                        .hover(|style| style.bg(theme.selection))
                        .child(format!("Apply to {count} tasks"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_batch(cx)))
                }),
        )
    }

    fn batch_text_field(
        &self,
        field: BatchField,
        value: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing = self.batch_editing == Some(field) && self.focus == super::FocusArea::Field;
        let icon = match field {
            BatchField::Project => Icon::Project,
            BatchField::Tags => Icon::Tag,
            BatchField::Defer => Icon::DeferDate,
            BatchField::Planned => Icon::PlannedDate,
            BatchField::Due => Icon::DueDate,
            BatchField::Estimate => Icon::Clock,
        };
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(icons::icon(icon, 11.0, theme.text_faint))
                    .child(field.label()),
            )
            .child(if editing {
                div()
                    .id("batch-field-editor")
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
                let boxes = self.batch_boxes.clone();
                let key = format!("batch-field:{}", field.slug());
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
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
                    })
                    .id(super::sid(format!("batch-field:{}", field.slug())))
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.field)
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
                    .on_click(cx.listener(move |this, _, _, cx| this.begin_batch_edit(field, cx)))
                    .into_any_element()
            });
        if editing {
            row = row.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("⏎ next · ⇥ next · esc cancel"),
            );
        }
        row
    }

    fn batch_priority(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let current = self.batch_draft.priority;
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(icons::icon(Icon::Flagged, 11.0, theme.text_faint))
                    .child("Priority"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(batch_choice(
                        theme,
                        "Unchanged",
                        current.is_none(),
                        "batch-priority:unchanged",
                        self.batch_boxes.clone(),
                        cx,
                        |this, cx| {
                            this.batch_draft.priority = None;
                            this.batch_draft.error = None;
                            cx.notify();
                        },
                    ))
                    .child(batch_choice(
                        theme,
                        "None",
                        current == Some(None),
                        "batch-priority:none",
                        self.batch_boxes.clone(),
                        cx,
                        |this, cx| {
                            this.batch_draft.priority = Some(None);
                            this.batch_draft.error = None;
                            cx.notify();
                        },
                    )),
            );
        let mut options = div().flex().items_center().gap_1();
        for priority in Priority::ALL {
            let selected = current == Some(Some(priority));
            options = options.child(batch_choice(
                theme,
                priority.label(),
                selected,
                &format!("batch-priority:{}", priority.slug()),
                self.batch_boxes.clone(),
                cx,
                move |this, cx| {
                    this.batch_draft.priority = Some(Some(priority));
                    this.batch_draft.error = None;
                    cx.notify();
                },
            ));
        }
        row.child(options)
    }

    fn batch_flag(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let current = self.batch_draft.flagged;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(icons::icon(Icon::Flagged, 11.0, theme.text_faint))
                    .child("Flag"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(batch_choice(
                        theme,
                        "Unchanged",
                        current.is_none(),
                        "batch-flag:unchanged",
                        self.batch_boxes.clone(),
                        cx,
                        |this, cx| {
                            this.batch_draft.flagged = None;
                            this.batch_draft.error = None;
                            cx.notify();
                        },
                    ))
                    .child(batch_choice(
                        theme,
                        "Flag all",
                        current == Some(true),
                        "batch-flag:true",
                        self.batch_boxes.clone(),
                        cx,
                        |this, cx| {
                            this.batch_draft.flagged = Some(true);
                            this.batch_draft.error = None;
                            cx.notify();
                        },
                    ))
                    .child(batch_choice(
                        theme,
                        "Unflag all",
                        current == Some(false),
                        "batch-flag:false",
                        self.batch_boxes.clone(),
                        cx,
                        |this, cx| {
                            this.batch_draft.flagged = Some(false);
                            this.batch_draft.error = None;
                            cx.notify();
                        },
                    )),
            )
    }

    pub(super) fn begin_batch_edit(&mut self, field: BatchField, cx: &mut Context<Self>) {
        if self.batch_editing.is_some() {
            let _ = self.commit_batch_field(cx);
        }
        let value = match field {
            BatchField::Project => self.batch_draft.project.clone(),
            BatchField::Tags => self.batch_draft.tags.clone(),
            BatchField::Defer => self.batch_draft.defer.clone(),
            BatchField::Planned => self.batch_draft.planned.clone(),
            BatchField::Due => self.batch_draft.due.clone(),
            BatchField::Estimate => self.batch_draft.estimate.clone(),
        };
        self.field.set(value);
        self.field.select_all();
        self.batch_editing = Some(field);
        self.batch_draft.error = None;
        self.focus = super::FocusArea::Field;
        cx.notify();
    }

    pub(super) fn commit_batch_field(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(field) = self.batch_editing else {
            return true;
        };
        let value = self.field.text().trim().to_string();
        match field {
            BatchField::Project => self.batch_draft.project = value,
            BatchField::Tags => self.batch_draft.tags = value,
            BatchField::Defer => self.batch_draft.defer = value,
            BatchField::Planned => self.batch_draft.planned = value,
            BatchField::Due => self.batch_draft.due = value,
            BatchField::Estimate => self.batch_draft.estimate = value,
        }
        self.batch_editing = None;
        self.batch_draft.error = None;
        cx.notify();
        true
    }

    fn clear_batch(&mut self, cx: &mut Context<Self>) {
        self.batch_draft = BatchDraft::default();
        self.batch_editing = None;
        self.field_error = None;
        cx.notify();
    }

    fn apply_batch(&mut self, cx: &mut Context<Self>) {
        if self.batch_editing.is_some() {
            let _ = self.commit_batch_field(cx);
        }
        let ids = self.selected.clone();
        if ids.len() < 2 {
            return;
        }
        let mut patch = TaskPatch::default();
        let mut changed = false;
        let project_text = self.batch_draft.project.trim().to_string();
        if !project_text.is_empty() {
            let clear = matches!(
                project_text.to_lowercase().as_str(),
                "none" | "clear" | "inbox" | "-"
            );
            if clear {
                patch.project = Some(None);
            } else {
                let db = self.store.db();
                let found = db
                    .find_project_by_path(&project_text)
                    .or_else(|| db.find_project_by_name(&project_text));
                match found {
                    Some(project) => patch.project = Some(Some(project)),
                    None => {
                        self.batch_draft.error = Some(format!("Project not found: {project_text}"));
                        cx.notify();
                        return;
                    }
                }
            }
            changed = true;
        }
        if let Some(priority) = self.batch_draft.priority {
            patch.priority = Some(priority);
            changed = true;
        }
        if let Some(flagged) = self.batch_draft.flagged {
            patch.flagged = Some(flagged);
            changed = true;
        }
        for (field, value) in [
            (BatchField::Defer, &self.batch_draft.defer),
            (BatchField::Planned, &self.batch_draft.planned),
            (BatchField::Due, &self.batch_draft.due),
        ] {
            match parse_batch_date(value) {
                Ok(Some(change)) => {
                    match field {
                        BatchField::Defer => patch.defer = Some(change),
                        BatchField::Planned => patch.planned = Some(change),
                        BatchField::Due => patch.due = Some(change),
                        _ => {}
                    }
                    changed = true;
                }
                Ok(None) => {}
                Err(error) => {
                    self.batch_draft.error = Some(format!("{}: {error}", field.label()));
                    cx.notify();
                    return;
                }
            }
        }
        if !self.batch_draft.estimate.trim().is_empty() {
            match parse_batch_estimate(&self.batch_draft.estimate) {
                Ok(change) => {
                    patch.estimate_min = Some(change);
                    changed = true;
                }
                Err(error) => {
                    self.batch_draft.error = Some(format!("Estimate: {error}"));
                    cx.notify();
                    return;
                }
            }
        }
        let (add_tags, remove_tags) = parse_batch_tags(&self.batch_draft.tags);
        if !add_tags.is_empty() || !remove_tags.is_empty() {
            changed = true;
        }
        if !changed {
            self.batch_draft.error = Some("Nothing to apply".to_string());
            cx.notify();
            return;
        }
        let _ = self.execute(Command::BatchUpdateTasks {
            ids,
            patch,
            add_tags,
            remove_tags,
        });
        if let Some(error) = self.command_error.take() {
            self.toast = None;
            self.batch_draft.error = Some(error);
            cx.notify();
            return;
        }
        self.batch_draft = BatchDraft::default();
        self.batch_editing = None;
        self.focus = super::FocusArea::Content;
        cx.notify();
    }

    fn set_fullscreen_notification(
        &mut self,
        target: &NotificationTarget,
        checked: bool,
        at: chrono::DateTime<chrono::Utc>,
        match_due: bool,
        cx: &mut Context<Self>,
    ) {
        let NotificationTarget::Task(id) = target else {
            return;
        };
        let mut patch = TaskPatch {
            fullscreen_reminder: Some(checked),
            ..Default::default()
        };
        if checked {
            patch.reminder_match_due = Some(match_due);
            patch.reminder_at = if match_due {
                Some(None)
            } else {
                Some(Some(Stamp {
                    at,
                    has_time: true,
                }))
            };
        }
        self.execute(Command::UpdateTask {
            id: id.clone(),
            patch,
        });
        cx.notify();
    }

    fn notifications_have_data(&self, target: &NotificationTarget) -> bool {
        let db = self.store.db();
        let settings = &db.settings;
        let (due, defer, planned, estimate, removed, custom, alerts) = match target {
            NotificationTarget::Task(id) => {
                let task = db.tasks.get(id);
                (
                    db.effective_due(id),
                    db.effective_defer(id),
                    db.effective_planned(id),
                    task.and_then(|task| task.estimate_min),
                    task.map(|task| task.removed_notifications.clone())
                        .unwrap_or_default(),
                    task.map(|task| task.custom_notifications.clone())
                        .unwrap_or_default(),
                    task.map(|task| task.alerts.clone()).unwrap_or_default(),
                )
            }
            NotificationTarget::Project(id) => {
                let project = db.projects.get(id);
                (
                    project.and_then(|project| project.due),
                    project.and_then(|project| project.defer),
                    project.and_then(|project| project.planned),
                    None,
                    project
                        .map(|project| project.removed_notifications.clone())
                        .unwrap_or_default(),
                    project
                        .map(|project| project.custom_notifications.clone())
                        .unwrap_or_default(),
                    project.map(|project| project.alerts.clone()).unwrap_or_default(),
                )
            }
        };
        if !custom.is_empty() {
            return true;
        }
        for alert in &alerts {
            let base = match alert.event {
                AlertEvent::Due => due,
                AlertEvent::Defer => defer,
                AlertEvent::Planned => planned,
            };
            if base.is_some() {
                return true;
            }
        }
        for kind in DefaultNotification::ALL {
            let global = match kind {
                DefaultNotification::Defer => settings.default_defer_notification,
                DefaultNotification::Planned => settings.default_planned_notification,
                DefaultNotification::LatestStart => settings.default_latest_start_notification,
                DefaultNotification::Due => settings.default_due_notification,
            };
            if !global || removed.contains(&kind) {
                continue;
            }
            let exists = match kind {
                DefaultNotification::Defer => defer.is_some(),
                DefaultNotification::Planned => planned.is_some(),
                DefaultNotification::LatestStart => due.is_some() && estimate.is_some(),
                DefaultNotification::Due => due.is_some(),
            };
            if exists {
                return true;
            }
        }
        false
    }

    fn inspector_notifications(
        &self,
        target: NotificationTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let settings = &db.settings;
        let (due, defer, planned, estimate, removed, custom, alerts) = match &target {
            NotificationTarget::Task(id) => {
                let task = db.tasks.get(id);
                (
                    db.effective_due(id),
                    db.effective_defer(id),
                    db.effective_planned(id),
                    task.and_then(|task| task.estimate_min),
                    task.map(|task| task.removed_notifications.clone())
                        .unwrap_or_default(),
                    task.map(|task| task.custom_notifications.clone())
                        .unwrap_or_default(),
                    task.map(|task| task.alerts.clone()).unwrap_or_default(),
                )
            }
            NotificationTarget::Project(id) => {
                let project = db.projects.get(id);
                (
                    project.and_then(|project| project.due),
                    project.and_then(|project| project.defer),
                    project.and_then(|project| project.planned),
                    None,
                    project
                        .map(|project| project.removed_notifications.clone())
                        .unwrap_or_default(),
                    project
                        .map(|project| project.custom_notifications.clone())
                        .unwrap_or_default(),
                    project.map(|project| project.alerts.clone()).unwrap_or_default(),
                )
            }
        };
        let is_task = matches!(target, NotificationTarget::Task(_));
        let fullscreen = match &target {
            NotificationTarget::Task(id) => db
                .tasks
                .get(id)
                .map(|task| task.fullscreen_reminder)
                .unwrap_or(false),
            NotificationTarget::Project(_) => false,
        };
        let fsr_at = match &target {
            NotificationTarget::Task(id) if fullscreen => {
                db.reminder_stamp(id).map(super::notifications::reminder_at)
            }
            _ => None,
        };

        let mut rows: Vec<(chrono::DateTime<chrono::Utc>, AnyElement)> = Vec::new();
        for kind in DefaultNotification::ALL {
            let global = match kind {
                DefaultNotification::Defer => settings.default_defer_notification,
                DefaultNotification::Planned => settings.default_planned_notification,
                DefaultNotification::LatestStart => settings.default_latest_start_notification,
                DefaultNotification::Due => settings.default_due_notification,
            };
            if !global || removed.contains(&kind) {
                continue;
            }
            let at = match kind {
                DefaultNotification::Defer => defer.map(super::notifications::reminder_at),
                DefaultNotification::Planned => planned.map(super::notifications::reminder_at),
                DefaultNotification::LatestStart => due.zip(estimate).map(|(due, estimate)| {
                    super::notifications::reminder_at(due)
                        - chrono::Duration::minutes(estimate as i64)
                }),
                DefaultNotification::Due => due.map(super::notifications::reminder_at),
            };
            let Some(at) = at else {
                continue;
            };
            let label = match kind {
                DefaultNotification::Defer => "Deferred",
                DefaultNotification::Planned => "Planned",
                DefaultNotification::LatestStart => "Latest Start",
                DefaultNotification::Due => "Due",
            };
            let mut next = removed.clone();
            next.insert(kind);
            let next_target = target.clone();
            let probe = format!("notification-default:{}", kind.slug());
            let remove = div()
                .on_children_prepainted(self.notification_probe(format!("{probe}:remove")))
                .id(super::sid(format!("notification-remove:{}", kind.slug())))
                .cursor_pointer()
                .text_color(theme.text_faint)
                .hover(|style| style.text_color(theme.text))
                .child(icons::icon(Icon::Close, 11.0, theme.text_faint))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_notifications(
                        next_target.clone(),
                        Some(next.clone()),
                        None,
                        None,
                        cx,
                    );
                }))
                .into_any_element();
            let check = is_task.then(|| {
                let checked = fsr_at == Some(at);
                let fsr_target = target.clone();
                notification_checkbox(
                    theme,
                    checked,
                    format!("notification-fsr:default.{}", kind.slug()),
                    self.notification_probe(format!("notification-fsr:default.{}", kind.slug())),
                    cx.listener(move |this, _, _, cx| {
                        this.set_fullscreen_notification(
                            &fsr_target,
                            !checked,
                            at,
                            kind == DefaultNotification::Due,
                            cx,
                        );
                    }),
                )
            });
            let row = notification_row(
                theme,
                check,
                label,
                None,
                &format_notification_time(at),
                Some(remove),
                self.notification_probe(probe),
            );
            rows.push((at, row));
        }
        for (index, alert) in alerts.iter().enumerate() {
            let base = match alert.event {
                flowlist_core::model::AlertEvent::Due => due,
                flowlist_core::model::AlertEvent::Defer => defer,
                flowlist_core::model::AlertEvent::Planned => planned,
            };
            let Some(base) = base else {
                continue;
            };
            let at = super::notifications::reminder_at(base)
                + chrono::Duration::minutes(alert.offset_minutes as i64);
            let (label, detail) = relative_notification_label(alert);
            let mut next = alerts.clone();
            next.remove(index);
            let next_target = target.clone();
            let probe = format!("notification-alert:{index}");
            let remove = div()
                .on_children_prepainted(
                    self.notification_probe(format!("notification-alert-remove:{index}")),
                )
                .id(super::sid(format!("notification-alert-remove:{index}")))
                .cursor_pointer()
                .text_color(theme.text_faint)
                .hover(|style| style.text_color(theme.text))
                .child(icons::icon(Icon::Close, 11.0, theme.text_faint))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_notifications(
                        next_target.clone(),
                        None,
                        None,
                        Some(next.clone()),
                        cx,
                    );
                }))
                .into_any_element();
            let check = is_task.then(|| {
                let checked = fsr_at == Some(at);
                let fsr_target = target.clone();
                let key = format!("notification-fsr:alert.{index}");
                notification_checkbox(
                    theme,
                    checked,
                    key.clone(),
                    self.notification_probe(key),
                    cx.listener(move |this, _, _, cx| {
                        this.set_fullscreen_notification(
                            &fsr_target,
                            !checked,
                            at,
                            false,
                            cx,
                        );
                    }),
                )
            });
            let row = notification_row(
                theme,
                check,
                &label,
                detail.as_deref(),
                &format_notification_time(at),
                Some(remove),
                self.notification_probe(probe),
            );
            rows.push((at, row));
        }
        for (index, stamp) in custom.iter().enumerate() {
            let at = super::notifications::custom_reminder_at(*stamp, settings);
            let mut next = custom.clone();
            next.remove(index);
            let next_target = target.clone();
            let probe = format!("notification-custom:{index}");
            let remove = div()
                .on_children_prepainted(
                    self.notification_probe(format!("notification-custom-remove:{index}")),
                )
                .id(super::sid(format!("notification-custom-remove:{index}")))
                .cursor_pointer()
                .text_color(theme.text_faint)
                .hover(|style| style.text_color(theme.text))
                .child(icons::icon(Icon::Close, 11.0, theme.text_faint))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_notifications(
                        next_target.clone(),
                        None,
                        Some(next.clone()),
                        None,
                        cx,
                    );
                }))
                .into_any_element();
            let check = is_task.then(|| {
                let checked = fsr_at == Some(at);
                let fsr_target = target.clone();
                let key = format!("notification-fsr:custom.{index}");
                notification_checkbox(
                    theme,
                    checked,
                    key.clone(),
                    self.notification_probe(key),
                    cx.listener(move |this, _, _, cx| {
                        this.set_fullscreen_notification(
                            &fsr_target,
                            !checked,
                            at,
                            false,
                            cx,
                        );
                    }),
                )
            });
            let row = notification_row(
                theme,
                check,
                "Custom",
                None,
                &format_notification_time(at),
                Some(remove),
                self.notification_probe(probe),
            );
            rows.push((at, row));
        }
        rows.sort_by_key(|(at, _)| *at);
        let mut list = div().flex().flex_col().gap_1();
        if rows.is_empty() {
            list = list.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No notifications"),
            );
        } else {
            for (_, row) in rows {
                list = list.child(row);
            }
        }

        let mut body = div().flex().flex_col().gap_2().child(list);
        if is_task {
            body = body.child(
                div()
                    .text_xs()
                    .whitespace_normal()
                    .text_color(theme.text_faint)
                    .child(
                        "Check a notification to show it full screen instead of as a banner",
                    ),
            );
        }
        let add_target = target.clone();
        let boxes = self.notification_boxes.clone();
        let anchor = self.notification_anchor.clone();
        body = body.child(
            div()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        anchor.set(Some(*bounds));
                        boxes.borrow_mut().insert(
                            "notification-add".to_string(),
                            [
                                f32::from(bounds.origin.x),
                                f32::from(bounds.origin.y),
                                f32::from(bounds.size.width),
                                f32::from(bounds.size.height),
                            ],
                        );
                    }
                })
                .id("notification-add")
                .flex()
                .items_center()
                .justify_between()
                .h(px(28.))
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.field)
                .text_xs()
                .text_color(theme.text_dim)
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child("Add Notification")
                .child(icons::icon(Icon::ChevronDown, 10.0, theme.text_faint))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.notification_menu = Some(add_target.clone());
                    cx.notify();
                })),
        );
        body.into_any_element()
    }

    pub(super) fn render_notification_menu(
        &self,
        target: &NotificationTarget,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = self.theme;
        let db = self.store.db();
        let settings = &db.settings;
        let (due, defer, planned, estimate, removed) = match target {
            NotificationTarget::Task(id) => {
                let task = db.tasks.get(id);
                (
                    db.effective_due(id),
                    db.effective_defer(id),
                    db.effective_planned(id),
                    task.and_then(|task| task.estimate_min),
                    task.map(|task| task.removed_notifications.clone())
                        .unwrap_or_default(),
                )
            }
            NotificationTarget::Project(id) => {
                let project = db.projects.get(id);
                (
                    project.and_then(|project| project.due),
                    project.and_then(|project| project.defer),
                    project.and_then(|project| project.planned),
                    None,
                    project
                        .map(|project| project.removed_notifications.clone())
                        .unwrap_or_default(),
                )
            }
        };
        let mut menu = div()
            .min_w(px(200.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg();
        let mut defaults = 0;
        for kind in DefaultNotification::ALL {
            let global = match kind {
                DefaultNotification::Defer => settings.default_defer_notification,
                DefaultNotification::Planned => settings.default_planned_notification,
                DefaultNotification::LatestStart => settings.default_latest_start_notification,
                DefaultNotification::Due => settings.default_due_notification,
            };
            let applicable = match kind {
                DefaultNotification::Defer => defer.is_some(),
                DefaultNotification::Planned => planned.is_some(),
                DefaultNotification::LatestStart => due.is_some() && estimate.is_some(),
                DefaultNotification::Due => due.is_some(),
            };
            if !global || !applicable || !removed.contains(&kind) {
                continue;
            }
            defaults += 1;
            let mut next = removed.clone();
            next.remove(&kind);
            let next_target = target.clone();
            menu = menu.child(notification_menu_item(
                theme,
                format!("{} Notification", kind.label()),
                format!("notification-add:{}", kind.slug()),
                self.notification_probe(format!("notification-add:{}", kind.slug())),
                cx.listener(move |this, _, _, cx| {
                    this.update_notifications(
                        next_target.clone(),
                        Some(next.clone()),
                        None,
                        None,
                        cx,
                    );
                    this.notification_menu = None;
                    cx.notify();
                }),
            ));
        }
        if defaults > 0 {
            menu = menu.child(widgets::divider(theme));
        }
        if due.is_some() || defer.is_some() || planned.is_some() {
            let event = if due.is_some() {
                AlertEvent::Due
            } else if defer.is_some() {
                AlertEvent::Defer
            } else {
                AlertEvent::Planned
            };
            let label = if due.is_some() {
                "Before Due…"
            } else {
                "Relative Alert…"
            };
            let next_target = target.clone();
            menu = menu.child(notification_menu_item(
                theme,
                label.to_string(),
                "notification-add:before-due".to_string(),
                self.notification_probe("notification-add:before-due".to_string()),
                cx.listener(move |this, _, _, cx| {
                    this.notification_menu = None;
                    this.open_modal(
                        super::modals::Modal::relative_notification(next_target.clone(), event),
                        cx,
                    );
                }),
            ));
        }
        let next_target = target.clone();
        menu = menu.child(notification_menu_item(
            theme,
            "Custom Notification…".to_string(),
            "notification-add:custom".to_string(),
            self.notification_probe("notification-add:custom".to_string()),
            cx.listener(move |this, _, _, cx| {
                this.notification_menu = None;
                this.open_modal(
                    super::modals::Modal::custom_notification(next_target.clone()),
                    cx,
                );
            }),
        ));
        menu
    }

    fn update_notifications(
        &mut self,
        target: NotificationTarget,
        removed: Option<BTreeSet<DefaultNotification>>,
        custom: Option<Vec<Stamp>>,
        alerts: Option<Vec<flowlist_core::model::TaskAlert>>,
        cx: &mut Context<Self>,
    ) {
        let section = match target {
            NotificationTarget::Task(_) => "task:notifications",
            NotificationTarget::Project(_) => "project:notifications",
        };
        match target {
            NotificationTarget::Task(id) => {
                let patch = TaskPatch {
                    removed_notifications: removed,
                    custom_notifications: custom,
                    alerts,
                    ..Default::default()
                };
                self.execute(Command::UpdateTask { id, patch });
            }
            NotificationTarget::Project(id) => {
                let patch = flowlist_core::command::ProjectPatch {
                    removed_notifications: removed,
                    custom_notifications: custom,
                    alerts,
                    ..Default::default()
                };
                self.execute(Command::UpdateProject { id, patch });
            }
        }
        // Removing the last row must not fold the section and hide the Add
        // Notification control.
        self.inspector_sections.insert(section.to_string(), false);
        self.save_session();
        cx.notify();
    }

    fn inspector_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let active = self.inspector_tab;
        let mut row = div()
            .flex()
            .items_center()
            .gap_1()
            .px_3()
            .py_1()
            .border_b_1()
            .border_color(theme.border);
        for (tab, label) in [
            (InspectorTab::Properties, "Properties"),
            (InspectorTab::Subtasks, "Subtasks"),
        ] {
            row = row.child(
                div()
                    .on_children_prepainted(
                        self.theme_probe(format!("inspector-tab:{}", tab.slug())),
                    )
                    .id(super::sid(format!("inspector-tab:{}", tab.slug())))
                    .flex_1()
                    .cursor_pointer()
                    .child(widgets::toggle_chip(theme, label, active == tab))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.inspector_tab = tab;
                        this.inspector_scroll
                            .set_offset(gpui::point(px(0.), px(0.)));
                        cx.notify();
                    })),
            );
        }
        row.into_any_element()
    }

    fn render_task_subtasks(&self, id: &TaskId, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let db = self.store.db();
        let Some(task) = db.tasks.get(id) else {
            return self.render_empty_inspector().into_any_element();
        };
        let title = task.title.clone();
        let sequential = task.sequential;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .whitespace_normal()
                    .child(title),
            )
            .child(self.inspector_task_type(id, sequential, cx))
            .into_any_element()
    }

    fn render_task_properties(
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
        let defer = db
            .effective_defer(&id)
            .map(|stamp| human_date(&stamp, now()));
        let planned = db
            .effective_planned(&id)
            .map(|stamp| human_date(&stamp, now()));
        let due = db.effective_due(&id).map(|stamp| human_date(&stamp, now()));
        let defer_suffix = inherited_date_suffix(db, task, DateField::Defer);
        let planned_suffix = inherited_date_suffix(db, task, DateField::Planned);
        let due_suffix = inherited_date_suffix(db, task, DateField::Due);
        let estimate = task.estimate_min.map(format_duration);
        let repeat_rule = task.repeat.clone();
        let attachments = task.attachments.clone();
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
        let has_action = priority.is_some() || project_id.is_some();
        let has_dates = defer.is_some() || planned.is_some() || due.is_some() || estimate.is_some();
        let has_notes = !notes.is_empty() || !attachments.is_empty();
        let notifications_target = NotificationTarget::Task(id.clone());
        let has_notifications = self.notifications_have_data(&notifications_target);

        let title_content = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.inspector_text_field("Title", EditField::Title, &title, false, cx))
            .into_any_element();
        let action_content = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.inspector_priority(&id, priority, cx))
            .child(self.inspector_project_row(project, project_id.clone(), cx))
            .into_any_element();
        let tags_content = self.inspector_tag_section(&tags, cx).into_any_element();
        let dates_content = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.inspector_date_field(
                DateField::Defer,
                &defer.unwrap_or_default(),
                defer_suffix,
                cx,
            ))
            .child(self.inspector_date_field(
                DateField::Planned,
                &planned.unwrap_or_default(),
                planned_suffix,
                cx,
            ))
            .child(self.inspector_date_field(
                DateField::Due,
                &due.unwrap_or_default(),
                due_suffix,
                cx,
            ))
            .child(self.inspector_text_field(
                "Estimate",
                EditField::Estimate,
                &estimate.unwrap_or_default(),
                true,
                cx,
            ))
            .into_any_element();
        let repeat_content = self
            .inspector_repeat_task(&id, repeat_rule.clone(), cx)
            .into_any_element();
        let notifications_content = self.inspector_notifications(notifications_target, cx);
        let notes_content = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.inspector_notes(&notes, cx))
            .child(self.inspector_attachments(
                &attachments,
                super::modals::AttachmentTarget::Task(id.clone()),
                window,
                cx,
            ))
            .into_any_element();
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
            .child(self.inspector_section("task:title", "Title", true, title_content, cx))
            .child(self.inspector_section(
                "task:action",
                "Action",
                has_action,
                action_content,
                cx,
            ))
            .child(self.inspector_section(
                "task:notes",
                "Notes & Attachments",
                has_notes,
                notes_content,
                cx,
            ))
            .child(self.inspector_section(
                "task:dates",
                "Dates",
                has_dates,
                dates_content,
                cx,
            ))
            .child(self.inspector_section(
                "task:repeat",
                "Repeat",
                repeat_rule.is_some(),
                repeat_content,
                cx,
            ))
            .child(self.inspector_section(
                "task:tags",
                "Tags",
                !tags.is_empty(),
                tags_content,
                cx,
            ))
            .child(self.inspector_section(
                "task:notifications",
                "Notifications",
                has_notifications,
                notifications_content,
                cx,
            ))
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

    fn set_task_repeat(
        &mut self,
        id: &TaskId,
        rule: Option<RepeatRule>,
        cx: &mut Context<Self>,
    ) {
        self.execute(Command::UpdateTask {
            id: id.clone(),
            patch: TaskPatch {
                repeat: Some(rule),
                ..Default::default()
            },
        });
        cx.notify();
    }

    fn update_task_repeat(
        &mut self,
        id: &TaskId,
        current: &RepeatRule,
        change: impl FnOnce(&mut RepeatRule),
        cx: &mut Context<Self>,
    ) {
        let mut rule = current.clone();
        change(&mut rule);
        self.set_task_repeat(id, Some(rule), cx);
    }

    /// OmniFocus-style repeat controls. The common fields are chips and a
    /// stepper; the compact text field at the bottom keeps advanced endings
    /// and rules available without turning the whole section into a form.
    fn inspector_repeat_task(
        &self,
        id: &TaskId,
        rule: Option<RepeatRule>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let id = id.clone();
        let enabled = rule.is_some();
        let every = rule.as_ref().map(|rule| rule.every.max(1)).unwrap_or(1);
        let unit = rule
            .as_ref()
            .map(|rule| rule.unit)
            .unwrap_or(IntervalUnit::Weeks);
        let mode = rule.as_ref().map(|rule| rule.mode).unwrap_or_default();
        let schedule = rule
            .as_ref()
            .map(|rule| rule.schedule)
            .unwrap_or_default();
        let weekdays_selected = rule
            .as_ref()
            .is_some_and(|rule| !rule.selected_weekdays().is_empty());
        let toggle_id = id.clone();
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("repeat-toggle".to_string()),
                            )
                            .id("inspector-repeat-toggle")
                            .size(px(15.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if enabled {
                                theme.accent
                            } else {
                                theme.border
                            })
                            .bg(if enabled {
                                theme.accent
                            } else {
                                theme.field
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(enabled, |this| {
                                this.child(icons::icon(Icon::Check, 10.0, theme.bg))
                            })
                            .when(!enabled, |this| {
                                // Keep one child so the probe bounds still paint
                                // for an unchecked repeat box.
                                this.child(div().size(px(1.)))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let next = if enabled {
                                    None
                                } else {
                                    Some(RepeatRule::new(
                                        RepeatMode::DueAgain,
                                        1,
                                        IntervalUnit::Weeks,
                                    ))
                                };
                                this.set_task_repeat(&toggle_id, next, cx);
                            })),
                    )
                    .child(div().text_sm().child("Repeat Every"))
                    .when(enabled, |this| {
                        this.child(
                            div().flex_1().child(self.inspector_repeat_interval(
                                &id,
                                every,
                                unit,
                                weekdays_selected,
                                cx,
                            )),
                        )
                    }),
            );
        if enabled {
            if unit == IntervalUnit::Days {
                body = body.child(self.inspector_week_strip(&id, &rule, cx));
            }
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("Schedule the next"),
                    )
                    .child(segmented_row(
                        theme,
                        "repeat-mode",
                        &[("due", "Due Date"), ("defer", "Defer Until Date")],
                        if mode == RepeatMode::DueAgain { 0 } else { 1 },
                        self.theme_boxes.clone(),
                        {
                            let current_rule = rule.clone();
                            let mode_id = id.clone();
                            move |this: &mut FlowlistApp, index, cx| {
                                if let Some(current) = &current_rule {
                                    this.update_task_repeat(
                                        &mode_id,
                                        current,
                                        |rule| {
                                            rule.mode = if index == 0 {
                                                RepeatMode::DueAgain
                                            } else {
                                                RepeatMode::DeferAnother
                                            }
                                        },
                                        cx,
                                    );
                                }
                            }
                        },
                        cx,
                    )),
            );
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("Repeat from"),
                    )
                    .child(segmented_row(
                        theme,
                        "repeat-schedule",
                        &[("completion", "Completion"), ("assigned", "Assigned Dates")],
                        if schedule == RepeatSchedule::AfterCompletion {
                            0
                        } else {
                            1
                        },
                        self.theme_boxes.clone(),
                        {
                            let current_rule = rule.clone();
                            let schedule_id = id.clone();
                            move |this: &mut FlowlistApp, index, cx| {
                                if let Some(current) = &current_rule {
                                    this.update_task_repeat(
                                        &schedule_id,
                                        current,
                                        |rule| {
                                            rule.schedule = if index == 0 {
                                                RepeatSchedule::AfterCompletion
                                            } else {
                                                RepeatSchedule::Fixed
                                            }
                                        },
                                        cx,
                                    );
                                }
                            }
                        },
                        cx,
                    )),
            );
            if let Some(rule) = &rule {
                body = body.child(
                    div()
                        .text_xs()
                        .whitespace_normal()
                        .text_color(theme.text_faint)
                        .child(rule.summary()),
                );
            }
        }
        body
    }

    fn inspector_repeat_interval(
        &self,
        id: &TaskId,
        every: u32,
        unit: IntervalUnit,
        weekdays_selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing = !weekdays_selected
            && self.editing == Some(EditField::Repeat)
            && self.focus == super::FocusArea::Field;
        let number = if weekdays_selected {
            div()
                .on_children_prepainted(self.theme_probe("repeat-custom".to_string()))
                .flex_1()
                .px_2()
                .text_sm()
                .text_color(theme.text_faint)
                .child("custom")
                .into_any_element()
        } else if editing {
            div()
                .on_children_prepainted(self.field_editor_probe())
                .id("repeat-interval-editor")
                .flex_1()
                .px_2()
                .child(widgets::inline_text_field_with_cursor(
                    theme,
                    &self.field,
                    "1",
                    true,
                    cx.listener(|this, hit: &widgets::TextHit, _, cx| this.field_text_hit(hit, cx)),
                ))
                .into_any_element()
        } else {
            div()
                .on_children_prepainted(self.theme_probe("field-box:repeat-every".to_string()))
                .id("field:repeat-every")
                .flex_1()
                .px_2()
                .text_sm()
                .cursor_pointer()
                .child(every.to_string())
                .on_click(cx.listener(|this, _, _, cx| this.begin_edit(EditField::Repeat, cx)))
                .into_any_element()
        };
        let anchor = self.repeat_unit_anchor.clone();
        let probe_boxes = self.notification_boxes.clone();
        let unit_id = id.clone();
        let unit_label = interval_unit_label(unit);
        div()
            .flex()
            .items_center()
            .h(px(28.))
            .rounded_md()
            .border_1()
            .border_color(if editing { theme.accent } else { theme.border })
            .bg(theme.field)
            .overflow_hidden()
            .child(number)
            .child(
                div()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(bounds) = bounds.first() {
                            anchor.set(Some(*bounds));
                            probe_boxes.borrow_mut().insert(
                                "repeat-unit-toggle".to_string(),
                                [
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ],
                            );
                        }
                    })
                    .id("repeat-unit-toggle")
                    .flex()
                    .items_center()
                    .gap_1()
                    .h_full()
                    .px_2()
                    .border_l_1()
                    .border_color(theme.border)
                    .text_sm()
                    .text_color(theme.text_dim)
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(unit_label)
                    .child(icons::icon(Icon::ChevronDown, 10.0, theme.text_faint))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.repeat_unit_menu = Some(unit_id.clone());
                        cx.notify();
                    })),
            )
    }

    fn inspector_week_strip(
        &self,
        id: &TaskId,
        rule: &Option<RepeatRule>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let selected = rule
            .as_ref()
            .map(|rule| rule.selected_weekdays())
            .unwrap_or_default();
        let letters = ["S", "M", "T", "W", "T", "F", "S"];
        let mut strip = div()
            .flex()
            .items_center()
            .h(px(30.))
            .w_full()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();
        for (day, letter) in letters.iter().enumerate() {
            let day = day as u8;
            let current_rule = rule.clone();
            let strip_id = id.clone();
            let active = selected.contains(&day);
            strip = strip.child(
                div()
                    .on_children_prepainted(self.theme_probe(format!("repeat-weekday:{day}")))
                    .id(super::sid(format!("repeat-weekday:{day}")))
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(day > 0, |this| this.border_l_1().border_color(theme.border))
                    .bg(if active {
                        theme.selection
                    } else {
                        theme.field
                    })
                    .text_xs()
                    .text_color(if active { theme.accent } else { theme.text_dim })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(*letter)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(current) = &current_rule {
                            let mut days = current.selected_weekdays();
                            if days.contains(&day) {
                                days.remove(&day);
                            } else {
                                days.insert(day);
                            }
                            this.update_task_repeat(
                                &strip_id,
                                current,
                                |rule| {
                                    rule.set_selected_weekdays(days);
                                    if !rule.selected_weekdays().is_empty() {
                                        rule.every = 1;
                                        rule.unit = IntervalUnit::Days;
                                    }
                                },
                                cx,
                            );
                        }
                    })),
            );
        }
        strip
    }

    pub(super) fn render_repeat_unit_menu(
        &self,
        id: &TaskId,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = self.theme;
        let current = self
            .store
            .db()
            .tasks
            .get(id)
            .and_then(|task| task.repeat.clone())
            .map(|rule| rule.unit)
            .unwrap_or(IntervalUnit::Weeks);
        let mut menu = div()
            .min_w(px(120.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.raised)
            .shadow_lg();
        for candidate in [
            IntervalUnit::Days,
            IntervalUnit::Weeks,
            IntervalUnit::Months,
            IntervalUnit::Years,
        ] {
            let slug = interval_unit_slug(candidate);
            let label = interval_unit_label(candidate);
            let active = current == candidate;
            let menu_id = id.clone();
            menu = menu.child(
                div()
                    .on_children_prepainted(
                        self.notification_probe(format!("repeat-unit-option:{slug}")),
                    )
                    .id(super::sid(format!("repeat-unit-option:{slug}")))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if active {
                        theme.accent
                    } else {
                        theme.text_dim
                    })
                    .bg(if active { theme.selection } else { theme.clear })
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .child(label.to_string())
                    .when(active, |this| {
                        this.child(icons::icon(Icon::Check, 11.0, theme.accent))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(current) = this
                            .store
                            .db()
                            .tasks
                            .get(&menu_id)
                            .and_then(|task| task.repeat.clone())
                        {
                            this.update_task_repeat(
                                &menu_id,
                                &current,
                                |rule| {
                                    rule.unit = candidate;
                                    if candidate != IntervalUnit::Days {
                                        rule.weekdays = None;
                                    }
                                },
                                cx,
                            );
                        }
                        this.repeat_unit_menu = None;
                        this.save_session();
                        cx.notify();
                    })),
            );
        }
        menu
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
        show_label: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let editing = self.editing == Some(field) && self.focus == super::FocusArea::Field;
        let is_title = field == EditField::Title;
        let placeholder = match field {
            EditField::Title => "Task title",
            EditField::Defer => "No defer date",
            EditField::Planned => "No planned date",
            EditField::Due => "No due date",
            EditField::Reminder => "No reminder time",
            EditField::Estimate => "No estimate (e.g. 45m)",
            EditField::Repeat => "1",
            EditField::Project => "Inbox",
            EditField::Notes => "Notes",
        };
        let mut row = div()
            .flex()
            .flex_col()
            .gap_1()
            .when(show_label, |this| {
                this.child(div().text_xs().text_color(theme.text_faint).child(label))
            })
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
                    .on_children_prepainted(self.theme_probe(format!(
                        "field-box:{}",
                        label.to_lowercase().replace(' ', "-")
                    )))
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
                    .child(div().when(is_title, |this| this.line_clamp(2)).child(
                        if value.is_empty() {
                            placeholder.to_string()
                        } else {
                            value.to_string()
                        },
                    ))
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
        suffix: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let label = match suffix {
            Some(suffix) => format!("{} {suffix}", field.label()),
            None => field.label().to_string(),
        };
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
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child(icons::icon(field.icon(), 11.0, theme.text_faint))
                    .child(label),
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
        if let Some((task, error_field, message)) = &self.date_error
            && *error_field == field
            && self.selected_one().as_ref() == Some(task)
        {
            row = row.child(
                div()
                    .text_xs()
                    .text_color(theme.warning)
                    .whitespace_normal()
                    .child(message.clone()),
            );
        }
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

    /// Record a notification control's painted bounds for the GUI probe.
    fn notification_probe(
        &self,
        key: String,
    ) -> impl Fn(Vec<gpui::Bounds<gpui::Pixels>>, &mut Window, &mut gpui::App) + 'static {
        let boxes = self.notification_boxes.clone();
        move |bounds, _, _| {
            if let Some(bounds) = bounds.first() {
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
                // A reminder cannot fire in the past. Earlier days are shown
                // for context but cannot be chosen; today stays live so the
                // clock step can still put it in the future.
                let past_reminder = field == DateField::Reminder && date < today;
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
                        if this.date_picker == Some(DateField::Reminder) && date < local_date(now())
                        {
                            this.toast = Some("A reminder must be in the future".to_string());
                            cx.notify();
                            return;
                        }
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
                if past_reminder {
                    cell = cell
                        .opacity(0.30)
                        .cursor(CursorStyle::Arrow)
                        .hover(|style| style.bg(theme.clear));
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
        let active = self.focus == super::FocusArea::TagInput;
        let mut row = div().flex().flex_wrap().items_center().gap_1();
        for (tag_id, path) in tags {
            let remove = tag_id.clone();
            row = row.child(
                div()
                    .id(super::sid(format!("tag-chip:{tag_id}")))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_0p5()
                    .rounded_md()
                    .bg(theme.raised)
                    .text_color(theme.text_dim)
                    .text_xs()
                    .child(path.clone())
                    .child(
                        div()
                            .id(super::sid(format!("remove-tag:{}", remove)))
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.danger))
                            .flow_tip(self, cx, theme, "Remove tag")
                            .child(icons::icon(Icon::Close, 11.0, theme.text_faint))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(id) = this.selected_one() {
                                    this.execute(Command::SetTaskTags {
                                        ids: vec![id],
                                        add: Vec::new(),
                                        remove: vec![remove.clone()],
                                    });
                                    this.inspector_sections
                                        .insert("task:tags".to_string(), false);
                                    this.save_session();
                                    cx.notify();
                                }
                            })),
                    ),
            );
        }
        row = row.child(if active {
            div()
                .flex_1()
                .min_w(px(140.))
                .id("tag-input")
                .child(widgets::inline_text_field_with_cursor(
                    theme,
                    &self.tag_buffer,
                    "Add tag…",
                    true,
                    cx.listener(|this, hit: &widgets::TextHit, _, cx| this.tag_text_hit(hit, cx)),
                ))
                .into_any_element()
        } else {
            div()
                .on_children_prepainted(self.theme_probe("tag-add".to_string()))
                .id("tag-add")
                .size(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_color(theme.text_faint)
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .child(icons::icon(Icon::Plus, 12.0, theme.text_faint))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.focus = super::FocusArea::TagInput;
                    this.suggestion_index = 0;
                    cx.notify();
                }))
                .into_any_element()
        });
        let mut section = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .p_2()
                    .rounded_lg()
                    .border_1()
                    .border_color(if active {
                        theme.accent
                    } else {
                        theme.border
                    })
                    .bg(theme.field)
                    .cursor_pointer()
                    .when(tags.is_empty() && !active, |this| this.justify_center())
                    .on_children_prepainted(self.theme_probe("tag-box".to_string()))
                    .id("tag-box")
                    .child(row)
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

    fn inspector_tags_readonly(&self, tags: &[String]) -> impl IntoElement {
        let theme = self.theme;
        let mut row = div().flex().flex_wrap().items_center().gap_1().p_2();
        if tags.is_empty() {
            row = row.child(
                div()
                    .text_xs()
                    .text_color(theme.text_faint)
                    .child("No tags"),
            );
        } else {
            for tag in tags {
                row = row.child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded_md()
                        .bg(theme.raised)
                        .text_color(theme.text_dim)
                        .text_xs()
                        .child(tag.clone()),
                );
            }
        }
        row.rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.field)
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
                    cx.listener(|this, hit: &widgets::TextHit, _, cx| this.field_text_hit(hit, cx)),
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
        let icon = project.icon.clone();
        let attachments = project.attachments.clone();
        let project_id = id.clone();
        let icon_project = id.clone();
        let rename_id = id.clone();
        let due = project.due.map(|stamp| human_date(&stamp, now()));
        let defer = project.defer.map(|stamp| human_date(&stamp, now()));
        let planned = project.planned.map(|stamp| human_date(&stamp, now()));
        let next_review = project.next_review.format("%b %-d, %Y").to_string();
        let review_days = project.review_interval_days;
        let has_dates = defer.is_some() || planned.is_some() || due.is_some();
        let has_attachments = !attachments.is_empty();
        let notifications_target = NotificationTarget::Project(id.clone());
        let has_notifications = self.notifications_have_data(&notifications_target);
        let last_reviewed = project
            .last_reviewed
            .map(|date| date.format("%b %-d, %Y").to_string())
            .unwrap_or_else(|| "Never".to_string());
        let review_content = self
            .inspector_review_project(&project_id, &next_review, review_days, &last_reviewed, cx)
            .into_any_element();
        let notifications_content = self.inspector_notifications(notifications_target, cx);
        let meta_content = div()
            .flex()
            .flex_col()
            .gap_2()
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
                            .on_children_prepainted(
                                self.theme_probe("project-icon-toggle".to_string()),
                            )
                            .id("project-icon-toggle")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.field)
                            .cursor_pointer()
                            .flow_tip(self, cx, theme, "Change the project icon")
                            .hover(|style| style.bg(theme.hover))
                            .child(widgets::icon_spec(&icon, 14.0, theme.text_dim))
                            .child(icons::icon(Icon::ChevronDown, 10.0, theme.text_faint))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_modal(
                                    super::modals::Modal::icon_picker(icon_project.clone()),
                                    cx,
                                );
                            })),
                    )
                    .when(!matches!(icon, IconSpec::Emoji(_)), |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(theme.text_faint)
                                .child(icon.label()),
                        )
                    }),
            )
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
            .into_any_element();
        let dates_content = div()
            .flex()
            .flex_col()
            .gap_2()
            .when_some(defer, |this, defer| {
                this.child(self.inspector_readonly("Defer", &defer))
            })
            .when_some(planned, |this, planned| {
                this.child(self.inspector_readonly("Planned", &planned))
            })
            .when_some(due, |this, due| {
                this.child(self.inspector_readonly("Due", &due))
            })
            .into_any_element();
        let tags_content = self.inspector_tags_readonly(&tags).into_any_element();
        let attachments_content = self
            .inspector_attachments(
                &attachments,
                super::modals::AttachmentTarget::Project(id.clone()),
                window,
                cx,
            )
            .into_any_element();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child(self.inspector_section("project:overview", "Project", true, meta_content, cx))
            .child(self.inspector_section(
                "project:dates",
                "Dates",
                has_dates,
                dates_content,
                cx,
            ))
            .child(self.inspector_section("project:review", "Review", true, review_content, cx))
            .child(self.inspector_section(
                "project:notifications",
                "Notifications",
                has_notifications,
                notifications_content,
                cx,
            ))
            .child(self.inspector_section(
                "project:tags",
                "Tags",
                !tags.is_empty(),
                tags_content,
                cx,
            ))
            .child(self.inspector_section(
                "project:attachments",
                "Attachments",
                has_attachments,
                attachments_content,
                cx,
            ))
            .into_any_element()
    }

    fn inspector_review_project(
        &self,
        id: &flowlist_core::ids::ProjectId,
        next_review: &str,
        review_days: u32,
        last_reviewed: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme;
        let step = review_step_days(review_days);
        let label = review_interval_label(review_days);
        let open_id = id.clone();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child("Next Review"),
                            )
                            .child(div().text_sm().child(next_review.to_string())),
                    )
                    .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("project-review-edit".to_string()),
                            )
                            .id("project-review-edit")
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_xs()
                            .text_color(theme.accent)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child("Change")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_modal(
                                    super::modals::Modal::review_interval(this, open_id.clone()),
                                    cx,
                                );
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("Review Every"),
                    )
                    .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("review-every-minus".to_string()),
                            )
                            .id("review-every-minus")
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child("−")
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| {
                                    this.set_project_review_interval(
                                        &id,
                                        review_days.saturating_sub(step).max(1),
                                        cx,
                                    );
                                }
                            })),
                    )
                    .child(div().min_w(px(70.)).text_center().text_sm().child(label))
                    .child(
                        div()
                            .on_children_prepainted(
                                self.theme_probe("review-every-plus".to_string()),
                            )
                            .id("review-every-plus")
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .border_1()
                            .border_color(theme.border)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover))
                            .child("+")
                            .on_click(cx.listener({
                                let id = id.clone();
                                move |this, _, _, cx| {
                                    this.set_project_review_interval(
                                        &id,
                                        review_days.saturating_add(step),
                                        cx,
                                    );
                                }
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(82.))
                            .text_xs()
                            .text_color(theme.text_faint)
                            .child("Last Reviewed"),
                    )
                    .child(div().text_sm().text_color(theme.text_dim).child(last_reviewed.to_string())),
            )
    }

    fn set_project_review_interval(
        &mut self,
        id: &flowlist_core::ids::ProjectId,
        days: u32,
        cx: &mut Context<Self>,
    ) {
        self.execute(Command::UpdateProject {
            id: id.clone(),
            patch: flowlist_core::command::ProjectPatch {
                review_interval_days: Some(days.max(1)),
                ..Default::default()
            },
        });
        cx.notify();
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
        // Parallel and Sequential show child tasks; One-off shows the checklist
        // steps. The inactive list is kept in the data but does not appear here.
        let (done, total) = if one_off {
            (
                checklist.iter().filter(|item| item.2).count(),
                checklist.len(),
            )
        } else {
            (
                children.iter().filter(|child| child.2).count(),
                children.len(),
            )
        };

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

        if !one_off {
            for (child_id, title, completed, priority) in children {
                let blocked_by = if completed {
                    None
                } else {
                    flowlist_core::query::availability::sequence_blocker(db, &child_id)
                        .and_then(|id| db.tasks.get(&id))
                        .map(|task| task.title.clone())
                };
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
                let separate_boxes = self.inspector_boxes.clone();
                let separate_key = format!("separate:{child_id}");
                let delete_boxes = self.inspector_boxes.clone();
                let delete_key = format!("delete:{child_id}");
                let grip_boxes = self.inspector_boxes.clone();
                let grip_key = format!("grip:{child_id}");
                let blocked = blocked_by.clone();
                let toggle_tip = if let Some(blocker) = &blocked_by {
                    format!("Finish “{blocker}” first — this is a sequential list")
                } else if completed {
                    "Mark subtask incomplete".to_string()
                } else {
                    "Mark subtask complete".to_string()
                };
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
                            .when(blocked_by.is_some(), |this| this.opacity(0.45))
                            .flow_tip(self, cx, theme, toggle_tip)
                            .child(widgets::checkbox(
                                theme,
                                completed,
                                theme.accent,
                                priority
                                    .map(|priority| super::theme::priority_color(theme, priority)),
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if let Some(blocker) = blocked.clone() {
                                    this.toast = Some(format!(
                                        "Finish “{blocker}” first — this is a sequential list"
                                    ));
                                    cx.notify();
                                    return;
                                }
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
                                .on_children_prepainted(move |bounds, _, _| {
                                    if let Some(bounds) = bounds.first() {
                                        separate_boxes.borrow_mut().insert(
                                            separate_key.clone(),
                                            [
                                                f32::from(bounds.origin.x),
                                                f32::from(bounds.origin.y),
                                                f32::from(bounds.size.width),
                                                f32::from(bounds.size.height),
                                            ],
                                        );
                                    }
                                })
                                .id(super::sid(format!("separate-subtask:{child_id}")))
                                .flex_shrink_0()
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
                            .on_children_prepainted(move |bounds, _, _| {
                                if let Some(bounds) = bounds.first() {
                                    delete_boxes.borrow_mut().insert(
                                        delete_key.clone(),
                                        [
                                            f32::from(bounds.origin.x),
                                            f32::from(bounds.origin.y),
                                            f32::from(bounds.size.width),
                                            f32::from(bounds.size.height),
                                        ],
                                    );
                                }
                            })
                            .id(super::sid(format!("delete-subtask:{child_id}")))
                            .flex_shrink_0()
                            .ml(px(10.))
                            .w(px(18.))
                            .text_center()
                            .text_xs()
                            .text_color(theme.text_faint)
                            .cursor_pointer()
                            .hover(|style| style.text_color(theme.danger))
                            .flow_tip(self, cx, theme, "Move subtask to Trash")
                            .child(icons::icon(Icon::Close, 12.0, theme.text_faint))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.trash_subtask(trash_id.clone(), cx);
                            })),
                    )
                    .child(
                        div()
                            .on_children_prepainted(move |bounds, _, _| {
                                if let Some(bounds) = bounds.first() {
                                    grip_boxes.borrow_mut().insert(
                                        grip_key.clone(),
                                        [
                                            f32::from(bounds.origin.x),
                                            f32::from(bounds.origin.y),
                                            f32::from(bounds.size.width),
                                            f32::from(bounds.size.height),
                                        ],
                                    );
                                }
                            })
                            .id(super::sid(format!("subtask-grip:{child_id}")))
                            .flex_shrink_0()
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
        }

        if one_off {
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
                                .child(widgets::checkbox(theme, *completed, theme.accent, None))
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
        }

        if total == 0 {
            section = section.child(div().text_xs().text_color(theme.text_faint).child(
                if one_off {
                    "No one-off steps yet."
                } else {
                    "No subtasks yet."
                },
            ));
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
                        this.favicons
                            .borrow_mut()
                            .insert(host.clone(), std::sync::Arc::new(image));
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
