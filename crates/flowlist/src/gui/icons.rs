//! A small embedded icon set drawn as SVG paths.
//!
//! Text glyphs render differently in every font and sit on their own baselines,
//! which made the toolbar feel uneven. These icons share a 24px grid, a 1.75px
//! stroke and round joins, so every control lines up no matter the theme.
//! The assets are compiled into the binary: no runtime files to lose.

use std::borrow::Cow;

use gpui::{AssetSource, Hsla, IntoElement, Result, SharedString, prelude::*, px, svg};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Inbox,
    Projects,
    Forecast,
    Flagged,
    Review,
    Tags,
    Completed,
    Nearby,
    Perspective,
    Folder,
    FolderOpen,
    Project,
    Tag,
    ChevronRight,
    ChevronDown,
    Pencil,
    Crosshair,
    Close,
    Undo,
    Redo,
    Help,
    Settings,
    Palette,
    PanelRight,
    Plus,
    ArrowUp,
    ArrowDown,
    Check,
    CircleDashed,
    Search,
    Trash,
    Clock,
    Calendar,
    Repeat,
}

impl Icon {
    pub const ALL: [Icon; 34] = [
        Icon::Inbox,
        Icon::Projects,
        Icon::Forecast,
        Icon::Flagged,
        Icon::Review,
        Icon::Tags,
        Icon::Completed,
        Icon::Nearby,
        Icon::Perspective,
        Icon::Folder,
        Icon::FolderOpen,
        Icon::Project,
        Icon::Tag,
        Icon::ChevronRight,
        Icon::ChevronDown,
        Icon::Pencil,
        Icon::Crosshair,
        Icon::Close,
        Icon::Undo,
        Icon::Redo,
        Icon::Help,
        Icon::Settings,
        Icon::Palette,
        Icon::PanelRight,
        Icon::Plus,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::Check,
        Icon::CircleDashed,
        Icon::Search,
        Icon::Trash,
        Icon::Clock,
        Icon::Calendar,
        Icon::Repeat,
    ];

