//! The floating quick-capture panel.
//!
//! Opened by the system-wide `⌃⌥Space` shortcut or the title-bar plus, it is a
//! small window of its own, so the main workspace, selection and scroll never
//! move. Capturing goes through the same `Store::execute` path as every other
//! edit, which keeps the action undoable from the main window.
//!
//! The panel is intentionally quiet: one large title line, a live parse of the
//! markers (`#tag`, `::Project`, `due:`, `repeat:`), and a footer that says
//! where the task will land. Enter files it and closes; `⌘⏎` files it and
//! reveals it in the main window; `⇧⏎` files it and stays open for the next
//! thought.

// `objc` 0.2 expands its macros with a `cfg(feature = "cargo-clippy")` check
// that Cargo's check-cfg does not know about. Silence that third-party noise,
// the same way `blur.rs` does.
#![allow(unexpected_cfgs)]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use flowlist_core::command::Command;
use flowlist_core::ids::TaskId;
use flowlist_core::parse::{QuickEntry, parse_quick};
use flowlist_core::paths::DataPaths;
use flowlist_core::store::Store;
use flowlist_core::time::{format_duration, human_date, now};
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, FocusHandle, Font, FontWeight, IntoElement,
    KeyDownEvent, MouseButton, Render, SharedString, TextRun, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, font, prelude::*, px, size,
};

use super::FlowlistApp;
use super::icons::{self, Icon};
use super::session::Session;
use super::text::TextBuffer;
use super::theme::{Appearance, Theme, ThemeKind};
use super::{BufferKey, handle_buffer_key};

const PANEL_WIDTH: f32 = 620.0;
const PANEL_HEIGHT: f32 = 150.0;

/// The outcome of trying to file the current text.
enum Commit {
    Empty,
    Added {
        id: Option<TaskId>,
        destination: String,
    },
    Failed,
}

pub struct QuickCapture {
    store: Store,
    theme: Theme,
    /// The app that was frontmost before the panel opened, so Esc and a plain
    /// capture can hand focus back instead of leaving Flowlist in front.
    previous_app: Option<i32>,
    buffer: TextBuffer,
    focus_handle: FocusHandle,
    error: Option<String>,
    flash: Option<String>,
    last_captured: Option<String>,
    test_state_path: Option<PathBuf>,
    test_state_last: Option<String>,
    /// Set just before the window goes away, so a last frame cannot overwrite
    /// the `open: false` state with a stale `true`.
    closing: bool,
    window_active: bool,
    active_window: Option<u64>,
}

impl QuickCapture {
    fn new(store: Store, theme: Theme, previous_app: Option<i32>, cx: &mut Context<Self>) -> Self {
        Self {
            store,
            theme,
            previous_app,
            buffer: TextBuffer::empty(),
            focus_handle: cx.focus_handle(),
            error: None,
            flash: None,
            last_captured: None,
            test_state_path: std::env::var_os("FLOWLIST_TEST_STATE")
                .map(PathBuf::from)
                .map(|path| path.with_file_name("capture.json")),
            test_state_last: None,
            closing: false,
            window_active: false,
            active_window: None,
        }
    }

    /// Bring an already-open panel back for another capture. The text is kept
    /// and selected so a new thought replaces it without a trip to Backspace.
    fn reopen(
        &mut self,
        theme: Theme,
        previous_app: Option<i32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme = theme;
        self.previous_app = previous_app;
        self.closing = false;
        self.error = None;
        self.flash = None;
        let _ = self.store.reload_if_changed();
        if !self.buffer.is_empty() {
            self.buffer.select_all();
        }
        window.defer(cx, |window, cx| {
            float_panel(window);
            activate_app();
            cx.activate(true);
            window.activate_window();
        });
        window.focus(&self.focus_handle);
        cx.notify();
    }

    /// Where the capture will land, for the footer. A plain thought stays in
    /// the Inbox; an explicit project wins, and a due or defer date schedules
    /// it into Today or Forecast.
    fn destination(&self, entry: Option<&QuickEntry>) -> String {
        let Some(entry) = entry else {
            return "Inbox".to_string();
        };
        if let Some(path) = &entry.project_path {
            return path.clone();
        }
        if let Some(due) = entry.due {
            return human_date(&due, now());
        }
        if let Some(defer) = entry.defer {
            return format!("Defer {}", human_date(&defer, now()));
        }
        "Inbox".to_string()
    }

