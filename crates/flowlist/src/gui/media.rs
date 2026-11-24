//! System media play/pause for the full-screen reminder.
//!
//! Pausing uses MediaRemote's transport command, not a synthetic media key.
//! macOS 26 gates media control behind **Device control and data access**; the
//! Reminders setting explains that grant. The transport command avoids
//! synthesising hardware-key events while the grant is in place.

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::{CStr, CString, c_char, c_void};
    use std::ptr;
    use std::sync::OnceLock;

    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    type SendCommand = unsafe extern "C" fn(u32, *mut c_void) -> u8;

    const MR_COMMAND_PLAY: u32 = 0;
    const MR_COMMAND_PAUSE: u32 = 1;
    const RTLD_LAZY: i32 = 1;
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

    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    static SEND_COMMAND: OnceLock<Option<SendCommand>> = OnceLock::new();

    fn send_command(command: u32) -> bool {
        let sender = SEND_COMMAND.get_or_init(|| {
            // SAFETY: both paths are NUL-terminated C strings that live for
            // this call. The handle is deliberately leaked for the process
            // lifetime; dlsym returns a function with the documented
            // MRMediaRemoteSendCommand signature.
            unsafe {
                let path = CString::new(
                    "/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote",
                )
                .ok()?;
                let handle = dlopen(path.as_ptr(), RTLD_LAZY);
                if handle.is_null() {
                    return None;
                }
                let symbol = CString::new("MRMediaRemoteSendCommand").ok()?;
                let pointer = dlsym(handle, symbol.as_ptr());
                if pointer.is_null() {
                    return None;
                }
                Some(std::mem::transmute::<*mut c_void, SendCommand>(pointer))
            }
        });
        let Some(sender) = sender else {
            return false;
        };
        // SAFETY: the function pointer came from MediaRemote, and the command
        // takes an optional dictionary that we do not need.
        unsafe { sender(command, ptr::null_mut()) != 0 }
    }

    pub(super) fn pause() -> bool {
        send_command(MR_COMMAND_PAUSE)
    }

    pub(super) fn resume() -> bool {
        send_command(MR_COMMAND_PLAY)
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
    /// public process list keeps a silent reminder from touching playback.
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

    /// Whether a media-capable app is still running. Used only before resuming,
    /// so quitting the player while the reminder is open cannot launch Music.
    pub(super) fn is_media_app_running() -> bool {
        // SAFETY: NSWorkspace and every returned object are autoreleased AppKit
        // values. The UTF8String bytes stay valid while the bundle object is
        // alive, which is the duration of this call.
        unsafe {
            let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
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
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub(super) fn pause() -> bool {
        false
    }

    pub(super) fn resume() -> bool {
        false
    }

    pub(super) fn is_audio_playing() -> bool {
        false
    }

    pub(super) fn is_media_app_running() -> bool {
        false
    }
}

pub(super) fn pause() -> bool {
    platform::pause()
}

pub(super) fn resume() -> bool {
    platform::resume()
}

pub(super) fn is_audio_playing() -> bool {
    platform::is_audio_playing()
}

pub(super) fn is_media_app_running() -> bool {
    platform::is_media_app_running()
}

#[cfg(test)]
mod tests {
    #[test]
    fn audio_and_player_queries_are_safe() {
        let _ = super::is_audio_playing();
        let _ = super::is_media_app_running();
    }
}
