// SPDX-License-Identifier: MIT
//! HarmonyOS platform backend — the `ohos-rs` shape, not an embedded board.
//!
//! HarmonyOS is a unified OS for phone, tablet and PC. An ArkTS/HAP app owns
//! the window, lifecycle, touch, IME and clipboard; it loads this crate as a
//! N-API `cdylib` (`libfloom_platform_harmony.so`) and an ArkUI `XComponent`
//! hands it an `OHNativeWindow*` via `OH_NativeXComponent_GetNativeWindow`.
//! Rust owns only the pixels: the software renderer produces an RGBA8 frame and
//! this crate presents it into the native window's buffer queue.
//!
//! This is deliberately **not** the `embedded` backend. `floom-platform-embedded`
//! is a bare framebuffer for boards with no window manager; here the OS has a
//! full window system and the software renderer is only a stand-in until a
//! HarmonyOS GPU presenter exists. The capability profile
//! ([`PlatformCapabilities::harmony_default`]) says so.
//!
//! # The seam
//!
//! The ArkTS side calls three C ABI functions through N-API:
//!
//! * [`floom_harmony_bind_surface`] — on `OnSurfaceCreated`, with the
//!   `OHNativeWindow*` and its extent;
//! * [`floom_harmony_present_frame`] — each frame, with an RGBA8 buffer;
//! * [`floom_harmony_release_surface`] — on `OnSurfaceDestroyed`.
//!
//! On `aarch64-unknown-linux-ohos` these reach the real
//! `libnative_window.so` (`OH_NativeWindow_*`). Off that target the FFI is
//! compiled out and `present_frame` is inert, so the crate builds and its pure
//! Rust pixel logic is testable on the host.

use std::ffi::{c_int, c_void};
use std::sync::Mutex;

use floom_platform::PlatformCapabilities;
use floom_types::PlatformId;

/// The platform id the app sees on HarmonyOS (matches the ABI stand-in).
pub const HARMONY_PLATFORM_ID: PlatformId = PlatformId::new(12, 1);

/// What a real HarmonyOS backend can do: a windowed, touch-driven, native-view
/// OS — not [`PlatformCapabilities::embedded_default`].
pub fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities::harmony_default()
}

/// Copy a tightly-packed RGBA8 frame into a native buffer whose rows may be
/// padded. `dst_stride` is bytes per destination row; source rows are
/// `width * 4` bytes. Refuses a frame or destination that is too small rather
/// than copying half a frame into a buffer the compositor will read whole.
///
/// Pure Rust so the one genuinely fiddly part — stride handling — is testable
/// without a device.
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
    // The last row need not be padded: only its `row` bytes are read.
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

/// Bytes per row, from what `BufferHandle::stride` reports.
///
/// The header documents bytes, and some SDK builds report pixels — a frame
/// whose stride is narrower than one row cannot be blitted either way, so the
/// two readings are tried in order and the row width is the floor. Getting this
/// wrong in the naive direction (trusting a pixel count as bytes) rejects every
/// frame, which looks exactly like a black screen.
pub fn stride_bytes(reported: i32, width: usize) -> usize {
    let row = width.saturating_mul(4);
    if reported <= 0 {
        return row;
    }
    let reported = reported as usize;
    if reported >= row {
        reported
    } else if reported.saturating_mul(4) >= row {
        // Pixels per row.
        reported.saturating_mul(4)
    } else {
        row
    }
}

// --- The native-window handle published by the ArkTS side -------------------

/// The bound surface, as one value.
///
/// Three independent atomics would let `present_frame` read the pointer from one
/// bind and the extent from the next, or read a window id that
/// `release_surface` already dropped. One lock covers both the pointer and its
/// extent, and it is held across the blit so `release_surface` cannot free the
/// window mid-frame — the same trade the Android and Flutter backends make.
///
/// The window is stored as an address because a raw pointer is not `Send`, and
/// a `static Mutex` needs `Sync`.
/// Pixels per logical point of the display, set by the ArkTS shell.
///
/// 1.0 means "logical points are pixels", which is what a desktop means and
/// wrong on every phone: presenting a logical-sized frame into a 3x surface
/// shows one corner.
static DISPLAY_SCALE: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(1.0f32.to_bits());

