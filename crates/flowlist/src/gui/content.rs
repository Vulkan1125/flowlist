//! The main task list, project dashboards, review queue and nearby places.

use gpui::{AnyElement, Context, IntoElement, MouseButton, Window, div, prelude::*, px};

use flowlist_core::command::Command;
use flowlist_core::ids::{ProjectId, TaskId};
use flowlist_core::model::{BuiltinPerspective, ProjectStatus, RepeatRule};
use flowlist_core::query::{self, Group, TaskRow};
use flowlist_core::time::{Stamp, format_duration, human_date, local_date, now};

use super::drag::{DragTask, ghost};
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
                                    .h(px(26.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_xs()
                                    .text_color(theme.text_dim)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.hover))
                                    .child(format!("Sort · {}", sort_label(self.sort)))
                                    .on_click(cx.listener(|this, _, _, cx| this.cycle_sort(cx))),
                            )
                            .child(
                                div()
                                    .id("group")
                                    .h(px(26.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_xs()
                                    .text_color(theme.text_dim)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.hover))
                                    .child(format!("Group · {}", group_label(self.group)))
                                    .on_click(cx.listener(|this, _, _, cx| this.cycle_group(cx))),
                            )
                            .when(shows_completed, |this| {
                                this.child(
                                    div()
                                        .id("show-completed")
                                        .h(px(26.))
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(if self.show_completed {
                                            theme.accent
                                        } else {
                                            theme.border
                                        })
                                        .bg(if self.show_completed {
                                            theme.selection
                                        } else {
                                            theme.bg
                                        })
                                        .text_xs()
                                        .text_color(if self.show_completed {
                                            theme.accent
                                        } else {
                                            theme.text_dim
                                        })
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.hover))
                                        .child("Completed")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_show_completed(cx)
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id("add-task")
                                    .child(widgets::button(theme, "＋ Task", true))
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
                .child(
                    div()
                        .text_color(theme.accent)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("＋"),
                )
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
                        .text_color(theme.text_faint)
                        .child(if open { "▾" } else { "▸" }),
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
                    .child(if expanded { "▾" } else { "▸" })
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
                    .child("⚑")
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
