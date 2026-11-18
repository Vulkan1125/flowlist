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

use anyhow::Result;
use flowlist_core::command::{Command, NewTask};
use flowlist_core::ids::{ProjectId, TagId, TaskId};
use flowlist_core::model::Priority;
use flowlist_core::parse::{QuickEntry, parse_quick};
use flowlist_core::paths::DataPaths;
use flowlist_core::store::Store;
use flowlist_core::time::{Stamp, format_duration, human_date, now};
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, FocusHandle, FontWeight, IntoElement,
    KeyDownEvent, MouseButton, Render, SharedString, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, prelude::*, px, size,
};

use super::FlowlistApp;
use super::icons::{self, Icon};
use super::session::Session;
use super::text::TextBuffer;
use super::theme::{Appearance, Theme, ThemeKind};
use super::{BufferKey, handle_buffer_key};

const PANEL_WIDTH: f32 = 620.0;
const PANEL_HEIGHT: f32 = 186.0;

/// What the panel adds when the text itself does not name a destination.
#[derive(Clone, Default)]
pub struct CaptureDefaults {
    pub project: Option<ProjectId>,
    pub tag: Option<TagId>,
    pub priority: Option<Priority>,
    pub flagged: bool,
    pub due: Option<Stamp>,
}

impl CaptureDefaults {
    pub fn inbox() -> Self {
        Self::default()
    }

    fn apply(&self, task: &mut NewTask) {
        if let Some(project) = &self.project {
            task.project = Some(project.clone());
            task.project_path = None;
        }
        if let Some(tag) = &self.tag
            && !task.tags.contains(tag)
        {
            task.tags.push(tag.clone());
        }
        if let Some(priority) = self.priority
            && task.priority.is_none()
        {
            task.priority = Some(priority);
        }
        if self.flagged {
            task.flagged = true;
        }
        if let Some(due) = self.due
            && task.due.is_none()
        {
            task.due = Some(due);
        }
    }

    fn destination(&self, store: &Store) -> String {
        if let Some(project) = &self.project
            && let Some(project) = store.db().projects.get(project)
        {
            return project.name.clone();
        }
        if let Some(tag) = &self.tag
            && let Some(tag) = store.db().tags.get(tag)
        {
            return format!("#{}", tag.name);
        }
        if let Some(priority) = self.priority {
            return format!("{} priority", priority.label());
        }
        if self.flagged {
            return "Flagged".to_string();
        }
        if self.due.is_some() {
            return "Today".to_string();
        }
        "Inbox".to_string()
    }

    fn icon(&self) -> Icon {
        if self.project.is_some() {
            Icon::Project
        } else if self.tag.is_some() {
            Icon::Tag
        } else if self.priority.is_some() || self.flagged {
            Icon::Flagged
        } else if self.due.is_some() {
            Icon::Calendar
        } else {
            Icon::Inbox
        }
    }
}

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
    defaults: CaptureDefaults,
    buffer: TextBuffer,
    focus_handle: FocusHandle,
    error: Option<String>,
    flash: Option<String>,
    last_captured: Option<String>,
    test_state_path: Option<PathBuf>,
    test_state_last: Option<String>,
    window_active: bool,
    active_window: Option<u64>,
}

impl QuickCapture {
    fn new(store: Store, theme: Theme, defaults: CaptureDefaults, cx: &mut Context<Self>) -> Self {
        Self {
            store,
            theme,
            defaults,
            buffer: TextBuffer::empty(),
            focus_handle: cx.focus_handle(),
            error: None,
            flash: None,
            last_captured: None,
            test_state_path: std::env::var_os("FLOWLIST_TEST_STATE")
                .map(PathBuf::from)
                .map(|path| path.with_file_name("capture.json")),
            test_state_last: None,
            window_active: false,
            active_window: None,
        }
    }

