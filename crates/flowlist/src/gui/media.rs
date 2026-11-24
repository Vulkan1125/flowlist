//! System-wide media play/pause for the full-screen reminder.

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::{CStr, c_char, c_void};
    use std::ptr;
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
    const MEDIA_BUNDLES: &[&str] = &[
        "com.apple.Music",
        "com.apple.iTunes",
        "com.spotify.client",
        "com.apple.QuickTimePlayerX",
        "com.apple.TV",
        "org.videolan.vlc",
        "com.colliderli.iina",
        "com.google.Chrome",
        "com.apple.Safari",
        "org.mozilla.firefox",
        "company.thebrowser.Browser",
        "com.microsoft.edgemac",
        "com.brave.Browser",
        "com.operasoftware.Opera",
        "com.plexapp.plex",
    ];

    #[repr(C)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventPost(tap: u32, event: *const c_void);
    }

    #[repr(C)]
    struct AudioObjectPropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    #[link(name = "CoreAudio", kind = "framework")]
    unsafe extern "C" {
        fn AudioObjectGetPropertyData(
            object: u32,
            address: *const AudioObjectPropertyAddress,
            qualifier_size: u32,
            qualifier: *const c_void,
            data_size: *mut u32,
            out: *mut c_void,
        ) -> i32;
    }

    const AUDIO_SYSTEM_OBJECT: u32 = 1;
    const AUDIO_SCOPE_GLOBAL: u32 = 0x676c_6f62; // 'glob'
    const AUDIO_ELEMENT_MAIN: u32 = 0;
    const AUDIO_PROCESS_LIST: u32 = 0x7072_7323; // 'prs#'
    const AUDIO_PROCESS_IS_RUNNING_OUTPUT: u32 = 0x7069_726f; // 'piro'

    /// Whether any process is currently producing audio output. CoreAudio's
    /// public process list avoids sending a media key when nothing is playing,
    /// which would otherwise let macOS fall back to Music.
    pub(super) fn is_audio_playing() -> bool {
        let list_address = AudioObjectPropertyAddress {
            selector: AUDIO_PROCESS_LIST,
            scope: AUDIO_SCOPE_GLOBAL,
            element: AUDIO_ELEMENT_MAIN,
        };
        let mut ids = vec![0u32; 1024];
        let mut size = (ids.len() * std::mem::size_of::<u32>()) as u32;
        // SAFETY: the address and buffers are valid for the duration of the
        // call; CoreAudio writes at most `size` bytes into `ids`.
        let status = unsafe {
            AudioObjectGetPropertyData(
                AUDIO_SYSTEM_OBJECT,
                &list_address,
                0,
                ptr::null(),
                &mut size,
                ids.as_mut_ptr().cast(),
            )
        };
        if status != 0 {
            return false;
        }
        ids.truncate((size as usize / std::mem::size_of::<u32>()).min(ids.len()));
        for id in ids {
            let output_address = AudioObjectPropertyAddress {
                selector: AUDIO_PROCESS_IS_RUNNING_OUTPUT,
                scope: AUDIO_SCOPE_GLOBAL,
                element: AUDIO_ELEMENT_MAIN,
            };
            let mut running: u32 = 0;
            let mut running_size = std::mem::size_of::<u32>() as u32;
            let status = unsafe {
                AudioObjectGetPropertyData(
                    id,
                    &output_address,
                    0,
                    ptr::null(),
                    &mut running_size,
                    (&mut running as *mut u32).cast(),
                )
            };
            if status == 0 && running != 0 {
                return true;
            }
        }
        false
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
            let cg_event: *mut c_void = msg_send![event, CGEvent];
            if cg_event.is_null() {
                return false;
            }
            CGEventPost(CG_HID_EVENT_TAP, cg_event);
            true
        }
    }

    /// Sending a media key with no player open can launch Music, so the close
    /// path only resumes when a media-capable app is still running. Browsers
    /// are included because they own the now-playing session for web audio.
    pub(super) fn is_media_app_running() -> bool {
        // SAFETY: NSWorkspace and every returned object are autoreleased AppKit
        // values. The UTF8String bytes stay valid while the bundle object is
        // alive, which is the duration of this call.
        unsafe {
            let workspace: *mut Object =
                msg_send![class!(NSWorkspace), sharedWorkspace];
            if workspace.is_null() {
                return false;
            }
            let apps: *mut Object = msg_send![workspace, runningApplications];
            if apps.is_null() {
                return false;
            }
            let count: usize = msg_send![apps, count];
            for index in 0..count {
                let app: *mut Object = msg_send![apps, objectAtIndex: index];
                if app.is_null() {
                    continue;
                }
                let bundle: *mut Object = msg_send![app, bundleIdentifier];
                if bundle.is_null() {
                    continue;
                }
                let utf8: *const c_char = msg_send![bundle, UTF8String];
                if utf8.is_null() {
                    continue;
                }
                if let Ok(identifier) = CStr::from_ptr(utf8).to_str()
                    && MEDIA_BUNDLES.contains(&identifier)
                {
                    return true;
                }
            }
        }
        false
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
    pub(super) fn is_audio_playing() -> bool {
        false
    }

    pub(super) fn is_media_app_running() -> bool {
        false
    }

    pub(super) fn toggle_playback() -> bool {
        false
    }
}

pub(super) fn is_audio_playing() -> bool {
    platform::is_audio_playing()
}

pub(super) fn is_media_app_running() -> bool {
    platform::is_media_app_running()
}

pub(super) fn toggle_playback() -> bool {
    platform::toggle_playback()
}

#[cfg(test)]
mod tests {
    #[test]
    fn audio_query_is_safe_when_nothing_is_playing() {
        // Exercises the CoreAudio FFI. The answer depends on the machine's
        // current audio output, so only the call itself is asserted here.
        let _ = super::is_audio_playing();
        let _ = super::is_media_app_running();
    }
}
