//! The full-screen due reminder.
//!
//! A task that comes due while Flowlist is running can take over the screen
//! instead of waiting as a notification banner. The window is a borderless
//! pop-up that joins every Space, so it is visible over full-screen apps too.
//! Completing goes through the ordinary command layer, which keeps Undo
//! working in the main window.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Result;
use flowlist_core::ids::TaskId;
use gpui::{
    App, Bounds, Context, CursorStyle, FocusHandle, IntoElement, KeyDownEvent, MouseButton,
    ObjectFit, Render, WeakEntity, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div, img, point, prelude::*, px, size,
};

use super::FlowlistApp;
use super::background::{self, AbstractStyle};
use super::icons::{self, Icon};
use super::theme::Theme;

#[derive(Clone)]
pub enum ReminderBackdrop {
    Abstract(AbstractStyle),
    Wallpaper(PathBuf, f32),
}

#[derive(Clone, Debug)]
pub struct ReminderTask {
    pub id: TaskId,
    pub title: String,
    pub project: Option<String>,
    pub time: String,
    pub due_at: chrono::DateTime<chrono::Utc>,
}

pub struct Reminder {
    app: WeakEntity<FlowlistApp>,
    theme: Theme,
    backdrop: ReminderBackdrop,
    snooze_minutes: u32,
    tasks: Vec<ReminderTask>,
    index: usize,
    /// The app that owned the screen before the reminder appeared, restored
    /// after Complete, Snooze, Dismiss or Esc. Opening the task keeps Flowlist
    /// in front instead.
    previous_app: Option<i32>,
    focus_handle: FocusHandle,
}

impl Reminder {
    fn new(
        app: WeakEntity<FlowlistApp>,
        theme: Theme,
        backdrop: ReminderBackdrop,
        snooze_minutes: u32,
        tasks: Vec<ReminderTask>,
        previous_app: Option<i32>,
        pause_media: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        if pause_media && super::media::is_audio_playing() {
            let _ = super::media::pause();
        }
        Self {
            app,
            theme,
            backdrop,
            snooze_minutes,
            tasks,
            index: 0,
            previous_app,
            focus_handle: cx.focus_handle(),
        }
    }

    fn reset(
        &mut self,
        theme: Theme,
        backdrop: ReminderBackdrop,
        snooze_minutes: u32,
        tasks: Vec<ReminderTask>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme = theme;
        self.backdrop = backdrop;
        self.snooze_minutes = snooze_minutes;
        let was_empty = self.tasks.is_empty();
        for task in tasks {
            if !self.tasks.iter().any(|existing| existing.id == task.id) {
                self.tasks.push(task);
            }
        }
        if was_empty {
            self.index = 0;
        }
        window.activate_window();
        window.focus(&self.focus_handle);
        cx.activate(true);
        cx.notify();
    }

    fn current(&self) -> Option<&ReminderTask> {
        self.tasks.get(self.index)
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>, restore_previous: bool) {
        if let Some(app) = self.app.upgrade() {
            let _ = app.update(cx, |app, _| app.reminder_closed());
        }
        window.remove_window();
        if restore_previous
            && let Some(pid) = self.previous_app.take()
        {
            super::quick_capture::restore_app(pid);
        }
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>, restore_previous: bool) {
        self.index += 1;
        if self.index >= self.tasks.len() {
            self.close(window, cx, restore_previous);
        } else {
            cx.notify();
        }
    }