    /// Bring an already-open panel back for another capture. The text is kept
    /// and selected so a new thought replaces it without a trip to Backspace.
    fn reopen(
        &mut self,
        theme: Theme,
        defaults: CaptureDefaults,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.theme = theme;
        self.defaults = defaults;
        self.error = None;
        self.flash = None;
        let _ = self.store.reload_if_changed();
        if !self.buffer.is_empty() {
            self.buffer.select_all();
        }
        if super::blur::sync(window, theme.appearance()).is_some() {
            // The material follows the palette, exactly like the main window.
        }
        window.defer(cx, |window, cx| {
            square_corners(window);
            activate_app();
            cx.activate(true);
            window.activate_window();
        });
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn destination(&self, entry: Option<&QuickEntry>) -> String {
        if let Some(entry) = entry
            && let Some(path) = &entry.project_path
        {
            return path.clone();
        }
        self.defaults.destination(&self.store)
    }

    fn destination_icon(&self, entry: Option<&QuickEntry>) -> Icon {
        if entry.is_some_and(|entry| entry.project_path.is_some()) {
            Icon::Project
        } else {
            self.defaults.icon()
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
        let mut task = entry.into_new_task();
        self.defaults.apply(&mut task);
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
            Commit::Empty => self.dismiss(window, cx),
            Commit::Added { id, .. } => {
                if open_in_main && let Some(id) = id {
                    reveal_in_main(id, cx);
                }
                self.dismiss(window, cx);
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

    fn dismiss(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        self.write_state(false);
        window.remove_window();
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

        if mods.platform && matches!(stroke.key.as_str(), "enter" | "return") {
            self.capture(true, window, cx);
            return;
        }
        if mods.shift && matches!(stroke.key.as_str(), "enter" | "return") {
            self.capture_keep(cx);
            return;
        }
        if stroke.key == "escape" {
            self.dismiss(window, cx);
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
            self.dismiss(window, cx);
            return;
        }

        match handle_buffer_key(&mut self.buffer, stroke, paste.as_deref()) {
            BufferKey::Changed => {
                self.error = None;
                self.flash = None;
                cx.notify();
            }
            BufferKey::Submit => self.capture(false, window, cx),
            BufferKey::Cancel => self.dismiss(window, cx),
            BufferKey::Tab | BufferKey::Ignored => {}
        }
    }

    fn render_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    ))
                    .cursor(CursorStyle::IBeam),
            )
            .child(keycap(theme, "esc"))
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
            ("due:tomorrow", theme.text_dim),
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

        let mut any = false;
        if entry.title != text {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .text_size(px(12.))
                    .text_color(theme.text_dim)
                    .child(format!("→ {}", entry.title)),
            );
            any = true;
        }
        if let Some(project) = &entry.project_path {
            row = row.child(chip(Icon::Project, project.clone(), theme.accent));
            any = true;
        }
        for tag in &entry.tag_paths {
            row = row.child(chip(Icon::Tag, format!("#{tag}"), theme.accent));
            any = true;
        }
        if entry.flagged {
            row = row.child(chip(Icon::Flagged, "Flagged", theme.warning));
            any = true;
        }
        if let Some(due) = entry.due {
            row = row.child(chip(Icon::Calendar, human_date(&due, now()), theme.accent));
            any = true;
        }
        if let Some(defer) = entry.defer {
            row = row.child(chip(
                Icon::Clock,
                format!("Defer {}", human_date(&defer, now())),
                theme.text_dim,
            ));
            any = true;
        }
        if let Some(minutes) = entry.estimate_min {
            row = row.child(chip(Icon::Clock, format_duration(minutes), theme.text_dim));
            any = true;
        }
        if let Some(repeat) = &entry.repeat {
            row = row.child(chip(Icon::Repeat, repeat.description(), theme.success));
            any = true;
        }
        if self.flash.is_some() {
            any = true;
        }
        if !any {
            return self.render_hints().into_any_element();
        }
        row.into_any_element()
    }

    fn render_footer(&self) -> impl IntoElement {
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
                    .gap_2()
                    .child(keycap(theme, "⏎"))
                    .child(label(theme, "Add"))
                    .child(div().w(px(4.)))
                    .child(keycap(theme, "⌘⏎"))
                    .child(label(theme, "Add & open")),
            )
    }
}

