//! Theme-synced abstract backdrops.
//!
//! The full-screen reminder can sit on a soft, layered abstract instead of a
//! flat colour. Every shape takes its colour from the active palette, so the
//! same style looks right in Plain, Nord, Dracula and the rest.

use gpui::{AnyElement, IntoElement, div, prelude::*, px, linear_color_stop, linear_gradient};

use super::theme::{Theme, ThemeKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstractStyle {
    None,
    Aurora,
    Dunes,
    Halo,
    Mesh,
    Tide,
    Ember,
}

impl AbstractStyle {
    pub const ALL: [AbstractStyle; 7] = [
        AbstractStyle::None,
        AbstractStyle::Aurora,
        AbstractStyle::Dunes,
        AbstractStyle::Halo,
        AbstractStyle::Mesh,
        AbstractStyle::Tide,
        AbstractStyle::Ember,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            AbstractStyle::None => "none",
            AbstractStyle::Aurora => "aurora",
            AbstractStyle::Dunes => "dunes",
            AbstractStyle::Halo => "halo",
            AbstractStyle::Mesh => "mesh",
            AbstractStyle::Tide => "tide",
            AbstractStyle::Ember => "ember",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AbstractStyle::None => "None",
            AbstractStyle::Aurora => "Aurora",
            AbstractStyle::Dunes => "Dunes",
            AbstractStyle::Halo => "Halo",
            AbstractStyle::Mesh => "Mesh",
            AbstractStyle::Tide => "Tide",
            AbstractStyle::Ember => "Ember",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.slug() == slug)
    }

    /// The style a theme opens with, so Match theme still has a voice.
    pub fn for_theme(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Plain | ThemeKind::Paper | ThemeKind::Graphite => AbstractStyle::Dunes,
            ThemeKind::Mist | ThemeKind::Nord | ThemeKind::Midnight => AbstractStyle::Aurora,
            ThemeKind::Catppuccin | ThemeKind::Dracula => AbstractStyle::Halo,
            ThemeKind::Gruvbox | ThemeKind::Vesper => AbstractStyle::Ember,
            ThemeKind::TokyoNight => AbstractStyle::Tide,
        }
    }
}

fn blob(color: gpui::Hsla, x: f32, y: f32, size: f32) -> AnyElement {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .size(px(size))
        .rounded_full()
        .bg(color)
        .into_any_element()
}

fn slab(color: gpui::Hsla, x: f32, y: f32, width: f32, height: f32) -> AnyElement {
    div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(width))
        .h(px(height))
        .rounded_full()
        .bg(color)
        .into_any_element()
}

pub fn render(style: AbstractStyle, theme: Theme) -> AnyElement {
    let base = div()
        .absolute()
        .size_full()
        .overflow_hidden()
        .bg(theme.bg);
    match style {
        AbstractStyle::None => base.into_any_element(),
        AbstractStyle::Aurora => base
            .bg(linear_gradient(
                130.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.accent.opacity(0.10), 1.0),
            ))
            .child(blob(theme.accent.opacity(0.14), -240.0, -260.0, 860.0))
            .child(blob(theme.success.opacity(0.11), 820.0, 180.0, 700.0))
            .child(blob(theme.warning.opacity(0.08), 280.0, 760.0, 540.0))
            .into_any_element(),
        AbstractStyle::Dunes => base
            .bg(linear_gradient(
                35.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.field, 1.0),
            ))
            .child(slab(theme.accent.opacity(0.10), -80.0, 220.0, 1180.0, 220.0))
            .child(slab(theme.warning.opacity(0.09), 140.0, 470.0, 980.0, 180.0))
            .child(slab(theme.success.opacity(0.07), -60.0, 720.0, 1040.0, 190.0))
            .into_any_element(),
        AbstractStyle::Halo => base
            .bg(linear_gradient(
                120.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.accent.opacity(0.07), 1.0),
            ))
            .child(
                div()
                    .absolute()
                    .left(px(280.))
                    .top(px(90.))
                    .size(px(760.))
                    .rounded_full()
                    .border_8()
                    .border_color(theme.accent.opacity(0.12)),
            )
            .child(blob(theme.accent.opacity(0.10), 420.0, 230.0, 480.0))
            .child(blob(theme.success.opacity(0.08), 760.0, 540.0, 380.0))
            .into_any_element(),
        AbstractStyle::Mesh => base
            .bg(theme.bg)
            .child(slab(theme.accent.opacity(0.11), -140.0, -100.0, 680.0, 680.0))
            .child(slab(theme.success.opacity(0.10), 620.0, 120.0, 560.0, 560.0))
            .child(slab(theme.warning.opacity(0.08), 140.0, 620.0, 560.0, 480.0))
            .child(blob(theme.danger.opacity(0.05), 760.0, 680.0, 420.0))
            .into_any_element(),
        AbstractStyle::Tide => base
            .bg(linear_gradient(
                65.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.accent.opacity(0.12), 1.0),
            ))
            .child(slab(theme.accent.opacity(0.12), 120.0, 120.0, 1100.0, 160.0))
            .child(slab(theme.success.opacity(0.09), -120.0, 380.0, 1120.0, 130.0))
            .child(slab(theme.warning.opacity(0.07), 220.0, 640.0, 980.0, 150.0))
            .into_any_element(),
        AbstractStyle::Ember => base
            .bg(linear_gradient(
                145.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.danger.opacity(0.10), 1.0),
            ))
            .child(blob(theme.danger.opacity(0.12), -180.0, 120.0, 760.0))
            .child(blob(theme.warning.opacity(0.11), 700.0, -120.0, 620.0))
            .child(blob(theme.accent.opacity(0.09), 360.0, 680.0, 560.0))
            .into_any_element(),
    }
}

fn image_extension(path: &std::path::Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "webp" => Some("webp"),
        "gif" => Some("gif"),
        "bmp" => Some("bmp"),
        "tif" | "tiff" => Some("tiff"),
        _ => None,
    }
}

/// Copy a picked image into the library root so the original can move away.
pub fn install_wallpaper(
    source: &std::path::Path,
    root: &std::path::Path,
    name: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let extension = image_extension(source)
        .ok_or_else(|| anyhow::anyhow!("Choose a PNG, JPEG, WebP, GIF, BMP or TIFF image"))?;
    let directory = root.join("wallpapers");
    std::fs::create_dir_all(&directory)?;
    let destination = directory.join(format!("{name}.{extension}"));
    std::fs::copy(source, &destination)?;
    Ok(destination)
}