    fn complete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.current().cloned() else {
            self.close(window, cx, true);
            return;
        };
        if let Some(app) = self.app.upgrade() {
            let _ = app.update(cx, |app, cx| {
                app.complete_reminder_task(&task.id, cx);
                app.dismiss_reminder_task(&task.id);
            });
        }
        self.advance(window, cx, true);
    }

    fn snooze(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.current().cloned() else {
            self.close(window, cx, true);
            return;
        };
        if let Some(app) = self.app.upgrade() {
            let minutes = self.snooze_minutes;
            let _ = app.update(cx, |app, _| {
                app.snooze_reminder_task(&task.id, minutes as i64);
                app.set_reminder_snooze_minutes(minutes);
                app.dismiss_reminder_task(&task.id);
            });
        }
        self.advance(window, cx, true);
    }

    /// `s` walks the three lengths the reminder offers, so the choice is made
    /// without leaving the full-screen view.
    fn cycle_snooze(&mut self, cx: &mut Context<Self>) {
        const CHOICES: [u32; 3] = [15, 30, 60];
        let next = match CHOICES.iter().position(|minutes| *minutes == self.snooze_minutes) {
            Some(index) => CHOICES[(index + 1) % CHOICES.len()],
            None => CHOICES[0],
        };
        self.snooze_minutes = next;
        cx.notify();
    }

    fn view_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(app) = self.app.upgrade() {
            let _ = app.update(cx, |app, cx| app.open_fullscreen_view(cx));
        }
        self.close(window, cx, false);
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.current().cloned() else {
            self.close(window, cx, false);
            return;
        };
        if let Some(app) = self.app.upgrade() {
            let _ = app.update(cx, |app, cx| app.open_reminder_task(&task.id, cx));
        }
        self.close(window, cx, false);
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Dismiss closes the whole batch, not just the card on top. The other
        // device is told about every task so it can clear its matching cards.
        let ids: Vec<TaskId> = self.tasks.iter().map(|task| task.id.clone()).collect();
        if let Some(app) = self.app.upgrade() {
            let _ = app.update(cx, |app, _| {
                for id in &ids {
                    app.dismiss_reminder_task(id);
                }
            });
        }
        self.close(window, cx, true);
    }

    /// Remove reminders dismissed on the other device. If the current card is
    /// gone, show the next one, and close the window when none remain.
    fn dismiss_ids(
        &mut self,
        ids: &BTreeSet<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self.tasks.len();
        self.tasks.retain(|task| !ids.contains(&task.id.to_string()));
        if self.tasks.len() == before {
            return;
        }
        if self.tasks.is_empty() {
            self.close(window, cx, true);
            return;
        }
        if self.index >= self.tasks.len() {
            self.index = self.tasks.len() - 1;
        }
        cx.notify();
    }

    fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let command = event.keystroke.modifiers.platform;
        match event.keystroke.key.as_str() {
            "escape" => self.dismiss(window, cx),
            // Enter saves the snooze shown on the button; Command-Enter is the
            // deliberate "done" action so a stray Return cannot complete a task.
            "enter" | "return" if command => self.complete(window, cx),
            "enter" | "return" => self.snooze(window, cx),
            "s" if !command => self.cycle_snooze(cx),
            "v" => self.view_all(window, cx),
            "o" => self.open(window, cx),
            _ => {}
        }
    }

    fn action(
        &self,
        id: &'static str,
        label: impl Into<String>,
        icon: Icon,
        primary: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let label = label.into();
        let theme = self.theme;
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .h(px(46.))
            .px_5()
            .rounded_lg()
            .border_1()
            .border_color(if primary { theme.accent } else { theme.border })
            .bg(if primary { theme.accent } else { theme.raised })
            .text_color(if primary { theme.raised } else { theme.text })
            .text_sm()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|style| {
                if primary {
                    style.opacity(0.9)
                } else {
                    style.bg(theme.hover)
                }
            })
            .child(icons::icon(
                icon,
                17.0,
                if primary { theme.raised } else { theme.text_dim },
            ))
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| {
                match id {
                    "reminder-complete" => this.complete(window, cx),
                    "reminder-snooze" => this.snooze(window, cx),
                    "reminder-view" => this.view_all(window, cx),
                    "reminder-open" => this.open(window, cx),
                    _ => this.dismiss(window, cx),
                }
            }))
    }
}

