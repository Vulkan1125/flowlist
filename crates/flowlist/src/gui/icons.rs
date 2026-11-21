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
    Eyedropper,
    Sun,
    Moon,
    PanelRight,
    Info,
    Plus,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Check,
    CircleDashed,
    Search,
    Trash,
    Clock,
    Calendar,
    Repeat,
    Star,
    Briefcase,
    Home,
    Heart,
    Book,
    Code,
    Rocket,
    Coffee,
    Paperclip,
    Link,
    Archive,
    Skull,
    Smartphone,
}

impl Icon {
    pub const ALL: [Icon; 53] = [
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
        Icon::Eyedropper,
        Icon::Sun,
        Icon::Moon,
        Icon::PanelRight,
        Icon::Info,
        Icon::Plus,
        Icon::ArrowUp,
        Icon::ArrowDown,
        Icon::ArrowLeft,
        Icon::ArrowRight,
        Icon::Check,
        Icon::CircleDashed,
        Icon::Search,
        Icon::Trash,
        Icon::Clock,
        Icon::Calendar,
        Icon::Repeat,
        Icon::Star,
        Icon::Briefcase,
        Icon::Home,
        Icon::Heart,
        Icon::Book,
        Icon::Code,
        Icon::Rocket,
        Icon::Coffee,
        Icon::Paperclip,
        Icon::Link,
        Icon::Archive,
        Icon::Skull,
        Icon::Smartphone,
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
            Icon::Eyedropper => "icons/eyedropper.svg",
            Icon::Sun => "icons/sun.svg",
            Icon::Moon => "icons/moon.svg",
            Icon::PanelRight => "icons/panel-right.svg",
            Icon::Info => "icons/info.svg",
            Icon::Plus => "icons/plus.svg",
            Icon::ArrowUp => "icons/arrow-up.svg",
            Icon::ArrowDown => "icons/arrow-down.svg",
            Icon::ArrowLeft => "icons/arrow-left.svg",
            Icon::ArrowRight => "icons/arrow-right.svg",
            Icon::Check => "icons/check.svg",
            Icon::CircleDashed => "icons/circle-dashed.svg",
            Icon::Search => "icons/search.svg",
            Icon::Trash => "icons/trash.svg",
            Icon::Clock => "icons/clock.svg",
            Icon::Calendar => "icons/calendar.svg",
            Icon::Repeat => "icons/repeat.svg",
            Icon::Star => "icons/star.svg",
            Icon::Briefcase => "icons/briefcase.svg",
            Icon::Home => "icons/home.svg",
            Icon::Heart => "icons/heart.svg",
            Icon::Book => "icons/book.svg",
            Icon::Code => "icons/code.svg",
            Icon::Rocket => "icons/rocket.svg",
            Icon::Coffee => "icons/coffee.svg",
            Icon::Paperclip => "icons/paperclip.svg",
            Icon::Link => "icons/link.svg",
            Icon::Archive => "icons/archive.svg",
            Icon::Skull => "icons/skull.svg",
            Icon::Smartphone => "icons/smartphone.svg",
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
                r#"<path d="M10 2v2"/><path d="M14 2v2"/><path d="M16 8a1 1 0 0 1 1 1v8a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1h14a4 4 0 1 1 0 8h-1"/><path d="M6 2v2"/>"#
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
            Icon::Eyedropper => {
                r#"<path d="m2 22 1-1h3l9-9"/><path d="M3 21v-3l9-9"/><path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"/>"#
            }
            Icon::Sun => {
                r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>"#
            }
            Icon::Moon => r#"<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>"#,
            Icon::PanelRight => {
                r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M15 3v18"/>"#
            }
            Icon::Info => {
                r#"<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>"#
            }
            Icon::Plus => r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#,
            Icon::ArrowUp => r#"<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>"#,
            Icon::ArrowDown => r#"<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>"#,
            Icon::ArrowLeft => r#"<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>"#,
            Icon::ArrowRight => r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#,
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
            Icon::Star => {
                r#"<path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/>"#
            }
            Icon::Briefcase => {
                r#"<path d="M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/><rect width="20" height="14" x="2" y="6" rx="2"/>"#
            }
            Icon::Home => {
                r#"<path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"/><path d="M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#
            }
            Icon::Heart => {
                r#"<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>"#
            }
            Icon::Book => {
                r#"<path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20"/>"#
            }
            Icon::Code => r#"<path d="m16 18 6-6-6-6"/><path d="m8 6-6 6 6 6"/>"#,
            Icon::Rocket => {
                r#"<path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"/><path d="m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"/><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"/><path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"/>"#
            }
            Icon::Coffee => {
                r#"<path d="M10 2v2"/><path d="M14 2v2"/><path d="M16 8a1 1 0 0 1 1 1v8a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1h14a4 4 0 1 1 0 8h-1"/><path d="M6 2v2"/>"#
            }
            Icon::Paperclip => {
                r#"<path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48"/>"#
            }
            Icon::Link => {
                r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#
            }
            Icon::Archive => {
                r#"<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>"#
            }
            Icon::Skull => {
                r#"<path d="m12.5 17-.5-1-.5 1h1z"/><path d="M15 22a1 1 0 0 0 1-1v-1a2 2 0 0 0 1.56-3.25 8 8 0 1 0-11.12 0A2 2 0 0 0 8 20v1a1 1 0 0 0 1 1z"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="12" r="1"/>"#
            }
            Icon::Smartphone => {
                r#"<rect width="14" height="20" x="5" y="2" rx="2" ry="2"/><path d="M12 18h.01"/>"#
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_set_is_complete_and_unique() {
        assert_eq!(Icon::ALL.len(), 53);
        let mut paths: Vec<&str> = Icon::ALL.iter().map(|icon| icon.path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(
            paths.len(),
            Icon::ALL.len(),
            "every icon needs its own path"
        );
        assert!(
            Icon::ALL.iter().all(|icon| !icon.body().is_empty()),
            "every icon needs a drawing"
        );
    }

    #[test]
    fn info_icon_is_available_for_the_inspector() {
        assert_eq!(Icon::Info.path(), "icons/info.svg");
        assert!(Icon::Info.body().contains("<circle"));
    }
}