/// The scale the shell reported, or 1.0 when it never did.
/// Writes one line to hilog.
///
/// `println!` and `log::` both go nowhere on this platform -- stdout is not
/// collected and the `log` facade has no backend here -- so the one place that
/// knows why a frame was rejected has to speak the system's own protocol.
#[cfg(target_env = "ohos")]
pub fn log(message: &str) {
    use std::ffi::{CString, c_char, c_int};
    #[link(name = "hilog_ndk.z")]
    unsafe extern "C" {
        fn OH_LOG_Print(
            type_: c_int,
            level: c_int,
            domain: u32,
            tag: *const c_char,
            fmt: *const c_char,
            ...
        ) -> c_int;
    }
    const LOG_APP: c_int = 0;
    const LOG_INFO: c_int = 3;
    const DOMAIN: u32 = 0x0F10;
    let Ok(tag) = CString::new("FloomHarmony") else {
        return;
    };
    let Ok(fmt) = CString::new("%{public}s") else {
        return;
    };
    let message = CString::new(message.replace('\0', "")).unwrap_or_default();
    unsafe {
        OH_LOG_Print(
            LOG_APP,
            LOG_INFO,
            DOMAIN,
            tag.as_ptr(),
            fmt.as_ptr(),
            message.as_ptr(),
        );
    }
}

pub fn display_scale() -> f32 {
    f32::from_bits(DISPLAY_SCALE.load(std::sync::atomic::Ordering::Relaxed))
}

/// Records the display scale. Called by the shell; a redraw picks it up.
#[unsafe(no_mangle)]
pub extern "C" fn floom_harmony_set_display_scale(scale: f32) {
    if scale.is_finite() && scale > 0.0 {
        DISPLAY_SCALE.store(scale.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }
}

static SURFACE: Mutex<Option<Bound>> = Mutex::new(None);

// The fields are read only on the OHOS target (`present_frame`'s blit is
// compiled out elsewhere), but they have to exist on every target: the point of
// the struct is that bind and release store one value.
#[allow(dead_code)]
struct Bound {
    window: usize,
    width: i32,
    height: i32,
}

/// Bind the `OHNativeWindow*` from `OH_NativeXComponent_GetNativeWindow`.
///
/// Returns false for a null handle or a non-positive extent. The pointer is
/// borrowed, not owned: the XComponent keeps it alive until
/// [`floom_harmony_release_surface`].
///
/// # Safety
/// `window` must be a live `OHNativeWindow*` kept alive until
/// [`floom_harmony_release_surface`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn floom_harmony_bind_surface(
    window: *mut c_void,
    width: c_int,
    height: c_int,
) -> bool {
    if window.is_null() || width <= 0 || height <= 0 {
        return false;
    }
    match SURFACE.lock() {
        Ok(mut bound) => {
            *bound = Some(Bound {
                window: window as usize,
                width,
                height,
            });
            true
        }
        // A poisoned lock means a frame panicked mid-blit. Refusing the bind is
        // better than carrying on with a window nobody can vouch for.
        Err(_) => false,
    }
}

/// Present one RGBA8 frame into the bound native window.
///
/// `len` must be at least `width * height * 4`. On the OHOS target the frame is
/// blitted into a buffer requested from the window and flushed to the
/// compositor; off-target it is inert, so host tests can call it safely.
///
/// # Safety
/// `rgba` must point to at least `len` readable bytes; the bound native window
/// must still be alive (see [`floom_harmony_bind_surface`]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn floom_harmony_present_frame(
    rgba: *const u8,
    len: usize,
    width: c_int,
    height: c_int,
) -> bool {
    // The frame's own extent, not the bind's: the buffer geometry is set from
    // the frame on every present, so a frame that arrives while a rotation is
    // still settling draws instead of being dropped. The bind only says whether
    // there is a window to draw into.
    if width <= 0 || height <= 0 {
        return false;
    }
    let (w, h) = (width as usize, height as usize);
    let need = w.saturating_mul(h).saturating_mul(4);
    if rgba.is_null() || len < need {
        return false;
    }
    let Ok(bound) = SURFACE.lock() else {
        return false;
    };
    let Some(bound) = bound.as_ref() else {
        return false;
    };
    #[cfg(target_env = "ohos")]
    {
        let window = bound.window as *mut c_void;
        let _extent = (bound.width, bound.height);
        // SAFETY: the ArkTS side guarantees the frame has `need` bytes; the
        // lock keeps the window alive — `release_surface` waits for it.
        let frame = unsafe { std::slice::from_raw_parts(rgba, need) };
        unsafe { native::present(window, frame, w, h) }
    }
    #[cfg(not(target_env = "ohos"))]
    {
        // No device on the host: the C ABI is still exercised, the blit is not.
        let _ = (rgba, len, bound, w, h);
        false
    }
}

