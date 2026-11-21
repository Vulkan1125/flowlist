//! Alternate Dock icons.
//!
//! macOS has no per-app icon picker. `setApplicationIconImage` swaps the icon
//! shown in the Dock and the app switcher for the lifetime of the process; the
//! Finder still shows the bundle's built-in artwork. The choice is saved with
//! the session and applied on the next launch.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppIcon {
    #[default]
    Blue,
    Red,
    Green,
    Orange,
    Purple,
    Graphite,
}

impl AppIcon {
    pub const ALL: [AppIcon; 6] = [
        AppIcon::Blue,
        AppIcon::Red,
        AppIcon::Green,
        AppIcon::Orange,
        AppIcon::Purple,
        AppIcon::Graphite,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            AppIcon::Blue => "blue",
            AppIcon::Red => "red",
            AppIcon::Green => "green",
            AppIcon::Orange => "orange",
            AppIcon::Purple => "purple",
            AppIcon::Graphite => "graphite",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|icon| icon.slug() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            AppIcon::Blue => "Blue",
            AppIcon::Red => "Red",
            AppIcon::Green => "Green",
            AppIcon::Orange => "Orange",
            AppIcon::Purple => "Purple",
            AppIcon::Graphite => "Graphite",
        }
    }

    /// The swatch colour shown in Settings, matching the tile's mid tone.
    pub fn color(self) -> u32 {
        match self {
            AppIcon::Blue => 0x3D8CD6,
            AppIcon::Red => 0xDC4C3E,
            AppIcon::Green => 0x2E8B57,
            AppIcon::Orange => 0xE07A2F,
            AppIcon::Purple => 0x7B5CC7,
            AppIcon::Graphite => 0x3E3E42,
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            AppIcon::Blue => include_bytes!("../../../assets/icons/Flowlist-blue.png"),
            AppIcon::Red => include_bytes!("../../../assets/icons/Flowlist-red.png"),
            AppIcon::Green => include_bytes!("../../../assets/icons/Flowlist-green.png"),
            AppIcon::Orange => include_bytes!("../../../assets/icons/Flowlist-orange.png"),
            AppIcon::Purple => include_bytes!("../../../assets/icons/Flowlist-purple.png"),
            AppIcon::Graphite => include_bytes!("../../../assets/icons/Flowlist-graphite.png"),
        }
    }
}

pub fn apply(icon: AppIcon) {
    #[cfg(target_os = "macos")]
    set_dock_icon(icon.bytes());
    #[cfg(not(target_os = "macos"))]
    let _ = icon;
}

#[cfg(target_os = "macos")]
fn set_dock_icon(bytes: &[u8]) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    // The call is made from the main thread when the app is built and from a
    // click handler, so AppKit is already on the right thread.
    unsafe {
        let data: *mut Object = msg_send![
            class!(NSData),
            dataWithBytes: bytes.as_ptr()
            length: bytes.len()
        ];
        if data.is_null() {
            return;
        }
        let image: *mut Object = msg_send![class!(NSImage), alloc];
        let image: *mut Object = msg_send![image, initWithData: data];
        if image.is_null() {
            return;
        }
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, setApplicationIconImage: image];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_a_unique_slug() {
        let mut slugs: Vec<&str> = AppIcon::ALL.iter().map(|icon| icon.slug()).collect();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), AppIcon::ALL.len());
        for icon in AppIcon::ALL {
            assert_eq!(AppIcon::from_slug(icon.slug()), Some(icon));
        }
    }

    #[test]
    fn each_icon_embeds_an_image() {
        for icon in AppIcon::ALL {
            assert!(icon.bytes().len() > 1024, "{} icon is empty", icon.slug());
        }
    }
}