    pub fn path(self) -> &'static str {
        match self {
            Icon::Inbox => "icons/inbox.svg",
            Icon::Projects => "icons/projects.svg",
            Icon::Forecast => "icons/forecast.svg",
            Icon::Flagged => "icons/flagged.svg",
            Icon::Review => "icons/review.svg",
            Icon::Tags => "icons/tags.svg",
            Icon::Completed => "icons/completed.svg",
            Icon::Nearby => "icons/nearby.svg",
            Icon::Perspective => "icons/perspective.svg",
            Icon::Folder => "icons/folder.svg",
            Icon::FolderOpen => "icons/folder-open.svg",
            Icon::Project => "icons/project.svg",
            Icon::Tag => "icons/tag.svg",
            Icon::ChevronRight => "icons/chevron-right.svg",
            Icon::ChevronDown => "icons/chevron-down.svg",
            Icon::Pencil => "icons/pencil.svg",
            Icon::Crosshair => "icons/crosshair.svg",
            Icon::Close => "icons/close.svg",
            Icon::Undo => "icons/undo.svg",
            Icon::Redo => "icons/redo.svg",
            Icon::Help => "icons/help.svg",
            Icon::Settings => "icons/settings.svg",
            Icon::Palette => "icons/palette.svg",
            Icon::PanelRight => "icons/panel-right.svg",
            Icon::Plus => "icons/plus.svg",
            Icon::ArrowUp => "icons/arrow-up.svg",
            Icon::ArrowDown => "icons/arrow-down.svg",
            Icon::Check => "icons/check.svg",
            Icon::CircleDashed => "icons/circle-dashed.svg",
            Icon::Search => "icons/search.svg",
            Icon::Trash => "icons/trash.svg",
            Icon::Clock => "icons/clock.svg",
            Icon::Calendar => "icons/calendar.svg",
            Icon::Repeat => "icons/repeat.svg",
        }
    }

    fn body(self) -> &'static str {
        match self {
            Icon::Inbox => {
                r#"<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>"#
            }
            Icon::Projects => {
                r#"<path d="m3 17 2 2 4-4"/><path d="m3 7 2 2 4-4"/><path d="M13 6h8"/><path d="M13 12h8"/><path d="M13 18h8"/>"#
            }
            Icon::Forecast => {
                r#"<path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/><path d="M8 14h.01"/><path d="M12 14h.01"/><path d="M16 14h.01"/><path d="M8 18h.01"/><path d="M12 18h.01"/><path d="M16 18h.01"/>"#
            }
            Icon::Flagged => {
                r#"<path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><path d="M4 22v-7"/>"#
            }
            Icon::Review => {
                r#"<path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/>"#
            }
            Icon::Tags => {
                r#"<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r=".5" fill="currentColor"/>"#
            }
            Icon::Completed => r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#,
            Icon::Nearby => {
                r#"<path d="M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0"/><circle cx="12" cy="10" r="3"/>"#
            }
            Icon::Perspective => {
                r#"<path d="M21 4h-7"/><path d="M10 4H3"/><path d="M21 12h-9"/><path d="M8 12H3"/><path d="M21 20h-5"/><path d="M12 20H3"/><path d="M14 2v4"/><path d="M8 10v4"/><path d="M16 18v4"/>"#
            }
            Icon::Folder => {
                r#"<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>"#
            }
            Icon::FolderOpen => {
                r#"<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>"#
            }
            Icon::Project => {
                r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M8 7v7"/><path d="M12 7v4"/><path d="M16 7v9"/>"#
            }
            Icon::Tag => {
                r#"<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r=".5" fill="currentColor"/>"#
            }
            Icon::ChevronRight => r#"<path d="m9 18 6-6-6-6"/>"#,
            Icon::ChevronDown => r#"<path d="m6 9 6 6 6-6"/>"#,
            Icon::Pencil => {
                r#"<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>"#
            }
            Icon::Crosshair => {
                r#"<circle cx="12" cy="12" r="10"/><path d="M22 12h-4"/><path d="M6 12H2"/><path d="M12 6V2"/><path d="M12 22v-4"/>"#
            }
            Icon::Close => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
            Icon::Undo => {
                r#"<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5v0a5.5 5.5 0 0 1-5.5 5.5H11"/>"#
            }
            Icon::Redo => {
                r#"<path d="m15 14 5-5-5-5"/><path d="M20 9H9.5A5.5 5.5 0 0 0 4 14.5v0A5.5 5.5 0 0 0 9.5 20H13"/>"#
            }
            Icon::Help => {
                r#"<circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>"#
            }
            Icon::Settings => {
                r#"<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>"#
            }
            Icon::Palette => {
                r#"<circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>"#
            }
            Icon::PanelRight => {
                r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 3v18"/>"#
            }
            Icon::Plus => r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#,
            Icon::ArrowUp => r#"<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>"#,
            Icon::ArrowDown => r#"<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>"#,
            Icon::Check => r#"<path d="M20 6 9 17l-5-5"/>"#,
            Icon::CircleDashed => {
                r#"<path d="M10.1 2.18a9.93 9.93 0 0 1 3.8 0"/><path d="M17.6 3.71a9.95 9.95 0 0 1 2.69 2.7"/><path d="M21.82 10.1a9.93 9.93 0 0 1 0 3.8"/><path d="M20.29 17.6a9.95 9.95 0 0 1-2.7 2.69"/><path d="M13.9 21.82a9.94 9.94 0 0 1-3.8 0"/><path d="M6.4 20.29a9.95 9.95 0 0 1-2.69-2.7"/><path d="M2.18 13.9a9.93 9.93 0 0 1 0-3.8"/><path d="M3.71 6.4a9.95 9.95 0 0 1 2.7-2.69"/>"#
            }
            Icon::Search => r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
            Icon::Trash => {
                r#"<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>"#
            }
            Icon::Clock => {
                r#"<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>"#
            }
            Icon::Calendar => {
                r#"<path d="M8 2v4"/><path d="M16 2v4"/><rect width="18" height="18" x="3" y="4" rx="2"/><path d="M3 10h18"/>"#
            }
            Icon::Repeat => {
                r#"<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>"#
            }
        }
    }
}

/// The compiled-in asset source handed to GPUI at startup.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let Some(icon) = Icon::ALL.iter().copied().find(|icon| icon.path() == path) else {
            return Ok(None);
        };
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
            icon.body()
        );
        Ok(Some(Cow::Owned(svg.into_bytes())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path.trim_end_matches('/') != "icons" {
            return Ok(Vec::new());
        }
        Ok(Icon::ALL
            .iter()
            .map(|icon| SharedString::from(icon.path()))
            .collect())
    }
}

/// Draw one icon at a fixed square size in the given colour.
pub fn icon(icon: Icon, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path(icon.path())
        .size(px(size))
        .text_color(color)
        .flex_shrink_0()
}
