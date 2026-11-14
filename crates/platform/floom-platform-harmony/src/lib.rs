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
use std::sync::atomic::{AtomicI32, AtomicPtr, Ordering};

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

// --- The native-window handle published by the ArkTS side -------------------

static SURFACE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static SURFACE_W: AtomicI32 = AtomicI32::new(0);
static SURFACE_H: AtomicI32 = AtomicI32::new(0);

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
    SURFACE_W.store(width, Ordering::SeqCst);
    SURFACE_H.store(height, Ordering::SeqCst);
    SURFACE.store(window, Ordering::SeqCst);
    true
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
    let w = SURFACE_W.load(Ordering::SeqCst);
    let h = SURFACE_H.load(Ordering::SeqCst);
    if width != w || height != h || width <= 0 || height <= 0 {
        return false;
    }
    let (w, h) = (width as usize, height as usize);
    let need = w.saturating_mul(h).saturating_mul(4);
    if rgba.is_null() || len < need {
        return false;
    }
    #[cfg(target_env = "ohos")]
    {
        let window = SURFACE.load(Ordering::SeqCst);
        if window.is_null() {
            return false;
        }
        // SAFETY: the ArkTS side guarantees the frame has `need` bytes and the
        // window is alive between bind and release.
        let frame = unsafe { std::slice::from_raw_parts(rgba, need) };
        unsafe { native::present(window, frame, w, h) }
    }
    #[cfg(not(target_env = "ohos"))]
    {
        // No device on the host: the C ABI is still exercised, the blit is not.
        let _ = (rgba, len, w, h);
        false
    }
}

/// Safe Rust wrapper over the C ABI present, for a facade backend that already
/// holds a frame as a slice.
pub fn present(frame: &[u8], width: u32, height: u32) -> bool {
    // SAFETY: `frame` is a live slice, so its pointer/len are valid; the bound
    // native window's validity is the host's contract (checked inside the callee).
    unsafe { floom_harmony_present_frame(frame.as_ptr(), frame.len(), width as c_int, height as c_int) }
}

/// Release the bound native window (XComponent `OnSurfaceDestroyed`).
#[unsafe(no_mangle)]
pub extern "C" fn floom_harmony_release_surface() {
    SURFACE.store(std::ptr::null_mut(), Ordering::SeqCst);
    SURFACE_W.store(0, Ordering::SeqCst);
    SURFACE_H.store(0, Ordering::SeqCst);
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
    /// SAFETY: `window` is a live `OHNativeWindow*` borrowed from the
    /// XComponent; `frame` is `w * h * 4` RGBA8 bytes.
    pub(super) unsafe fn present(window: *mut c_void, frame: &[u8], w: usize, h: usize) -> bool {
        // Configure the buffer queue before RequestBuffer: geometry is required
        // by the SDK, and asking for RGBA8888 + CPU-readable/writable DMA makes
        // the returned buffer mmap-able (see Flutter's SetNativeWindowOpt).
        let rc = unsafe {
            OH_NativeWindow_NativeWindowHandleOpt(
                window,
                SET_BUFFER_GEOMETRY,
                w as c_int,
                h as c_int,
            )
        };
        if rc == 0 {
            unsafe {
                OH_NativeWindow_NativeWindowHandleOpt(window, SET_FORMAT, PIXEL_FMT_RGBA_8888);
                OH_NativeWindow_NativeWindowHandleOpt(window, SET_USAGE, USAGE_CPU_RW_DMA);
            }
        }
        if rc != 0 {
            return false;
        }
        let mut buffer: *mut c_void = std::ptr::null_mut();
        let mut fence: c_int = -1;
        let rc =
            unsafe { OH_NativeWindow_NativeWindowRequestBuffer(window, &mut buffer, &mut fence) };
        if rc != 0 || buffer.is_null() {
            return false;
        }
        let handle = unsafe { OH_NativeWindow_GetBufferHandleFromNative(buffer) };
        if handle.is_null() {
            return false;
        }
        let hnd = unsafe { &*handle };
        if hnd.stride <= 0 || hnd.size <= 0 || hnd.fd < 0 {
            return false;
        }
        let stride = hnd.stride as usize;
        let cap = hnd.size as usize;
        // The buffer is a dmabuf fd: map it into CPU address space. `virAddr`
        // is a GPU-side address (often null) and must not be used for a CPU blit.
        let addr = unsafe {
            mmap(
                std::ptr::null_mut(),
                cap,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                hnd.fd,
                0,
            )
        };
        if addr.is_null() || addr as isize == MAP_FAILED {
            return false;
        }
        let dst = unsafe { std::slice::from_raw_parts_mut(addr.cast::<u8>(), cap) };
        let ok = copy_rgba_with_stride(frame, dst, w, h, stride);
        unsafe { munmap(addr, cap) };
        let region = Region {
            rects: std::ptr::null_mut(),
            rect_number: 0,
        };
        let rc = unsafe { OH_NativeWindow_NativeWindowFlushBuffer(window, buffer, fence, region) };
        ok && rc == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn c_abi_rejects_a_null_frame_and_unbound_surface() {
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