    fn destination_icon(&self, entry: Option<&QuickEntry>) -> Icon {
        let Some(entry) = entry else {
            return Icon::Inbox;
        };
        if entry.project_path.is_some() {
            Icon::Project
        } else if entry.due.is_some() {
            Icon::Calendar
        } else if entry.defer.is_some() {
            Icon::Clock
        } else {
            Icon::Inbox
        }
    }

    /// Parse and file the text. Errors stay on screen; a success is handed
    /// back to the caller so Enter, `⌘⏎` and `⇧⏎` can each finish differently.
    fn commit(&mut self) -> Commit {
        let text = self.buffer.text().trim().to_string();
        if text.is_empty() {
            return Commit::Empty;
        }
        let entry = match parse_quick(&text, now()) {
            Ok(entry) => entry,
            Err(error) => {
                self.error = Some(error.to_string());
                return Commit::Failed;
            }
        };
        if entry.title.trim().is_empty() {
            self.error = Some("Add a title before the markers".to_string());
            return Commit::Failed;
        }
        let task = entry.into_new_task();
        match self.store.execute(Command::CreateTask(task)) {
            Ok(outcome) => {
                self.error = None;
                self.last_captured = Some(entry.title.clone());
                Commit::Added {
                    id: outcome
                        .created
                        .first()
                        .map(|raw| TaskId::from_string(raw.clone())),
                    destination: self.destination(Some(&entry)),
                }
            }
            Err(error) => {
                self.error = Some(error.to_string());
                Commit::Failed
            }
        }
    }

    fn capture(&mut self, open_in_main: bool, window: &mut Window, cx: &mut Context<Self>) {
        match self.commit() {
            Commit::Empty => self.dismiss(window, cx, true),
            Commit::Added { id, .. } => {
                let reveal = open_in_main && id.is_some();
                if open_in_main && let Some(id) = id {
                    reveal_in_main(id, cx);
                }
                // Revealing in Flowlist means the main window should stay in
                // front; every other exit hands focus back to the old app.
                self.dismiss(window, cx, !reveal);
            }
            Commit::Failed => cx.notify(),
        }
    }

    /// File the text and stay open, for a run of small thoughts.
    fn capture_keep(&mut self, cx: &mut Context<Self>) {
        match self.commit() {
            Commit::Empty => {}
            Commit::Added { destination, .. } => {
                self.flash = Some(format!("Added to {destination}"));
                self.buffer.clear();
                cx.notify();
            }
            Commit::Failed => cx.notify(),
        }
    }

    fn dismiss(&mut self, window: &mut Window, _cx: &mut Context<Self>, restore_previous: bool) {
        self.closing = true;
        self.write_state(false);
        let previous = self.previous_app.take();
        window.remove_window();
        if restore_previous && let Some(pid) = previous {
            restore_app(pid);
        }
    }

