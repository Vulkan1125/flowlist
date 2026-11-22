//! Site favicons for link attachments.
//!
//! Fetched from the linked host itself (`/favicon.ico`, `/favicon.png`,
//! `/apple-touch-icon.png`) rather than a third-party favicon service, so a
//! link does not leak to anyone except the site it points at. Failures are
//! remembered for the session and the inspector falls back to the chain icon.

use std::io::Read;
use std::time::Duration;

/// The host portion of an `http(s)` link, lowercased.
pub fn host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let without_user = authority.rsplit('@').next()?;
    let host = without_user.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Fetch a favicon for the host. Tries the common icon paths over HTTPS and
/// then HTTP; the response is capped so a rogue server cannot stream forever.
pub fn fetch(host: &str) -> Option<Vec<u8>> {
    const PATHS: [&str; 3] = ["/favicon.ico", "/favicon.png", "/apple-touch-icon.png"];
    for scheme in ["https", "http"] {
        for path in PATHS {
            let url = format!("{scheme}://{host}{path}");
            let response = ureq::get(&url)
                .set(
                    "User-Agent",
                    "Flowlist/0.9.28 (+https://github.com/Vulkan1125/flowlist)",
                )
                .timeout(Duration::from_secs(5))
                .call();
            let Ok(response) = response else {
                continue;
            };
            let mut bytes = Vec::new();
            let mut reader = response.into_reader().take(512 * 1024);
            if reader.read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                return Some(bytes);
            }
        }
    }
    None
}

/// Decode an ICO/PNG/JPEG favicon into the BGRA buffer gpui renders.
pub fn decode(bytes: &[u8]) -> Option<gpui::RenderImage> {
    let rgba = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let mut bgra = rgba.into_raw();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let buffer = image::RgbaImage::from_raw(width, height, bgra)?;
    Some(gpui::RenderImage::new([image::Frame::new(buffer)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_hosts() {
        assert_eq!(host("https://github.com/zed-industries/zed"), Some("github.com".into()));
        assert_eq!(host("http://user@example.com:8080/a?b#c"), Some("example.com".into()));
        assert_eq!(host("https://www.wikipedia.org"), Some("www.wikipedia.org".into()));
        assert_eq!(host("not a link"), None);
    }

    #[test]
    #[ignore = "network"]
    fn fetches_and_decodes_github_favicon() {
        let bytes = fetch("github.com").expect("favicon bytes");
        let image = decode(&bytes).expect("decoded favicon");
        assert!(image.size(0).width.0 > 0);
    }
}