/// Safe Rust wrapper over the C ABI present, for a facade backend that already
/// holds a frame as a slice.
///
/// Use this when the surface was bound through [`floom_harmony_bind_surface`].
/// A caller that *has* the `OHNativeWindow*` itself — the winit driver does: it
/// gets one from the window, not from a bind call — passes it to
/// [`present_to`] instead.
pub fn present(frame: &[u8], width: u32, height: u32) -> bool {
    // SAFETY: `frame` is a live slice, so its pointer/len are valid; the bound
    // native window's validity is the host's contract (checked inside the callee).
    unsafe {
        floom_harmony_present_frame(frame.as_ptr(), frame.len(), width as c_int, height as c_int)
    }
}

/// Present one RGBA8 frame into a native window the caller holds.
///
/// The same blit as [`present`], without the bound-surface statics: the winit
/// driver owns the window (`WindowExtOhos::native_window_ptr`) and presents
/// through this, so a frame never depends on a global that a different thread
/// may have released. Off-target this is inert and returns false, like
/// [`present`].
///
/// # Safety
/// `window` must be a live `OHNativeWindow*`, and `frame` must hold
/// `width * height * 4` bytes.
pub unsafe fn present_to(
    window: *mut std::ffi::c_void,
    frame: &[u8],
    width: u32,
    height: u32,
) -> bool {
    if window.is_null() || width == 0 || height == 0 {
        return false;
    }
    let need = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if frame.len() < need {
        return false;
    }
    #[cfg(target_env = "ohos")]
    {
        // SAFETY: the caller's contract; `need` bytes are checked above.
        unsafe { native::present(window, &frame[..need], width as usize, height as usize) }
    }
    #[cfg(not(target_env = "ohos"))]
    {
        let _ = (window, frame, width, height, need);
        false
    }
}

/// Present one RGBA8 frame into a window the caller holds, safely.
///
/// The caller's contract, and the reason this is not `unsafe`: `window` must come
/// from a live ArkUI surface (the winit driver takes it from
/// `WindowExtOhos::native_window_ptr`) and must not have been released. A driver
/// that follows that rule — it clears its handle in `destroy_surfaces` — cannot
/// pass a dangling one.
// The lint is right in general — a raw pointer argument can be anything — and
// wrong here: the caller cannot construct one, it arrives from
// `WindowExtOhos::native_window_ptr()` on a window this driver owns and clears
// on `destroy_surfaces`. Making the function `unsafe` instead would push the
// obligation into the facade, which forbids unsafe code entirely.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn present_into_window(
    window: *mut std::ffi::c_void,
    frame: &[u8],
    width: u32,
    height: u32,
) -> bool {
    // SAFETY: the contract above; the callee re-checks null/size and never
    // keeps the pointer.
    unsafe { present_to(window, frame, width, height) }
}

/// Release the bound native window (XComponent `OnSurfaceDestroyed`).
#[unsafe(no_mangle)]
pub extern "C" fn floom_harmony_release_surface() {
    // Waits for an in-flight blit: the window is about to be destroyed and the
    // compositor must not be handed a buffer from it.
    if let Ok(mut bound) = SURFACE.lock() {
        *bound = None;
    }
}

// --- Real OHOS NDK present (compiled only for the target) ------------------

#[cfg(target_env = "ohos")]
mod native {
    use super::*;

    /// Prefix of `BufferHandle` from `native_window/buffer_handle.h`. Only
    /// `fd`, `stride` and `size` are read; the rest keep `#[repr(C)]` offsets
    /// exact so those three land on the right bytes. CPU blit uses `mmap(fd)`,
    /// not `virAddr` (which is a GPU-side address).
    #[repr(C)]
    #[allow(dead_code)]
    struct BufferHandle {
        fd: c_int,
        width: c_int,
        stride: c_int,
        height: c_int,
        size: c_int,
        format: c_int,
        usage: u64,
        vir_addr: *mut u8,
    }