    fn write_state(&mut self, open: bool) {
        let Some(path) = self.test_state_path.clone() else {
            return;
        };
        let text = self.buffer.text().to_string();
        let trimmed = text.trim().to_string();
        let parsed = if trimmed.is_empty() {
            None
        } else {
            Some(parse_quick(&trimmed, now()))
        };
        let (title, parse_error) = match &parsed {
            Some(Ok(entry)) => (entry.title.clone(), None),
            Some(Err(error)) => (String::new(), Some(error.to_string())),
            None => (String::new(), None),
        };
        let entry = parsed.as_ref().and_then(|parsed| parsed.as_ref().ok());
        let state = serde_json::json!({
            "open": open,
            "text": text,
            "title": title,
            "error": self.error.clone().or(parse_error),
            "destination": self.destination(entry),
            "last_captured": self.last_captured,
            "flash": self.flash,
            "window_active": self.window_active,
            "active_window": self.active_window,
            "app_active": app_is_active(),
        });
        let serialized = state.to_string();
        if self.test_state_last.as_deref() == Some(serialized.as_str()) {
            return;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&path, &serialized).is_ok() {
            self.test_state_last = Some(serialized);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        let mods = stroke.modifiers;

        if stroke.key == "f12"
            && let Some(path) = super::test_shot_path()
        {
            super::capture_window_png(window, &path);
            return;
        }

        if mods.control && mods.alt && stroke.key == "space" {
            mark_toggle();
            self.dismiss(window, cx, true);
            return;
        }
        if mods.platform && matches!(stroke.key.as_str(), "enter" | "return") {
            self.capture(true, window, cx);
            return;
        }
        if mods.shift && matches!(stroke.key.as_str(), "enter" | "return") {
            self.capture_keep(cx);
            return;
        }
        if stroke.key == "escape" {
            self.dismiss(window, cx, true);
            return;
        }

        let paste = if mods.platform && stroke.key == "v" {
            cx.read_from_clipboard().and_then(|item| item.text())
        } else {
            None
        };
        if mods.platform && stroke.key == "a" {
            self.buffer.select_all();
            cx.notify();
            return;
        }
        if mods.platform && stroke.key == "c" {
            let text = self.buffer.selected_text();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            return;
        }
        if mods.platform && stroke.key == "x" {
            let text = self.buffer.selected_text();
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.buffer.delete_selection();
            cx.notify();
            return;
        }
        if mods.platform && stroke.key == "w" {
            self.dismiss(window, cx, true);
            return;
        }

        match handle_buffer_key(&mut self.buffer, stroke, paste.as_deref()) {
            BufferKey::Changed => {
                self.error = None;
                self.flash = None;
                cx.notify();
            }
            BufferKey::Submit => self.capture(false, window, cx),
            BufferKey::Cancel => self.dismiss(window, cx, true),
            BufferKey::Tab | BufferKey::Ignored => {}
        }
    }

    fn render_input(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let focused = true;
        div()
            .flex()
            .items_center()
            .gap_3()
            .px(px(22.))
            .mt(px(12.))
            .h(px(34.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(30.))
                    .rounded_full()
                    .bg(theme.accent.opacity(0.14))
                    .child(icons::icon(Icon::Plus, 15.0, theme.accent)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(field_text(
                        theme,
                        &self.buffer,
                        "What's on your mind?",
                        focused,
                        window,
                    ))
                    .cursor(CursorStyle::IBeam),
            )
            .child(key_hint(theme, "esc"))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus_handle);
                    cx.notify();
                }),
            )
    }

    fn render_hints(&self) -> impl IntoElement {
        let theme = self.theme;
        let mut row = div()
            .flex()
            .items_center()
            .gap_2()
            .px(px(22.))
            .mt(px(10.))
            .h(px(24.))
            .overflow_hidden();
        for (token, color) in [
            ("#tag", theme.accent),
            ("::Project", theme.accent),
            ("!", theme.warning),
            ("@1 high", theme.warning),
            ("due:tomorrow", theme.text_dim),
            ("fsr:1h", theme.accent),
            ("repeat:every week", theme.text_dim),
        ] {
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .h(px(20.))
                    .px(px(6.))
                    .rounded_lg()
                    .bg(color.opacity(0.08))
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color)
                    .child(token),
            );
        }
        row
    }

    fn render_preview(&self) -> impl IntoElement {
        let theme = self.theme;
        let text = self.buffer.text().trim().to_string();
        if text.is_empty() {
            return self.render_hints().into_any_element();
        }
        let parsed = parse_quick(&text, now());
        let mut row = div()
            .flex()
            .items_center()
            .gap_2()
            .px(px(22.))
            .mt(px(10.))
            .h(px(24.))
            .overflow_hidden();
        let entry = match parsed {
            Ok(entry) => entry,
            Err(error) => {
                return row
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(theme.warning)
                            .child(format!("Could not read that: {error}")),
                    )
                    .into_any_element();
            }
        };

        // The title already shows in the field above, so the row spends its
        // width on what parsing decided. At most three chips fit beside the
        // gutter at the widest chip; the rest collapse into a count rather
        // than hanging over the panel edge half-drawn.
        let mut items: Vec<(Icon, String, gpui::Hsla)> = Vec::new();
        if let Some(project) = &entry.project_path {
            items.push((Icon::Project, project.clone(), theme.accent));
        }
        if let Some(due) = entry.due {
            items.push((Icon::Calendar, human_date(&due, now()), theme.accent));
        }
        if entry.fullscreen_reminder {
            let label = entry
                .reminder_at
                .map(|stamp| human_date(&stamp, now()))
                .unwrap_or_else(|| "Follows due".to_string());
            items.push((Icon::Clock, format!("FSR {label}"), theme.accent));
        }
        if let Some(defer) = entry.defer {
            items.push((
                Icon::Clock,
                format!("Defer {}", human_date(&defer, now())),
                theme.text_dim,
            ));
        }
        if let Some(planned) = entry.planned {
            items.push((
                Icon::Calendar,
                format!("Plan {}", human_date(&planned, now())),
                theme.text_dim,
            ));
        }
        if let Some(minutes) = entry.estimate_min {
            items.push((Icon::Clock, format_duration(minutes), theme.text_dim));
        }
        if let Some(repeat) = &entry.repeat {
            items.push((Icon::Repeat, repeat.description(), theme.success));
        }
        if entry.flagged {
            items.push((Icon::Flagged, "Flagged".to_string(), theme.warning));
        }
        for tag in &entry.tag_paths {
            items.push((Icon::Tag, format!("#{tag}"), theme.accent));
        }
        const SHOWN: usize = 3;
        let total = items.len();
        for (icon, label, color) in items.into_iter().take(SHOWN) {
            row = row.child(chip(icon, label, color));
        }
        if total > SHOWN {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .h(px(21.))
                    .px(px(7.))
                    .flex()
                    .items_center()
                    .rounded_lg()
                    .bg(theme.raised)
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_dim)
                    .child(format!("+{}", total - SHOWN)),
            );
        }
        let any = total > 0 || self.flash.is_some();
        if !any {
            return self.render_hints().into_any_element();
        }
        row.into_any_element()
    }

    fn render_footer(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let text = self.buffer.text().trim().to_string();
        let parsed = if text.is_empty() {
            None
        } else {
            parse_quick(&text, now()).ok()
        };
        let destination = self.destination(parsed.as_ref());
        let icon = self.destination_icon(parsed.as_ref());
        let mut left = div()
            .flex()
            .items_center()
            .gap_2()
            .child(icons::icon(icon, 13.0, theme.text_faint))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_dim)
                    .child(destination),
            );
        if let Some(flash) = &self.flash {
            left = left
                .child(
                    div()
                        .size(px(3.))
                        .rounded_full()
                        .bg(theme.text_faint.opacity(0.5)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.success)
                        .child(flash.clone()),
                );
        } else {
            left = left
                .child(
                    div()
                        .size(px(3.))
                        .rounded_full()
                        .bg(theme.text_faint.opacity(0.5)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .child("Quick capture"),
                );
        }

        div()
            .flex()
            .items_center()
            .justify_between()
            .mt(px(10.))
            .h(px(38.))
            .px(px(22.))
            .border_t_1()
            .border_color(theme.border_soft)
            .child(left)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(key_hint(theme, "⏎"))
                            .child(label(theme, "Add")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(key_hint(theme, "⌘⏎"))
                            .child(label(theme, "Add & open")),
                    ),
            )
    }
}

