//! A system-wide capture shortcut.
//!
//! GPUI has no global-shortcut API of its own. On macOS the smallest stable
//! route is Carbon's `RegisterEventHotKey`, which keeps working while another
//! app has focus and needs no Accessibility permission (unlike NSEvent global
//! monitors). The callback only posts to a channel; the actual window work
//! happens on GPUI's foreground thread.
//!
//! Everywhere else registration simply reports "not available" and the app
//! keeps working with the in-window capture bar.

/// Control + Option + Space, the same quick-entry chord OmniFocus uses.
const CAPTURE_KEY_CODE: u32 = 49; // kVK_Space
const CAPTURE_MODIFIERS: u32 = 0x1000 | 0x0800; // controlKey | optionKey

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::c_void;
    use std::sync::Mutex;

    use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

    type OSStatus = i32;
    type EventHandlerRef = *mut c_void;
    type EventHotKeyRef = *mut c_void;
    type EventTargetRef = *mut c_void;

    #[repr(C)]
    struct EventTypeSpec {
        event_class: u32,
        event_kind: u32,
    }

    #[repr(C)]
    struct EventHotKeyId {
        signature: u32,
        id: u32,
    }

    type EventHandler = extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> OSStatus;

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn GetApplicationEventTarget() -> EventTargetRef;
        fn InstallEventHandler(
            target: EventTargetRef,
            handler: Option<EventHandler>,
            num_types: usize,
            types: *const EventTypeSpec,
            user_data: *mut c_void,
            out_ref: *mut EventHandlerRef,
        ) -> OSStatus;
        fn RegisterEventHotKey(
            key_code: u32,
            modifiers: u32,
            hot_key_id: EventHotKeyId,
            target: EventTargetRef,
            options: u32,
            out_ref: *mut EventHotKeyRef,
        ) -> OSStatus;
    }

    /// `kEventClassKeyboard` / `kEventHotKeyPressed`.
    const KEYBOARD_EVENT: EventTypeSpec = EventTypeSpec {
        event_class: u32::from_be_bytes(*b"keyb"),
        event_kind: 5,
    };

    static CAPTURE_REQUESTS: Mutex<Option<UnboundedSender<()>>> = Mutex::new(None);

    extern "C" fn on_hot_key(
        _call: *mut c_void,
        _event: *mut c_void,
        _user_data: *mut c_void,
    ) -> OSStatus {
        if let Ok(sender) = CAPTURE_REQUESTS.lock()
            && let Some(sender) = sender.as_ref()
        {
            // A closed receiver just means the app is shutting down.
            let _ = sender.unbounded_send(());
        }
        0
    }

    pub(super) fn register() -> Option<UnboundedReceiver<()>> {
        let (sender, receiver) = unbounded();
        *CAPTURE_REQUESTS.lock().ok()? = Some(sender);
        // The refs are intentionally dropped: the registrations live for the
        // life of the process, and macOS removes them on exit.
        let mut handler_ref: EventHandlerRef = std::ptr::null_mut();
        let mut hot_key_ref: EventHotKeyRef = std::ptr::null_mut();
        let signature = EventHotKeyId {
            signature: u32::from_be_bytes(*b"FLST"),
            id: 1,
        };
        unsafe {
            let target = GetApplicationEventTarget();
            let status = InstallEventHandler(
                target,
                Some(on_hot_key),
                1,
                &KEYBOARD_EVENT,
                std::ptr::null_mut(),
                &mut handler_ref,
            );
            if status != 0 {
                eprintln!("flowlist: could not install the capture hotkey handler ({status})");
                *CAPTURE_REQUESTS.lock().ok()? = None;
                return None;
            }
            let status = RegisterEventHotKey(
                super::CAPTURE_KEY_CODE,
                super::CAPTURE_MODIFIERS,
                signature,
                target,
                0,
                &mut hot_key_ref,
            );
            if status != 0 {
                eprintln!("flowlist: could not register the capture hotkey ({status})");
                *CAPTURE_REQUESTS.lock().ok()? = None;
                return None;
            }
        }
        Some(receiver)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use futures::channel::mpsc::UnboundedReceiver;

    pub(super) fn register() -> Option<UnboundedReceiver<()>> {
        None
    }
}

/// Register the capture shortcut. Returns a stream that yields once for every
/// press, or `None` when this platform has no global-shortcut support.
pub fn register() -> Option<futures::channel::mpsc::UnboundedReceiver<()>> {
    platform::register()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_shortcut_is_control_option_space() {
        assert_eq!(super::CAPTURE_KEY_CODE, 49);
        assert_eq!(super::CAPTURE_MODIFIERS, 0x1800);
    }
}
