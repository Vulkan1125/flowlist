//! The system eyedropper, used by Plain's accent controls.
//!
//! `NSColorSampler` is the same magnifier macOS puts in the colour panel: it
//! dims the screen, lets the pointer sample any pixel (the wallpaper included)
//! and then hands back one colour. The sampler calls its handler on the main
//! thread, so a shared slot is enough to carry the result back into the GPUI
//! update that started it.

use std::cell::RefCell;
use std::rc::Rc;

/// Where the sampler writes its result. The outer `None` means the picker is
/// still open; `Some(None)` means the user cancelled, and `Some(Some(rgb))`
/// carries the sampled `0xRRGGBB`.
pub type PickSlot = Rc<RefCell<Option<Option<u32>>>>;

/// Show the eyedropper. Returns `false` when the system sampler is not
/// available, in which case the slot already holds the cancelled result.
#[cfg(target_os = "macos")]
pub fn sample(slot: PickSlot) -> bool {
    use block::ConcreteBlock;
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let sampler: *mut Object = msg_send![class!(NSColorSampler), new];
        if sampler.is_null() {
            *slot.borrow_mut() = Some(None);
            return false;
        }
        let callback = ConcreteBlock::new(move |color: *mut Object| {
            let picked = if color.is_null() {
                None
            } else {
                let space: *mut Object = msg_send![class!(NSColorSpace), sRGBColorSpace];
                let converted: *mut Object = msg_send![color, colorUsingColorSpace: space];
                if converted.is_null() {
                    None
                } else {
                    let red: f64 = msg_send![converted, redComponent];
                    let green: f64 = msg_send![converted, greenComponent];
                    let blue: f64 = msg_send![converted, blueComponent];
                    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
                    Some((channel(red) << 16) | (channel(green) << 8) | channel(blue))
                }
            };
            *slot.borrow_mut() = Some(picked);
        });
        let callback = callback.copy();
        let _: () = msg_send![sampler, showSamplerWithSelectionHandler: &*callback];
        // The sampler is still on screen when this returns, and the system
        // calls the block later. Neither is released here: one short-lived
        // pick leaks a tiny object, which is safer than a callback into freed
        // memory.
        let _ = (sampler, callback);
        true
    }
}

#[cfg(not(target_os = "macos"))]
pub fn sample(slot: PickSlot) -> bool {
    *slot.borrow_mut() = Some(None);
    false
}