impl Render for QuickCapture {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.window_active = window.is_window_active();
        self.active_window = cx.active_window().map(|handle| handle.window_id().as_u64());
        if !self.closing {
            self.write_state(true);
        }
        let theme = self.theme;
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .rounded_none()
            .overflow_hidden()
            .border_1()
            .border_color(theme.border)
            // An opaque surface keeps glyph edges crisp; the old glass wash
            // made small text look soft against the desktop behind it.
            .bg(theme.bg)
            .text_color(theme.text)
            .font_family("SF Pro Text")
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
            // The grab handle also marks the window's drag band.
            .child(
                div().flex().justify_center().pt(px(8.)).child(
                    div()
                        .w(px(40.))
                        .h(px(4.))
                        .rounded_full()
                        .bg(theme.text_faint.opacity(0.30)),
                ),
            )
            .child(self.render_input(window, cx))
            .child(self.render_preview())
            .child(self.render_footer(cx))
    }
}

/// The global shortcut is a toggle: the second press puts the panel away
/// without filing anything. A short debounce absorbs the duplicate event when
/// both the Carbon hotkey and the in-window fallback see the same chord.
static LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);

/// Mark a toggle that was handled locally, so the global hotkey callback
/// ignores the duplicate event a moment later.
pub(super) fn mark_toggle() {
    if let Ok(mut last) = LAST_TOGGLE.lock() {
        *last = Some(Instant::now());
    }
}

