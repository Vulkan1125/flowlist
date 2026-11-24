//! The floating quick-capture panel.
//!
//! Opened by the system-wide `⌃⌥Space` shortcut or the title-bar plus, it is a
//! small window of its own, so the main workspace, selection and scroll never
//! move. Capturing goes through the same `Store::execute` path as every other
//! edit, which keeps the action undoable from the main window.
//!
//! The panel is intentionally quiet: a plus icon, one title line, and a live
//! read-out of the markers (`@1`, `due:3d`, `fsr:2d`, `#tag`, `::Project`)
//! with their dates resolved. Enter files it and closes; `⌘⏎` files it and
//! reveals it in the main window; `⇧⏎` files it and stays open for the next
//! thought.

// `objc` 0.2 expands its macros with a `cfg(feature = "cargo-clippy")` check
// that Cargo's check-cfg does not know about. Silence that third-party noise,
// the same way `blur.rs` does.
#![allow(unexpected_cfgs)]

use std::path::PathBuf;

use anyhow::Result;
use flowlist_core::command::Command;
use flowlist_core::ids::TaskId;
use flowlist_core::parse::{QuickEntry, parse_quick};
use flowlist_core::paths::DataPaths;
use flowlist_core::store::Store;
use flowlist_core::time::{Stamp, format_duration, human_date, now};
use gpui::{
    AnyElement, App, Bounds, ClipboardItem, Context, CursorStyle, FocusHandle, Font, FontWeight,
    IntoElement, KeyDownEvent, MouseButton, Render, SharedString, TextRun, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, font, prelude::*,
    px, size,
};

use super::FlowlistApp;
use super::icons::{self, Icon};
use super::session::Session;
use super::text::TextBuffer;
use super::theme::{Appearance, Theme, ThemeKind};
use super::{BufferKey, handle_buffer_key};

const PANEL_WIDTH: f32 = 620.0;
const PANEL_HEIGHT: f32 = 126.0;

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
        let syntax = entry
            .map(|entry| {
                interpretation_items(entry)
                    .into_iter()
                    .map(|(label, value)| format!("{label}:{value}"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let state = serde_json::json!({
            "open": open,
            "text": text,
            "title": title,
            "error": self.error.clone().or(parse_error),
            "destination": self.destination(entry),
            "syntax": syntax,
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
        div()
            .flex()
            .items_center()
            .gap_3()
            .px(px(22.))
            .pt(px(16.))
            .h(px(58.))
            .child(
                div()
                    .id("capture-add")
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(22.))
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.72))
                    .child(icons::icon(Icon::Plus, 18.0, theme.accent))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.capture(false, window, cx);
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(field_text(
                        theme,
                        &self.buffer,
                        "What's on your mind?  due:3d  fsr:2d  @1",
                        true,
                        window,
                    ))
                    .cursor(CursorStyle::IBeam),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus_handle);
                    cx.notify();
                }),
            )
    }

    /// The syntax the parser actually understood, with dates resolved to the
    /// day and time they will fire. The task title never appears here; this is
    /// a read-out of the markers. It wraps onto a second line instead of
    /// truncating.
    fn render_interpretation(&self) -> AnyElement {
        let theme = self.theme;
        let text = self.buffer.text().trim().to_string();
        let area = div()
            .id("capture-syntax")
            .flex()
            .flex_col()
            .gap_1()
            .px(px(22.))
            .pt(px(9.))
            .pb(px(14.))
            .border_t_1()
            .border_color(theme.border_soft);
        if text.is_empty() {
            return match &self.flash {
                Some(flash) => area
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.success)
                            .child(flash.clone()),
                    )
                    .into_any_element(),
                None => div().into_any_element(),
            };
        }
        let entry = match parse_quick(&text, now()) {
            Ok(entry) => entry,
            Err(error) => {
                return area
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.warning)
                            .child(format!("Couldn't read that: {error}")),
                    )
                    .into_any_element();
            }
        };
        let items = interpretation_items(&entry);
        if items.is_empty() {
            return div().into_any_element();
        }
        let mut line = div().flex().flex_wrap().items_center().gap_2();
        for (label, value) in items {
            line = line.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.text_faint)
                            .child(format!("{label}:")),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(value),
                    ),
            );
        }
        area.child(line).into_any_element()
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
        let text = self.buffer.text().trim().to_string();
        let has_syntax = self.flash.is_some()
            || (!text.is_empty()
                && parse_quick(&text, now())
                    .ok()
                    .is_some_and(|entry| !interpretation_items(&entry).is_empty()));
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .rounded_none()
            .overflow_hidden()
            .when(!has_syntax, |this| this.justify_center())
            .border_1()
            .border_color(theme.border)
            // Opaque on purpose: a translucent panel over the desktop produced
            // hard-edged corner artefacts on every rounded control inside it.
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
            .child(self.render_input(window, cx))
            .child(self.render_interpretation())
    }
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
            // No titlebar controls: the card itself is the whole window.
            // The background is opaque so controls inside never pick up the
            // desktop tint or its corner artefacts.
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
fn frontmost_app() -> Option<i32> {
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
fn restore_app(pid: i32) {
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
    match anchor {
        // The Add task button belongs to the window, so its panel opens over
        // the window. The global shortcut keeps the Spotlight habit of
        // landing in the middle of the display.
        Some(anchor) => Bounds::new(
            gpui::point(
                anchor.origin.x + (anchor.size.width - panel.width) / 2.0,
                anchor.origin.y + (anchor.size.height - panel.height) / 2.0,
            ),
            panel,
        ),
        None => Bounds::centered(None, panel, cx),
    }
}

/// Room for the title line inside the 620px panel, after the plus icon and the
/// panel padding have taken theirs.
const INPUT_ROOM: f32 = 540.0;

fn stamp_display(stamp: Stamp) -> String {
    if stamp.has_time {
        stamp.local_naive().format("%Y-%m-%d %H:%M").to_string()
    } else {
        stamp.local_date().format("%Y-%m-%d").to_string()
    }
}

/// The markers the parser understood, in the order the user usually reads
/// them. Dates are resolved, so `due:3d` reads back as the actual day.
fn interpretation_items(entry: &QuickEntry) -> Vec<(String, String)> {
    let mut items = Vec::new();
    if let Some(priority) = entry.priority {
        items.push(("priority".to_string(), priority.label().to_lowercase()));
    }
    if let Some(due) = entry.due {
        items.push(("due".to_string(), stamp_display(due)));
    }
    if entry.fullscreen_reminder {
        let value = match entry.reminder_at {
            Some(stamp) => stamp_display(stamp),
            None if entry.due.is_some() => "follows due".to_string(),
            None => "ignored (no time)".to_string(),
        };
        items.push(("fsr".to_string(), value));
    }
    if let Some(defer) = entry.defer {
        items.push(("defer".to_string(), stamp_display(defer)));
    }
    if let Some(minutes) = entry.estimate_min {
        items.push(("estimate".to_string(), format_duration(minutes)));
    }
    if let Some(repeat) = &entry.repeat {
        items.push(("repeat".to_string(), repeat.description()));
    }
    if entry.flagged {
        items.push(("flag".to_string(), "yes".to_string()));
    }
    if let Some(project) = &entry.project_path {
        items.push(("project".to_string(), project.clone()));
    }
    for tag in &entry.tag_paths {
        items.push(("tag".to_string(), format!("#{tag}")));
    }
    items
}

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
        .font_weight(FontWeight::MEDIUM)
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
            weight: FontWeight::MEDIUM,
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