impl Render for Reminder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let Some(task) = self.current().cloned() else {
            return div().size_full().bg(theme.bg);
        };
        let position = format!("{} of {}", self.index + 1, self.tasks.len());
        let detail = match &task.project {
            Some(project) => format!("{project} · at {}", task.time),
            None => format!("Inbox · at {}", task.time),
        };
        let due_label = if self.tasks.len() > 1 {
            format!("{} reminders due", self.tasks.len())
        } else {
            "Reminder".to_string()
        };
        let backdrop = match &self.backdrop {
            ReminderBackdrop::Abstract(style) => background::render(*style, theme),
            ReminderBackdrop::Wallpaper(path, opacity) => div()
                .absolute()
                .size_full()
                .overflow_hidden()
                .child(
                    img(path.clone())
                        .object_fit(ObjectFit::Cover)
                        .size_full()
                        .opacity(*opacity),
                )
                .into_any_element(),
        };
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(theme.bg)
            .text_color(theme.text)
            .font_family("SF Pro Display")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::Arrow)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus_handle);
                    cx.notify();
                }),
            )
            .child(backdrop)
            .child(
                div()
                    .absolute()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(px(56.))
                    .child(
                        div()
                            .w_full()
                            .max_w(px(920.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_6()
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.bg)
                            .p(px(44.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.accent)
                                    .child(icons::icon(Icon::Clock, 16.0, theme.accent))
                                    .child(due_label),
                            )
                            .child(
                                div()
                                    .max_w(px(800.))
                                    .text_center()
                                    .text_size(px(58.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .line_height(px(68.))
                                    .child(task.title.clone()),
                            )
                            .child(div().text_lg().text_color(theme.text_dim).child(detail))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(self.action(
                                        "reminder-complete",
                                        "Complete",
                                        Icon::Check,
                                        true,
                                        cx,
                                    ))
                                    .child(self.action(
                                        "reminder-snooze",
                                        snooze_label(self.snooze_minutes),
                                        Icon::Clock,
                                        false,
                                        cx,
                                    ))
                                    .child(self.action(
                                        "reminder-open",
                                        "Open task",
                                        Icon::ListChecks,
                                        false,
                                        cx,
                                    ))
                                    .when(self.tasks.len() > 1, |this| {
                                        this.child(self.action(
                                            "reminder-view",
                                            "View all",
                                            Icon::ListChecks,
                                            false,
                                            cx,
                                        ))
                                    })
                                    .child(self.action(
                                        "reminder-dismiss",
                                        "Dismiss",
                                        Icon::Close,
                                        false,
                                        cx,
                                    )),
                            )
                            .child(div().text_xs().text_color(theme.text_faint).child(format!(
                                "{position} · Esc dismisses · S cycles snooze · Enter snoozes · ⌘Enter completes"
                            ))),
                    ),
            )
    }
}

/// Close any open reminder cards whose task ids appear in the other device's
/// dismissal list. Safe to call when no reminder is open.
pub fn dismiss_tasks(ids: &[String], cx: &mut App) {
    if ids.is_empty() {
        return;
    }
    let set: BTreeSet<String> = ids.iter().cloned().collect();
    for any in cx.windows() {
        if let Some(handle) = any.downcast::<Reminder>() {
            let _ = handle.update(cx, |view, window, cx| view.dismiss_ids(&set, window, cx));
        }
    }
}

fn snooze_label(minutes: u32) -> String {
    if minutes == 60 {
        "Snooze 1 hour".to_string()
    } else {
        format!("Snooze {minutes} min")
    }
}

/// Show the reminder, reusing the open reminder window when there is one.
pub fn show(
    app: WeakEntity<FlowlistApp>,
    theme: Theme,
    backdrop: ReminderBackdrop,
    snooze_minutes: u32,
    pause_media: bool,
    tasks: Vec<ReminderTask>,
    cx: &mut App,
) -> Result<()> {
    let previous_app = super::quick_capture::frontmost_app();
    for any in cx.windows() {
        if let Some(handle) = any.downcast::<Reminder>() {
            let updated = handle.update(cx, |view, window, cx| {
                view.reset(
                    theme,
                    backdrop.clone(),
                    snooze_minutes,
                    tasks.clone(),
                    window,
                    cx,
                );
            });
            if updated.is_ok() {
                return Ok(());
            }
        }
    }

    let display = cx
        .primary_display()
        .or_else(|| cx.displays().into_iter().next());
    let bounds = display.map(|display| display.bounds()).unwrap_or_else(|| {
        Bounds::new(point(px(0.), px(0.)), size(px(1280.), px(800.)))
    });
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            focus: true,
            show: true,
            // A pop-up joins every Space and can sit over a full-screen app.
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Opaque,
            ..Default::default()
        },
        move |window, cx| {
            let view = cx.new(|cx| {
                Reminder::new(
                    app.clone(),
                    theme,
                    backdrop,
                    snooze_minutes,
                    tasks,
                    previous_app,
                    pause_media,
                    cx,
                )
            });
            let handle = view.read(cx).focus_handle.clone();
            window.focus(&handle);
            window.activate_window();
            cx.activate(true);
            view
        },
    )?;
    Ok(())
}