impl Render for QuickCapture {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.window_active = window.is_window_active();
        self.active_window = cx.active_window().map(|handle| handle.window_id().as_u64());
        self.write_state(true);
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
            // The same vertical glass wash as the sidebar and inspector, so the
            // panel belongs to the window rather than floating on its own tint.
            .bg(theme.glass_fill(180.))
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
            .child(self.render_input(cx))
            .child(self.render_preview())
            .child(self.render_footer())
    }
}

/// Open or bring forward the capture panel.
pub fn show(cx: &mut App, paths: DataPaths, theme: Theme, defaults: CaptureDefaults) -> Result<()> {
    // One panel at a time. Reusing it keeps the glass, and the theme, warm.
    for any in cx.windows() {
        if let Some(handle) = any.downcast::<QuickCapture>() {
            activate_app();
            cx.activate(true);
            let updated = handle.update(cx, |view, window, cx| {
                view.reopen(theme, defaults.clone(), window, cx);
                window.activate_window();
            });
            if updated.is_ok() {
                return Ok(());
            }
        }
    }

    let store = Store::open(paths)?;
    let bounds = panel_bounds(cx);
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
            window_background: WindowBackgroundAppearance::Blurred,
            window_min_size: Some(size(px(PANEL_WIDTH), px(PANEL_HEIGHT))),
            ..Default::default()
        },
        move |window, cx| {
            let appearance = theme.appearance();
            let view = cx.new(|cx| QuickCapture::new(store, theme, defaults, cx));
            let handle = view.read(cx).focus_handle.clone();
            let _ = super::blur::sync(window, appearance);
            window.focus(&handle);
            // A global shortcut fires while another app owns the menu bar.
            // Activating on the next turn, after the window exists, is what
            // makes the panel key instead of merely visible.
            window.defer(cx, |window, cx| {
                square_corners(window);
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
        .map(|session| session.theme_choice())
        .unwrap_or((ThemeKind::Paper, Appearance::Light));
    Theme::new(kind, appearance)
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

/// Give the panel square corners. GPUI opens a titled window even when the
/// titlebar is hidden, and macOS rounds every titled window. Removing the
/// radius from the window's frame layer keeps the native key handling intact,
/// which a borderless style mask does not.
#[cfg(target_os = "macos")]
fn square_corners(window: &Window) {
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
fn square_corners(_window: &Window) {}

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

fn panel_bounds(cx: &App) -> Bounds<gpui::Pixels> {
    let panel = size(px(PANEL_WIDTH), px(PANEL_HEIGHT));
    // Centred on the primary display, like a Spotlight prompt: the eye lands
    // in the middle of the screen instead of hunting the menu bar.
    Bounds::centered(None, panel, cx)
}

fn field_text(
    theme: Theme,
    buffer: &TextBuffer,
    placeholder: &str,
    focused: bool,
) -> impl IntoElement {
    let chars: Vec<char> = buffer.text().chars().collect();
    let (start, end) = buffer.selection();
    let has_selection = start != end;
    let before: String = chars[..start.min(chars.len())].iter().collect();
    let selected: String = chars[start.min(chars.len())..end.min(chars.len())]
        .iter()
        .collect();
    let after: String = chars[end.min(chars.len())..].iter().collect();
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

fn run(text: String, selected: bool, theme: Theme) -> impl IntoElement {
    div()
        .whitespace_normal()
        .when(selected, |this| this.bg(theme.selection))
        .child(text)
}

fn chip(icon: Icon, text: impl Into<SharedString>, color: gpui::Hsla) -> impl IntoElement {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_1()
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
        .child(text.into())
}

fn keycap(theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .h(px(20.))
        .px(px(6.))
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.field)
        .text_size(px(10.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.text_faint)
        .child(text.into())
}

fn label(theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(theme.text_faint)
        .child(text.into())
}
