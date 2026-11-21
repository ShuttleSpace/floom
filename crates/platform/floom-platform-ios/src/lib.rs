// SPDX-License-Identifier: MIT
//! iOS platform backend — a UIKit app, not an embedded board.
//!
//! iOS is a full OS: UIKit owns the window, lifecycle, touch, IME and
//! clipboard, and it links Rust as a **static library** (there is no dynamic
//! N-API loader on iOS; this is the symmetric case to HarmonyOS's `ohos-rs`
//! cdylib). A `UIView` backed by a `CGBitmapContext` or a `CAMetalLayer`
//! hands Rust its drawable pixel buffer; the software renderer produces an
//! RGBA8 frame and this crate blits it in.
//!
//! This is deliberately **not** the `embedded` backend: the OS has a real
//! window system, and the software renderer is only a stand-in until an iOS
//! GPU presenter exists. The capability profile
//! ([`PlatformCapabilities::ios_default`]) says so.
//!
//! # The seam
//!
//! The Swift/ObjC host calls three C ABI functions (bridged via the generated
//! header or a module map):
//!
//! * [`floom_ios_bind_drawable`] — with the `CGBitmapContext`'s `data` pointer,
//!   byte stride and extent;
//! * [`floom_ios_present_frame`] — each frame, with an RGBA8 buffer;
//! * [`floom_ios_release_drawable`] — when the view goes away.
//!
//! Keeping the drawable as a plain pixel pointer means the crate links with
//! only the Apple SDK's libc — no `objc2`/UIKit symbols are needed in Rust, so
//! the staticlib link is checkable with `xcrun` on this machine. All the UIKit
//! work (creating the context, refreshing the layer) stays on the host side.
//!
//! The window-level packages (clipboard, safe area, system chrome, soft
//! keyboard) live in [`packages`]. They use objc2/UIKit for the reads Rust can
//! do on its own, and a C ABI seam for facts UIKit only delivers to the host
//! (keyboard notifications, IME text) — see `packages/keyboard/soft_input`.

pub mod packages;

/// Host entry point: publish safe-area insets (logical points) read from
/// `UIView.safeAreaInsets`. No-op off iOS.
#[unsafe(no_mangle)]
pub extern "C" fn floom_ios_set_safe_area_insets(top: f64, right: f64, bottom: f64, left: f64) {
    #[cfg(target_os = "ios")]
    packages::safe_area::set_safe_area_insets(floom_types::Insets {
        top,
        right,
        bottom,
        left,
    });
    #[cfg(not(target_os = "ios"))]
    let _ = (top, right, bottom, left);
}

/// Host entry point: publish soft-keyboard visibility and height (logical
/// points) from `keyboardWillShow/Hide`. No-op off iOS.
#[unsafe(no_mangle)]
pub extern "C" fn floom_ios_set_keyboard_state(visible: bool, height: f64) {
    #[cfg(target_os = "ios")]
    packages::keyboard::soft_input::set_keyboard_state(visible, height);
    #[cfg(not(target_os = "ios"))]
    let _ = (visible, height);
}

/// Host entry point: publish IME text. `committed` distinguishes a finished
/// commit from a preedit; `text` is a NUL-terminated UTF-8 string.
///
/// # Safety
/// `text` must point to a NUL-terminated UTF-8 string valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn floom_ios_push_ime_input(committed: bool, text: *const std::ffi::c_char) {
    #[cfg(target_os = "ios")]
    packages::keyboard::soft_input::push_ime_input(committed, text);
    #[cfg(not(target_os = "ios"))]
    let _ = (committed, text);
}

use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};

use floom_platform::PlatformCapabilities;
use floom_types::PlatformId;

/// The platform id the app sees on iOS (matches the ABI stand-in).
pub const IOS_PLATFORM_ID: PlatformId = PlatformId::new(11, 1);

/// What a real iOS backend can do: windowed, touch-driven, native-view — not
/// [`PlatformCapabilities::embedded_default`].
pub fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities::ios_default()
}

/// Copy a tightly-packed RGBA8 frame into a `CGBitmapContext` drawable whose
/// rows are padded to `dst_stride` bytes. Refuses short frames/buffers rather
/// than copying half a frame. Pure Rust so stride handling is host-testable.
pub fn copy_rgba_with_stride(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    dst_stride: usize,
) -> bool {
    let row = width.saturating_mul(4);
    if row == 0 || dst_stride < row {
        return false;
    }
    if src.len() < row.saturating_mul(height) {
        return false;
    }
    let needed = dst_stride
        .saturating_mul(height.saturating_sub(1))
        .saturating_add(row);
    if dst.len() < needed {
        return false;
    }
    for y in 0..height {
        let s = y * row;
        let d = y * dst_stride;
        dst[d..d + row].copy_from_slice(&src[s..s + row]);
    }
    true
}

static DRAWABLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static DRAWABLE_STRIDE: AtomicI32 = AtomicI32::new(0);
static DRAWABLE_W: AtomicI32 = AtomicI32::new(0);
static DRAWABLE_H: AtomicI32 = AtomicI32::new(0);

