//! System-wide media play/pause for the full-screen reminder.

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::c_void;
    use std::thread;
    use std::time::Duration;

    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    const NSEVENT_TYPE_SYSTEM_DEFINED: usize = 14;
    const NSEVENT_SUBTYPE_MEDIA: i16 = 8;
    const NX_KEYTYPE_PLAY: i64 = 16;
    const KEY_DOWN: i64 = 0xA;
    const KEY_UP: i64 = 0xB;
    const CG_HID_EVENT_TAP: u32 = 0;

    #[repr(C)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventPost(tap: u32, event: *const c_void);
    }

    fn post(state: i64) -> bool {
        let flags = if state == KEY_DOWN { 0xA00u64 } else { 0xB00u64 };
        let data1 = (NX_KEYTYPE_PLAY << 16) | (state << 8);
        // SAFETY: the event is an autoreleased NSEvent. Every selector and
        // struct layout below comes from AppKit's public NSEvent factory, and
        // CGEventPost only consumes the backing event during this call.
        unsafe {
            let event: *mut Object = msg_send![
                class!(NSEvent),
                otherEventWithType: NSEVENT_TYPE_SYSTEM_DEFINED
                location: NSPoint { x: 0.0, y: 0.0 }
                modifierFlags: flags
                timestamp: 0.0f64
                windowNumber: 0isize
                context: std::ptr::null::<Object>()
                subtype: NSEVENT_SUBTYPE_MEDIA
                data1: data1
                data2: -1isize
            ];
            if event.is_null() {
                return false;
            }
            let cg_event: *mut c_void = msg_send![event, cgEvent];
            if cg_event.is_null() {
                return false;
            }
            CGEventPost(CG_HID_EVENT_TAP, cg_event);
            true
        }
    }

    pub(super) fn toggle_playback() -> bool {
        if !post(KEY_DOWN) {
            return false;
        }
        thread::sleep(Duration::from_millis(35));
        post(KEY_UP)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub(super) fn toggle_playback() -> bool {
        false
    }
}

pub(super) fn toggle_playback() -> bool {
    platform::toggle_playback()
}