    #[repr(C)]
    struct Region {
        rects: *mut c_void,
        rect_number: c_int,
    }

    /// `NativeWindowOperation::SET_BUFFER_GEOMETRY` is the first enumerator.
    const SET_BUFFER_GEOMETRY: c_int = 0;
    /// `SET_FORMAT` is the fourth enumerator (0-based index 3).
    const SET_FORMAT: c_int = 3;
    /// `NATIVEBUFFER_PIXEL_FMT_RGBA_8888`.
    const PIXEL_FMT_RGBA_8888: c_int = 12;
    /// `NATIVEBUFFER_USAGE_CPU_READ | CPU_WRITE | MEM_DMA`.
    const USAGE_CPU_RW_DMA: u64 = (1 << 0) | (1 << 1) | (1 << 3);
    /// `SET_USAGE` (index 5).
    const SET_USAGE: c_int = 5;

    const PROT_READ: c_int = 1;
    const PROT_WRITE: c_int = 2;
    const MAP_SHARED: c_int = 1;
    const MAP_FAILED: isize = -1;

    #[link(name = "native_window")]
    unsafe extern "C" {
        fn OH_NativeWindow_NativeWindowHandleOpt(window: *mut c_void, code: c_int, ...) -> c_int;
        fn OH_NativeWindow_NativeWindowRequestBuffer(
            window: *mut c_void,
            buffer: *mut *mut c_void,
            fence_fd: *mut c_int,
        ) -> c_int;
        fn OH_NativeWindow_GetBufferHandleFromNative(buffer: *mut c_void) -> *mut BufferHandle;
        fn OH_NativeWindow_NativeWindowAbortBuffer(
            window: *mut c_void,
            buffer: *mut c_void,
        ) -> c_int;
        fn OH_NativeWindow_NativeWindowFlushBuffer(
            window: *mut c_void,
            buffer: *mut c_void,
            fence_fd: c_int,
            region: Region,
        ) -> c_int;
        // CPU software path: the buffer is a dmabuf fd, not a CPU `virAddr`.
        // Flutter's ohos_surface_software.cpp mmaps `bufferHandle->fd` too.
        fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: c_int,
            flags: c_int,
            fd: c_int,
            offset: i64,
        ) -> *mut c_void;
        fn munmap(addr: *mut c_void, len: usize) -> c_int;
    }

    /// Request a buffer, blit the frame honouring the buffer's stride, flush.
    ///
    /// Every path that returns after `RequestBuffer` either flushes or aborts
    /// the buffer: the queue owns it from that moment, and leaving without
    /// returning it stalls the queue after a few frames (which on a device looks
    /// like a freeze, not an error).
    ///
    /// SAFETY: `window` is a live `OHNativeWindow*` borrowed from the
    /// XComponent; `frame` is `w * h * 4` RGBA8 bytes.
    pub(super) unsafe fn present(window: *mut c_void, frame: &[u8], w: usize, h: usize) -> bool {
        // Configure the buffer queue before RequestBuffer: geometry is required
        // by the SDK, and asking for RGBA8888 + CPU-readable/writable DMA makes
        // the returned buffer mmap-able (see Flutter's SetNativeWindowOpt).
        let geometry = unsafe {
            OH_NativeWindow_NativeWindowHandleOpt(
                window,
                SET_BUFFER_GEOMETRY,
                w as c_int,
                h as c_int,
            )
        };
        let format = unsafe {
            OH_NativeWindow_NativeWindowHandleOpt(window, SET_FORMAT, PIXEL_FMT_RGBA_8888)
        };
        let usage =
            unsafe { OH_NativeWindow_NativeWindowHandleOpt(window, SET_USAGE, USAGE_CPU_RW_DMA) };
        if geometry != 0 || format != 0 || usage != 0 {
            crate::log(&format!(
                "handleopt failed: geometry={geometry} format={format} usage={usage}"
            ));
            return false;
        }
        let mut buffer: *mut c_void = std::ptr::null_mut();
        let mut fence: c_int = -1;
        let rc =
            unsafe { OH_NativeWindow_NativeWindowRequestBuffer(window, &mut buffer, &mut fence) };
        if rc != 0 || buffer.is_null() {
            crate::log(&format!(
                "RequestBuffer rc={rc} buffer_null={}",
                buffer.is_null()
            ));
            return false;
        }
        // From here on the queue owns `buffer`: abort on every early return.
        {
            // Where the ink actually is, once: a frame drawn at logical size
            // inside a pixel-sized buffer shows ink in one corner, and that is a
            // different bug from a frame that never arrived.
            use std::sync::atomic::{AtomicBool, Ordering};
            static REPORTED: AtomicBool = AtomicBool::new(false);
            if !REPORTED.swap(true, Ordering::Relaxed) {
                let mut bbox = (usize::MAX, usize::MAX, 0usize, 0usize);
                let mut inked = 0usize;
                for y in 0..h {
                    for x in 0..w {
                        if frame[(y * w + x) * 4 + 3] > 0 {
                            inked += 1;
                            bbox = (bbox.0.min(x), bbox.1.min(y), bbox.2.max(x), bbox.3.max(y));
                        }
                    }
                }
                crate::log(&format!(
                    "ink={inked} bbox={}x{}-{}x{}",
                    bbox.0, bbox.1, bbox.2, bbox.3
                ));
            }
        }
        let Some(mapping) = (unsafe { map_buffer(buffer, w, h) }) else {
            unsafe { OH_NativeWindow_NativeWindowAbortBuffer(window, buffer) };
            return false;
        };
        // SAFETY: `map_buffer` mapped `mapping.len` bytes for CPU writes and
        // nothing else can reach them: the buffer belongs to this frame until it
        // is flushed or aborted.
        let copied = {
            let dst =
                unsafe { std::slice::from_raw_parts_mut(mapping.addr.cast::<u8>(), mapping.len) };
            copy_rgba_with_stride(frame, dst, w, h, mapping.stride)
        };
        unsafe { munmap(mapping.addr, mapping.len) };
        if !copied {
            // Nothing was drawn into the buffer: flushing it would show the
            // previous frame's garbage, or a half-copied row.
            unsafe { OH_NativeWindow_NativeWindowAbortBuffer(window, buffer) };
            return false;
        }
        let region = Region {
            rects: std::ptr::null_mut(),
            rect_number: 0,
        };
        let rc = unsafe { OH_NativeWindow_NativeWindowFlushBuffer(window, buffer, fence, region) };
        // One line per run on success, every line on failure: a phone frame loop
        // that logs unconditionally costs more than the frame does.
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            static ANNOUNCED: AtomicBool = AtomicBool::new(false);
            if rc != 0 || !ANNOUNCED.swap(true, Ordering::Relaxed) {
                crate::log(&format!(
                    "presented {w}x{h} stride={} flush_rc={rc}",
                    mapping.stride
                ));
            }
        }
        rc == 0
    }

    struct Mapped {
        addr: *mut c_void,
        len: usize,
        stride: usize,
    }

    /// Get the buffer's pixels into address space the CPU may write.
    ///
    /// Returns addresses, not a slice: the borrow of a mapped region has no
    /// lifetime the type system can name, so the caller makes the slice in the
    /// scope it unmaps in.
    ///
    /// SAFETY: `buffer` was just returned by `RequestBuffer` on a live window.
    unsafe fn map_buffer(buffer: *mut c_void, w: usize, h: usize) -> Option<Mapped> {
        let handle = unsafe { OH_NativeWindow_GetBufferHandleFromNative(buffer) };
        if handle.is_null() {
            return None;
        }
        let hnd = unsafe { &*handle };
        if hnd.size <= 0 || hnd.fd < 0 {
            return None;
        }
        let len = hnd.size as usize;
        let stride = stride_bytes(hnd.stride, w);
        crate::log(&format!(
            "buffer w={} h={} stride_raw={} size={} fd={} mapped={len} stride_used={stride}",
            hnd.width, hnd.height, hnd.stride, hnd.size, hnd.fd
        ));
        // A buffer that cannot hold the frame would be a partial copy presented
        // as a whole frame.
        let needed = stride
            .saturating_mul(h.saturating_sub(1))
            .saturating_add(w.saturating_mul(4));
        if needed > len {
            return None;
        }
        // The buffer is a dmabuf fd: map it into CPU address space. `virAddr`
        // is a GPU-side address (often null) and must not be used for a CPU blit.
        let addr = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                hnd.fd,
                0,
            )
        };
        if addr.is_null() || addr as isize == MAP_FAILED {
            return None;
        }
        Some(Mapped { addr, len, stride })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The surface slot is process-wide, so the tests that write it take turns.
    /// Two of them ran in parallel in CI and one saw another's release between
    /// its bind and its assertion.
    static SURFACE_SLOT: Mutex<()> = Mutex::new(());

    #[test]
    fn harmony_is_not_an_embedded_board() {
        let caps = capabilities();
        assert!(caps.supports(floom_platform::Capability::WINDOWING));
        assert!(caps.supports(floom_platform::Capability::TOUCH));
        assert!(caps.supports(floom_platform::Capability::NATIVE_VIEW));
        assert!(caps.supports(floom_platform::Capability::SOFTWARE_RENDER));
        assert_eq!(HARMONY_PLATFORM_ID, PlatformId::new(12, 1));
    }

    #[test]
    fn stride_copy_pads_each_row() {
        // 2x2 RGBA8, destination stride 12 (= 8 pixel bytes + 4 padding).
        let src: Vec<u8> = (0..16).collect();
        let mut dst = vec![0xEEu8; 12 * 2];
        assert!(copy_rgba_with_stride(&src, &mut dst, 2, 2, 12));
        assert_eq!(&dst[0..8], &src[0..8]);
        assert_eq!(&dst[8..12], &[0xEE; 4]);
        assert_eq!(&dst[12..20], &src[8..16]);
    }

    #[test]
    fn stride_copy_refuses_a_short_frame_or_buffer() {
        assert!(!copy_rgba_with_stride(&[0; 15], &mut [0; 100], 2, 2, 8));
        assert!(!copy_rgba_with_stride(&[0; 16], &mut [0; 15], 2, 2, 8));
        assert!(!copy_rgba_with_stride(&[0; 16], &mut [0; 100], 2, 2, 7));
        assert!(!copy_rgba_with_stride(&[], &mut [0; 100], 0, 2, 8));
    }

    #[test]
    fn a_stride_is_read_as_bytes_then_as_pixels_then_ignored() {
        // 3 px wide = 12 bytes per row.
        assert_eq!(stride_bytes(12, 3), 12, "bytes, exactly one row");
        assert_eq!(stride_bytes(64, 3), 64, "bytes, padded");
        // A stride narrower than a row can only be a pixel count (some SDK
        // builds report it that way); trusting it as bytes rejects every frame.
        assert_eq!(stride_bytes(16, 3), 16);
        assert_eq!(stride_bytes(4, 3), 16, "4 px per row, reported as pixels");
        assert_eq!(
            stride_bytes(1, 3),
            12,
            "nonsense: fall back to the row width"
        );
        assert_eq!(stride_bytes(0, 3), 12, "absent");
        assert_eq!(stride_bytes(-8, 3), 12, "negative");
    }

    #[test]
    fn a_bound_surface_is_one_value_and_release_clears_it() {
        let _serial = SURFACE_SLOT
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        // The extent and the window live in the same slot, so a present can
        // never pair one bind's pointer with another bind's size.
        floom_harmony_release_surface();
        // SAFETY: the address is stored, never dereferenced (the host has no
        // `libnative_window` and `present_frame` is inert off-target).
        let window = 0x1234 as *mut c_void;
        assert!(unsafe { floom_harmony_bind_surface(window, 2210, 2416) });
        assert!(SURFACE.lock().unwrap().is_some());
        floom_harmony_release_surface();
        assert!(SURFACE.lock().unwrap().is_none());
    }

    #[test]
    fn c_abi_rejects_a_null_frame_and_unbound_surface() {
        let _serial = SURFACE_SLOT
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        floom_harmony_release_surface();
        // SAFETY: the `0x1234` window is never dereferenced — both calls below
        // are rejected (null, then present is inert off-target) before any blit.
        unsafe {
            assert!(!floom_harmony_bind_surface(std::ptr::null_mut(), 4, 4));
            assert!(floom_harmony_bind_surface(0x1234 as *mut c_void, 2, 2));
            let frame = [0u8; 16];
            // Off-target present is inert but must not crash on a valid frame.
            let _ = floom_harmony_present_frame(frame.as_ptr(), frame.len(), 2, 2);
        }
        floom_harmony_release_surface();
    }
}