/// Bind the `CGBitmapContext` drawable. `pixels` is borrowed for the view's
/// lifetime; `stride` is bytes per row (`CGBitmapContextGetBytesPerRow`).
/// # Safety
/// `pixels` must point to a drawable of at least `stride * height` writable
/// bytes that stays alive until [`floom_ios_release_drawable`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn floom_ios_bind_drawable(
    pixels: *mut c_void,
    stride: c_int,
    width: c_int,
    height: c_int,
) -> bool {
    if pixels.is_null() || stride <= 0 || width <= 0 || height <= 0 {
        return false;
    }
    DRAWABLE_STRIDE.store(stride, Ordering::SeqCst);
    DRAWABLE_W.store(width, Ordering::SeqCst);
    DRAWABLE_H.store(height, Ordering::SeqCst);
    DRAWABLE.store(pixels, Ordering::SeqCst);
    true
}

/// Present one RGBA8 frame into the bound drawable.
///
/// # Safety
/// `rgba` must point to at least `len` readable bytes; the bound drawable must
/// still be alive (see [`floom_ios_bind_drawable`]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn floom_ios_present_frame(
    rgba: *const u8,
    len: usize,
    width: c_int,
    height: c_int,
) -> bool {
    let pixels = DRAWABLE.load(Ordering::SeqCst);
    let stride = DRAWABLE_STRIDE.load(Ordering::SeqCst);
    if pixels.is_null()
        || stride <= 0
        || width != DRAWABLE_W.load(Ordering::SeqCst)
        || height != DRAWABLE_H.load(Ordering::SeqCst)
        || width <= 0
        || height <= 0
    {
        return false;
    }
    let (w, h) = (width as usize, height as usize);
    let need = w.saturating_mul(h).saturating_mul(4);
    if rgba.is_null() || len < need {
        return false;
    }
    let row = w * 4;
    let cap = stride as usize * (h - 1) + row;
    // SAFETY: the UIKit host guarantees a drawable of at least `cap` bytes and
    // keeps it alive between bind and release.
    let dst = unsafe { std::slice::from_raw_parts_mut(pixels.cast::<u8>(), cap) };
    let frame = unsafe { std::slice::from_raw_parts(rgba, need) };
    copy_rgba_with_stride(frame, dst, w, h, stride as usize)
}

/// Safe Rust wrapper over the C ABI present, for a facade backend that already
/// holds a frame as a slice.
pub fn present(frame: &[u8], width: u32, height: u32) -> bool {
    // SAFETY: `frame` is a live slice, so its pointer/len are valid; the bound
    // drawable's validity is the host's contract (checked inside the callee).
    unsafe { floom_ios_present_frame(frame.as_ptr(), frame.len(), width as c_int, height as c_int) }
}

/// Release the bound drawable.
#[unsafe(no_mangle)]
pub extern "C" fn floom_ios_release_drawable() {
    DRAWABLE.store(std::ptr::null_mut(), Ordering::SeqCst);
    DRAWABLE_STRIDE.store(0, Ordering::SeqCst);
    DRAWABLE_W.store(0, Ordering::SeqCst);
    DRAWABLE_H.store(0, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The drawable is one process-wide slot; two tests that bind it must not
    /// interleave, or one releases what the other is about to blit into.
    static DRAWABLE: Mutex<()> = Mutex::new(());

    #[test]
    fn ios_is_not_an_embedded_board() {
        let caps = capabilities();
        assert!(caps.supports(floom_platform::Capability::WINDOWING));
        assert!(caps.supports(floom_platform::Capability::TOUCH));
        assert!(caps.supports(floom_platform::Capability::NATIVE_VIEW));
        assert!(caps.supports(floom_platform::Capability::SOFTWARE_RENDER));
        assert_eq!(IOS_PLATFORM_ID, PlatformId::new(11, 1));
    }

    #[test]
    fn present_blits_into_a_strided_drawable() {
        let _guard = DRAWABLE.lock().unwrap_or_else(|e| e.into_inner());
        // 2x2 RGBA8, CGBitmapContext row stride 16 (= 8 pixel bytes + pad).
        let src: Vec<u8> = (0..16).collect();
        let mut drawable = vec![0xEEu8; 16 * 2];
        // SAFETY: `drawable` outlives the two calls and has 32 writable bytes.
        unsafe {
            assert!(floom_ios_bind_drawable(
                drawable.as_mut_ptr().cast(),
                16,
                2,
                2
            ));
            assert!(floom_ios_present_frame(src.as_ptr(), src.len(), 2, 2));
        }
        assert_eq!(&drawable[0..8], &src[0..8]);
        assert_eq!(&drawable[8..16], &[0xEE; 8]);
        assert_eq!(&drawable[16..24], &src[8..16]);
        floom_ios_release_drawable();
    }

    #[test]
    fn present_rejects_a_null_frame_wrong_size_or_unbound_drawable() {
        let _guard = DRAWABLE.lock().unwrap_or_else(|e| e.into_inner());
        floom_ios_release_drawable();
        let frame = [0u8; 16];
        // SAFETY: `frame` is live; the `0xDEAD` drawable is never dereferenced
        // because every present below is rejected before the blit.
        unsafe {
            assert!(!floom_ios_present_frame(frame.as_ptr(), frame.len(), 2, 2));
            assert!(floom_ios_bind_drawable(0xDEAD as *mut c_void, 8, 2, 2));
            assert!(!floom_ios_present_frame(frame.as_ptr(), 8, 2, 2));
            assert!(!floom_ios_present_frame(frame.as_ptr(), frame.len(), 3, 2));
        }
        floom_ios_release_drawable();
    }
}
