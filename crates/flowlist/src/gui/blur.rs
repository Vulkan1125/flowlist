//! The macOS window glass.
//!
//! GPUI opens a behind-window `NSVisualEffectView` for
//! [`gpui::WindowBackgroundAppearance::Blurred`], but it fixes the material to
//! `.selection` and lets the view inherit the system appearance. `.selection`
//! is a thin source-list material: on a light Mac with a dark palette it
//! composites a pale wash under the tint, which reads as grey instead of glass.
//!
//! Dia and Zed set the material and appearance per window. We do the same:
//! `.underWindowBackground` supplies the deeper, saturated backdrop, and the
//! appearance is pinned to the palette so a light or dark theme never borrows
//! the other mode's material. The tint itself stays opaque enough to keep text
//! legible — the glass only changes how much of the desktop shows through, not
//! how much contrast the text gets.

// `objc` 0.2 expands `sel_impl!` with a `cfg(feature = "cargo-clippy")` check,
// which Cargo's check-cfg does not know about. Silence that third-party noise.
#![allow(unexpected_cfgs)]

use super::theme::Appearance;

/// `NSVisualEffectMaterialUnderWindowBackground`, the material Apple uses for
/// the backdrop behind a window's content.
#[cfg(target_os = "macos")]
const UNDER_WINDOW_BACKGROUND: i64 = 21;

/// `NSVisualEffectBlendingModeBehindWindow`.
#[cfg(target_os = "macos")]
const BEHIND_WINDOW: i64 = 0;

#[cfg(target_os = "macos")]
mod platform {
    use super::{Appearance, BEHIND_WINDOW, UNDER_WINDOW_BACKGROUND};
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe fn ns_string(text: &str) -> *mut Object {
        let text = CString::new(text).expect("appearance names have no interior NUL");
        unsafe { msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()] }
    }

    unsafe fn class_name(view: *mut Object) -> String {
        let name: *mut Object = unsafe { msg_send![view, className] };
        let utf8: *const c_char = unsafe { msg_send![name, UTF8String] };
        if utf8.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(utf8) }
                .to_string_lossy()
                .into_owned()
        }
    }

    #[allow(unexpected_cfgs)]
    pub(super) fn sync(window: &gpui::Window, appearance: Appearance) -> Option<&'static str> {
        use raw_window_handle::RawWindowHandle;

        let handle = raw_window_handle::HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return None;
        };
        let view = appkit.ns_view.as_ptr().cast::<Object>();

        // SAFETY: `window_handle` returned an AppKit handle, so the pointer is
        // a live `NSView` owned by the window for the duration of this call.
        // All messages below are main-thread AppKit calls, and `render` runs on
        // the main thread.
        unsafe {
            let ns_window: *mut Object = msg_send![view, window];
            if ns_window.is_null() {
                return None;
            }
            let content: *mut Object = msg_send![ns_window, contentView];
            if content.is_null() {
                return None;
            }
            let subviews: *mut Object = msg_send![content, subviews];
            if subviews.is_null() {
                return None;
            }
            let count: usize = msg_send![subviews, count];
            let mut blur_view = std::ptr::null_mut();
            for index in 0..count {
                let candidate: *mut Object = msg_send![subviews, objectAtIndex: index];
                if class_name(candidate) == "BlurredView" {
                    blur_view = candidate;
                    break;
                }
            }
            if blur_view.is_null() {
                return None;
            }

            let _: () = msg_send![blur_view, setMaterial: UNDER_WINDOW_BACKGROUND];
            let _: () = msg_send![blur_view, setBlendingMode: BEHIND_WINDOW];
            let appearance_name = match appearance {
                Appearance::Light => "NSAppearanceNameAqua",
                Appearance::Dark => "NSAppearanceNameDarkAqua",
            };
            let ns_appearance: *mut Object =
                msg_send![class!(NSAppearance), appearanceNamed: ns_string(appearance_name)];
            if !ns_appearance.is_null() {
                let _: () = msg_send![blur_view, setAppearance: ns_appearance];
            }
        }

        Some(match appearance {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        })
    }

    /// Hide the three standard window buttons.
    ///
    /// The window is a custom glass surface, so the red/yellow/green controls
    /// read as debris over the header. `NSWindowButton` is an integer enum:
    /// close `0`, miniaturize `1`, zoom `2`. Returns `true` once every button
    /// is hidden, so the caller can retry on a later frame if AppKit has not
    /// created the titlebar yet.
    pub(super) fn hide_traffic_lights(window: &gpui::Window) -> bool {
        use raw_window_handle::RawWindowHandle;

        let Ok(handle) = raw_window_handle::HasWindowHandle::window_handle(window) else {
            return false;
        };
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return false;
        };
        let view = appkit.ns_view.as_ptr().cast::<Object>();

        // SAFETY: `window_handle` returned an AppKit handle, so `view` is a
        // live `NSView` owned by the window. `standardWindowButton:` may
        // return null for a window style that has no such button; that is
        // checked before messaging. `render` runs on the main thread.
        unsafe {
            let ns_window: *mut Object = msg_send![view, window];
            if ns_window.is_null() {
                return false;
            }
            let mut all_hidden = true;
            for kind in 0..3usize {
                let button: *mut Object = msg_send![ns_window, standardWindowButton: kind];
                if button.is_null() {
                    all_hidden = false;
                    continue;
                }
                let _: () = msg_send![button, setHidden: true];
            }
            all_hidden
        }
    }
}

/// Point the window's blur view at the right material and light/dark variant.
///
/// Returns `None` when there is no AppKit blur view to tune yet, so the caller
/// can retry on a later frame without treating it as an error.
#[cfg(target_os = "macos")]
pub(super) fn sync(window: &gpui::Window, appearance: Appearance) -> Option<&'static str> {
    platform::sync(window, appearance)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn sync(_window: &gpui::Window, _appearance: Appearance) -> Option<&'static str> {
    None
}

/// Remove the standard macOS window buttons from a custom-titled window.
///
/// Returns `true` once they are hidden; on other platforms there is nothing to
/// hide, so this reports success.
#[cfg(target_os = "macos")]
pub(super) fn hide_traffic_lights(window: &gpui::Window) -> bool {
    platform::hide_traffic_lights(window)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn hide_traffic_lights(_window: &gpui::Window) -> bool {
    true
}
