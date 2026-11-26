//! System media play/pause for the full-screen reminder.
//!
//! Pausing uses MediaRemote's transport command, not a synthetic media key.
//! macOS 26 gates media control behind **Device control and data access**; the
//! Reminders setting explains that grant. The transport command avoids
//! synthesising hardware-key events while the grant is in place.

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::{CString, c_char, c_void};
    use std::ptr;
    use std::sync::OnceLock;

    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    type SendCommand = unsafe extern "C" fn(u32, *mut c_void) -> u8;

    const MR_COMMAND_PAUSE: u32 = 1;
    const RTLD_LAZY: i32 = 1;
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

}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub(super) fn pause() -> bool {
        false
    }

    pub(super) fn is_audio_playing() -> bool {
        false
    }

}

pub(super) fn pause() -> bool {
    platform::pause()
}

pub(super) fn is_audio_playing() -> bool {
    platform::is_audio_playing()
}

#[cfg(test)]
mod tests {
    #[test]
    fn audio_and_player_queries_are_safe() {
        let _ = super::is_audio_playing();
    }
}