pub fn toggle(
    cx: &mut App,
    paths: DataPaths,
    theme: Theme,
    anchor: Option<Bounds<gpui::Pixels>>,
) -> Result<()> {
    let now = Instant::now();
    if let Ok(mut last) = LAST_TOGGLE.lock() {
        if last.is_some_and(|last| now.duration_since(last) < Duration::from_millis(180)) {
            return Ok(());
        }
        *last = Some(now);
    }
    for any in cx.windows() {
        if let Some(handle) = any.downcast::<QuickCapture>() {
            handle.update(cx, |view, window, cx| {
                view.dismiss(window, cx, true);
            })?;
            return Ok(());
        }
    }
    show(cx, paths, theme, anchor)
}

/// Open or bring forward the capture panel.
pub fn show(
    cx: &mut App,
    paths: DataPaths,
    theme: Theme,
    anchor: Option<Bounds<gpui::Pixels>>,
) -> Result<()> {
    // Remember who was in front before Flowlist takes over, so a plain capture
    // or Esc can put them back exactly where they were.
    let previous_app = frontmost_app();
    // One panel at a time. Reusing it keeps the glass, and the theme, warm.
    for any in cx.windows() {
        if let Some(handle) = any.downcast::<QuickCapture>() {
            activate_app();
            cx.activate(true);
            let updated = handle.update(cx, |view, window, cx| {
                view.reopen(theme, previous_app, window, cx);
                window.activate_window();
            });
            if updated.is_ok() {
                return Ok(());
            }
        }
    }

    let store = Store::open(paths)?;
    let bounds = panel_bounds(cx, anchor);
    activate_app();
    cx.activate(true);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // No titlebar controls: the card itself is the whole window. With
            // a transparent titlebar AppKit still gives the panel its native
            // rounded corners and shadow.
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Floating,
            is_movable: true,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Opaque,
            window_min_size: Some(size(px(PANEL_WIDTH), px(PANEL_HEIGHT))),
            ..Default::default()
        },
        move |window, cx| {
            let view = cx.new(|cx| QuickCapture::new(store, theme, previous_app, cx));
            let handle = view.read(cx).focus_handle.clone();
            window.focus(&handle);
            // A global shortcut fires while another app owns the menu bar.
            // Activating on the next turn, after the window exists, is what
            // makes the panel key instead of merely visible.
            window.defer(cx, |window, cx| {
                float_panel(window);
                activate_app();
                cx.activate(true);
                window.activate_window();
            });
            view
        },
    )?;
    activate_app();
    cx.activate(true);
    Ok(())
}

/// The palette the main window would use even when it is closed.
pub fn stored_theme(paths: &DataPaths) -> Theme {
    let session = std::fs::read_to_string(paths.session())
        .ok()
        .and_then(|text| serde_json::from_str::<Session>(&text).ok());
    let (kind, appearance) = session
        .as_ref()
        .map(|session| session.theme_choice())
        .unwrap_or((ThemeKind::Plain, Appearance::Light));
    let accent = session
        .as_ref()
        .and_then(|session| session.plain_accent(appearance));
    Theme::with_accent(kind, appearance, accent)
}

/// Bring the app forward the way a global launcher does. GPUI's
/// `App::activate` uses the older `NSApplication` call, which macOS can ignore
/// for a process launched from a bare executable; `NSRunningApplication`'s
/// modern activation is the reliable route.
#[cfg(target_os = "macos")]
fn activate_app() {
    use objc::runtime::{BOOL, Object};
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let running: *mut Object = msg_send![class!(NSRunningApplication), currentApplication];
        if !running.is_null() {
            // NSApplicationActivateIgnoringOtherApps
            let _: BOOL = msg_send![running, activateWithOptions: 2u64];
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn activate_app() {}

/// The process in front right now, unless it is Flowlist itself.
#[cfg(target_os = "macos")]
pub(super) fn frontmost_app() -> Option<i32> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace.is_null() {
            return None;
        }
        let front: *mut Object = msg_send![workspace, frontmostApplication];
        if front.is_null() {
            return None;
        }
        let pid: i32 = msg_send![front, processIdentifier];
        let own: *mut Object = msg_send![class!(NSRunningApplication), currentApplication];
        let own_pid: i32 = if own.is_null() {
            std::process::id() as i32
        } else {
            msg_send![own, processIdentifier]
        };
        (pid > 0 && pid != own_pid).then_some(pid)
    }
}

#[cfg(not(target_os = "macos"))]
fn frontmost_app() -> Option<i32> {
    None
}

