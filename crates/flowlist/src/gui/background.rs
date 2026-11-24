//! Theme-synced abstract backdrops.
//!
//! The full-screen reminder can sit on a soft, layered wash instead of a flat
//! colour. Every layer takes its colour from the active palette, so the same
//! style looks right in Plain, Nord, Dracula and the rest.

use gpui::{AnyElement, IntoElement, div, linear_color_stop, linear_gradient, prelude::*, px};

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

fn wash(angle: f32, from: gpui::Hsla, to: gpui::Hsla) -> AnyElement {
    div()
        .absolute()
        .size_full()
        .bg(linear_gradient(
            angle,
            linear_color_stop(from, 0.0),
            linear_color_stop(to, 1.0),
        ))
        .into_any_element()
}

pub fn render(style: AbstractStyle, theme: Theme) -> AnyElement {
    let clear = theme.bg.opacity(0.0);
    let base = div()
        .absolute()
        .size_full()
        .overflow_hidden()
        .bg(theme.bg);
    match style {
        AbstractStyle::None => base.into_any_element(),
        AbstractStyle::Aurora => base
            .child(wash(135.0, theme.accent.opacity(0.16), clear))
            .child(wash(25.0, theme.success.opacity(0.10), clear))
            .child(wash(310.0, theme.warning.opacity(0.06), clear))
            .into_any_element(),
        AbstractStyle::Dunes => base
            .bg(linear_gradient(
                20.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.field, 1.0),
            ))
            .child(wash(10.0, theme.accent.opacity(0.10), clear))
            .child(wash(170.0, theme.warning.opacity(0.08), clear))
            .into_any_element(),
        AbstractStyle::Halo => base
            .child(wash(90.0, theme.accent.opacity(0.14), clear))
            .child(wash(270.0, theme.success.opacity(0.09), clear))
            .into_any_element(),
        AbstractStyle::Mesh => base
            .child(wash(45.0, theme.accent.opacity(0.11), clear))
            .child(wash(135.0, theme.success.opacity(0.09), clear))
            .child(wash(90.0, theme.warning.opacity(0.07), clear))
            .into_any_element(),
        AbstractStyle::Tide => base
            .bg(linear_gradient(
                65.0,
                linear_color_stop(theme.bg, 0.0),
                linear_color_stop(theme.accent.opacity(0.08), 1.0),
            ))
            .child(wash(115.0, theme.success.opacity(0.09), clear))
            .into_any_element(),
        AbstractStyle::Ember => base
            .child(wash(145.0, theme.danger.opacity(0.14), clear))
            .child(wash(35.0, theme.warning.opacity(0.10), clear))
            .into_any_element(),
    }
}

/// A small version of the backdrop for the style picker.
pub fn preview(style: AbstractStyle, theme: Theme) -> AnyElement {
    div()
        .relative()
        .w(px(72.))
        .h(px(48.))
        .rounded_md()
        .overflow_hidden()
        .child(render(style, theme))
        .into_any_element()
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
        "heic" | "heif" => Some("heic"),
        "jxl" => Some("jxl"),
        _ => None,
    }
}

/// Copy a picked image into the library root so the original can move away.
pub fn install_wallpaper(
    source: &std::path::Path,
    root: &std::path::Path,
    name: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let extension = image_extension(source).ok_or_else(|| {
        anyhow::anyhow!("Choose a PNG, JPEG, WebP, JXL, GIF, BMP or TIFF image")
    })?;
    let directory = root.join("wallpapers");
    std::fs::create_dir_all(&directory)?;
    if extension == "heic" || extension == "jxl" {
        // The renderer cannot decode HEIC or JPEG XL. macOS can convert both.
        let destination = directory.join(format!("{name}.png"));
        let status = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "png"])
            .arg(source)
            .arg("--out")
            .arg(&destination)
            .status()?;
        if !status.success() {
            anyhow::bail!("Could not convert that HEIC/JXL image. Try a JPEG or PNG.");
        }
        return Ok(destination);
    }
    let destination = directory.join(format!("{name}.{extension}"));
    std::fs::copy(source, &destination)?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_extensions_cover_the_picker() {
        for name in ["a.png", "a.JPG", "a.webp", "a.heic", "a.jxl", "a.tiff"] {
            assert!(image_extension(std::path::Path::new(name)).is_some());
        }
        assert!(image_extension(std::path::Path::new("a.txt")).is_none());
    }
}
