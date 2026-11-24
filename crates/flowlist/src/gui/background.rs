//! Theme-synced abstract backdrops.
//!
//! The full-screen reminder can sit on a soft, layered wash instead of a flat
//! colour. Every layer takes its colour from the active palette, so the same
//! style looks right in Plain, Nord, Dracula and the rest.

use std::sync::atomic::{AtomicU64, Ordering};

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

/// A palette colour washed over the theme background. Both ends are opaque so
/// the gradient never fades through transparent black, which is what made the
/// old backdrops look hazy at the edges.
fn tint(theme: Theme, color: gpui::Hsla, amount: f32) -> gpui::Hsla {
    theme.bg.blend(color.opacity(amount))
}

/// A single clean two-colour sweep. One gradient across the whole surface is
/// crisp and predictable; stacking several translucent washes produced the
/// halo in the middle that made the previews look muddy.
fn sweep(angle: f32, from: gpui::Hsla, to: gpui::Hsla) -> gpui::Background {
    linear_gradient(
        angle,
        linear_color_stop(from, 0.0),
        linear_color_stop(to, 1.0),
    )
}

pub fn render(style: AbstractStyle, theme: Theme) -> AnyElement {
    let base = div().absolute().size_full().overflow_hidden().bg(theme.bg);
    match style {
        AbstractStyle::None => base.into_any_element(),
        AbstractStyle::Aurora => base
            .bg(sweep(
                135.0,
                tint(theme, theme.accent, 0.22),
                tint(theme, theme.success, 0.14),
            ))
            .into_any_element(),
        AbstractStyle::Dunes => base
            .bg(sweep(
                18.0,
                tint(theme, theme.warning, 0.20),
                tint(theme, theme.accent, 0.10),
            ))
            .into_any_element(),
        AbstractStyle::Halo => base
            .bg(sweep(
                90.0,
                tint(theme, theme.accent, 0.18),
                tint(theme, theme.field, 0.55),
            ))
            .into_any_element(),
        AbstractStyle::Mesh => base
            .bg(sweep(
                45.0,
                tint(theme, theme.success, 0.18),
                tint(theme, theme.accent, 0.12),
            ))
            .into_any_element(),
        AbstractStyle::Tide => base
            .bg(sweep(
                70.0,
                tint(theme, theme.accent, 0.17),
                tint(theme, theme.bg, 0.0),
            ))
            .into_any_element(),
        AbstractStyle::Ember => base
            .bg(sweep(
                160.0,
                tint(theme, theme.danger, 0.20),
                tint(theme, theme.warning, 0.12),
            ))
            .into_any_element(),
    }
}

/// A small version of the backdrop for the style picker. The preview fills its
/// tile so it never looks cropped before the right edge.
pub fn preview(style: AbstractStyle, theme: Theme) -> AnyElement {
    div()
        .relative()
        .w_full()
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
///
/// Every copy gets a unique name. GPUI caches decoded images by path, so
/// replacing a file in place left the old picture on screen; a fresh path
/// forces the new wallpaper to load.
pub fn install_wallpaper(
    source: &std::path::Path,
    root: &std::path::Path,
    name: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let extension = image_extension(source)
        .ok_or_else(|| anyhow::anyhow!("Choose a PNG, JPEG, WebP, JXL, GIF, BMP or TIFF image"))?;
    let directory = root.join("wallpapers");
    std::fs::create_dir_all(&directory)?;
    static NONCE: AtomicU64 = AtomicU64::new(0);
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed)
        + std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0);
    if extension == "heic" || extension == "jxl" {
        // The renderer cannot decode HEIC or JPEG XL. macOS can convert both.
        let destination = directory.join(format!("{name}-{nonce}.png"));
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
    let destination = directory.join(format!("{name}-{nonce}.{extension}"));
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

    #[test]
    fn replacing_a_wallpaper_gets_a_fresh_path() {
        let root = std::env::temp_dir().join(format!(
            "flowlist-wallpaper-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("picked.png");
        std::fs::write(&source, b"not a real image").unwrap();
        let first = install_wallpaper(&source, &root, "reminder").unwrap();
        let second = install_wallpaper(&source, &root, "reminder").unwrap();
        assert_ne!(first, second);
        assert!(first.exists() && second.exists());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