/// Put the app that owned the screen before capture back in front. This is the
/// Things-style exit: the floating panel disappears and the old window keeps
/// its focus, so nothing in Flowlist's workspace moves.
#[cfg(target_os = "macos")]
pub(super) fn restore_app(pid: i32) {
    use objc::runtime::{BOOL, Object};
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let running: *mut Object =
            msg_send![class!(NSRunningApplication), runningApplicationWithProcessIdentifier: pid];
        if running.is_null() {
            return;
        }
        let hidden: BOOL = msg_send![running, isHidden];
        if hidden {
            let _: () = msg_send![running, unhide];
        }
        // NSApplicationActivateIgnoringOtherApps
        let _: BOOL = msg_send![running, activateWithOptions: 2u64];
    }
}

#[cfg(not(target_os = "macos"))]
fn restore_app(_pid: i32) {}

/// Give the panel square corners and keep it above the work behind it. GPUI
/// opens a titled window even when the titlebar is hidden, and macOS rounds
/// every titled window; removing the radius from the window's frame layer
/// keeps the native key handling intact, which a borderless style mask does
/// not. `WindowKind::Floating` still maps to the normal window level in GPUI,
/// so the level is raised here: the panel behaves like Things' quick entry and
/// stays visible when another app comes forward.
#[cfg(target_os = "macos")]
fn float_panel(window: &Window) {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::RawWindowHandle;

    let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    let view = appkit.ns_view.as_ptr().cast::<Object>();
    // SAFETY: `window_handle` returned an AppKit handle, so this is a live
    // `NSView` owned by the window for the duration of the call. Every message
    // is a main-thread AppKit call and `render` runs on the main thread.
    unsafe {
        let native: *mut Object = msg_send![view, window];
        if native.is_null() {
            return;
        }
        // NSFloatingWindowLevel. Normal windows, including every other app's,
        // sit below it; system panels and alerts still sit above.
        let _: () = msg_send![native, setLevel: 3i64];
        let _: () = msg_send![native, setHidesOnDeactivate: false];
        let content: *mut Object = msg_send![native, contentView];
        if content.is_null() {
            return;
        }
        // The theme frame is the view AppKit rounds for a titled window.
        let frame: *mut Object = msg_send![content, superview];
        for target in [content, frame] {
            if target.is_null() {
                continue;
            }
            let layer: *mut Object = msg_send![target, layer];
            if !layer.is_null() {
                let _: () = msg_send![layer, setCornerRadius: 0.0f64];
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn float_panel(_window: &Window) {}

#[cfg(target_os = "macos")]
fn app_is_active() -> bool {
    use objc::runtime::{BOOL, Object};
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let running: *mut Object = msg_send![class!(NSRunningApplication), currentApplication];
        if running.is_null() {
            return false;
        }
        let active: BOOL = msg_send![running, isActive];
        active
    }
}

#[cfg(not(target_os = "macos"))]
fn app_is_active() -> bool {
    true
}

fn reveal_in_main(id: TaskId, cx: &mut Context<QuickCapture>) {
    for any in cx.windows() {
        if let Some(handle) = any.downcast::<FlowlistApp>() {
            let _ = handle.update(cx, |app, window, cx| {
                app.reveal_task(id.clone(), cx);
                window.activate_window();
                cx.activate(true);
            });
            return;
        }
    }
}

fn panel_bounds(cx: &App, anchor: Option<Bounds<gpui::Pixels>>) -> Bounds<gpui::Pixels> {
    let panel = size(px(PANEL_WIDTH), px(PANEL_HEIGHT));
    let (x, y) = match anchor {
        // The Add task button belongs to the window, so its panel opens over
        // the window. The global shortcut keeps the Spotlight habit of
        // landing in the middle of the display.
        Some(anchor) => (
            anchor.origin.x + (anchor.size.width - panel.width) / 2.0,
            anchor.origin.y + (anchor.size.height - panel.height) / 2.0,
        ),
        None => {
            let display = cx
                .primary_display()
                .or_else(|| cx.displays().into_iter().next());
            let bounds = display.map(|display| display.bounds()).unwrap_or_else(|| {
                Bounds::new(
                    gpui::point(px(0.), px(0.)),
                    size(px(1280.), px(800.)),
                )
            });
            (
                bounds.origin.x + (bounds.size.width - panel.width) / 2.0,
                bounds.origin.y + (bounds.size.height - panel.height) / 2.0,
            )
        }
    };
    // Whole-pixel origins keep text on the pixel grid instead of landing on a
    // half-pixel boundary and looking soft.
    Bounds::new(
        gpui::point(px(f32::from(x).round()), px(f32::from(y).round())),
        panel,
    )
}

/// Room for the title line inside the 620px panel, after the plus badge and
/// the panel padding have taken theirs.
const INPUT_ROOM: f32 = 500.0;

fn field_text(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
    window: &mut Window,
) -> impl IntoElement {
    let chars: Vec<char> = buffer.text().chars().collect();
    let (start, end) = buffer.selection();
    let start = start.min(chars.len());
    let end = end.min(chars.len());
    let has_selection = start != end;
    // A long capture is one line and the caret sits at the end, so show the
    // tail rather than the head: the part being typed never leaves the panel.
    let drop = visible_start(&chars, end, theme, window);
    let before_start = start.max(drop);
    let before_end = end.max(drop);
    let before: String = chars[drop..before_start].iter().collect();
    let selected: String = chars[before_start..before_end].iter().collect();
    let after: String = chars[before_end..].iter().collect();
    let empty = chars.is_empty();

    div()
        .flex()
        .items_center()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_size(px(17.))
        .line_height(px(22.))
        .font_weight(FontWeight::NORMAL)
        .when(drop > 0, |this| {
            this.child(
                div()
                    .flex_shrink_0()
                    .text_color(theme.text_faint)
                    .child("…"),
            )
        })
        .child(run(before, false, theme))
        .when(has_selection, |this| {
            this.child(run(selected.clone(), true, theme))
        })
        .when(!has_selection && focused, |this| {
            this.child(div().w(px(1.5)).h(px(18.)).flex_shrink_0().bg(theme.accent))
        })
        .child(run(after, false, theme))
        .when(empty, |this| {
            this.child(
                div()
                    .font_weight(FontWeight::NORMAL)
                    .text_color(theme.text_faint)
                    .child(placeholder.to_string()),
            )
        })
}

/// How many leading characters to drop so the caret and its tail fit in the
/// panel. Zero when the whole line fits.
fn visible_start(chars: &[char], end: usize, theme: Theme, window: &mut Window) -> usize {
    let whole: String = chars.iter().collect();
    if text_width(&whole, theme, window) <= INPUT_ROOM {
        return 0;
    }
    // Width falls as more characters are dropped, so find the first index
    // whose tail fits: that is the most characters that can go.
    let mut low = 0;
    let mut high = end;
    while low < high {
        let middle = (low + high) / 2;
        let mut candidate = String::from("…");
        candidate.extend(&chars[middle..]);
        if text_width(&candidate, theme, window) <= INPUT_ROOM {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    // Start on a word rather than mid-syllable, but do not hunt far.
    let mut drop = low;
    if drop > 0 {
        let limit = (drop + 16).min(end);
        while drop < limit && !chars[drop].is_whitespace() {
            drop += 1;
        }
        while drop < end && chars[drop].is_whitespace() {
            drop += 1;
        }
    }
    drop
}

fn text_width(text: &str, theme: Theme, window: &mut Window) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let runs = [TextRun {
        len: text.len(),
        font: Font {
            weight: FontWeight::NORMAL,
            ..font("SF Pro Text")
        },
        color: theme.text,
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let shaped =
        window
            .text_system()
            .shape_line(SharedString::from(text.to_string()), px(17.), &runs, None);
    f32::from(shaped.width)
}

fn run(text: String, selected: bool, theme: Theme) -> impl IntoElement {
    div()
        .when(selected, |this| this.bg(theme.selection))
        .child(text)
}

fn chip(icon: Icon, text: impl Into<SharedString>, color: gpui::Hsla) -> impl IntoElement {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .max_w(px(170.))
        .h(px(21.))
        .px(px(7.))
        .rounded_lg()
        .border_1()
        .border_color(color.opacity(0.22))
        .bg(color.opacity(0.10))
        .text_size(px(10.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .child(icons::icon(icon, 11.0, color))
        .child(div().flex_1().min_w_0().truncate().child(text.into()))
}

/// A shortcut printed as plain text. No border, background or rounded box, so
/// it cannot pick up a corner artefact or disappear into the panel tint.
fn key_hint(theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.text_dim)
        .child(text.into())
}

fn label(theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(theme.text_faint)
        .child(text.into())
}
