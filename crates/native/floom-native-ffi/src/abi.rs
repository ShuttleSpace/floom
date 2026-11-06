// SPDX-License-Identifier: MIT
//! The Rust mirror of `include/floom_native_provider.h`.
//!
//! Every type here is `#[repr(C)]` and every field is in the same order as
//! the header. The header is the **source of truth** — it is what a native
//! implementation includes — and this file is the machine-checked mirror of
//! it: `tests/c_provider.rs` compiles a C program that reports each struct's
//! size and each field's offset, and fails if the two disagree.
//!
//! That test is the point. A hand-written ABI drifts: a field is reordered,
//! a type is widened, an enum is given a different discriminant, and the
//! symptom is a provider that writes into a struct the framework reads at
//! the wrong offsets — which is memory corruption, not a compile error, and
//! not something a Rust test can catch on its own.

use std::ffi::c_char;

use floom_native::{ClipSupport, DeviceRect, InputMode, InputModes, NativeCompositionMode};

/// The composition modes, as the header numbers them.
///
/// The values are the header's enumerators, in the same order. They are not
/// a strength order — see the header.
pub const MODE_OVERLAY: u32 = 0;
pub const MODE_TEXTURE: u32 = 1;
pub const MODE_SURFACE: u32 = 2;
pub const MODE_OFFSCREEN: u32 = 3;

/// The clip support values, as the header numbers them.
pub const CLIP_UNSUPPORTED: u32 = 0;
pub const CLIP_PLATFORM: u32 = 1;
pub const CLIP_PROVIDER: u32 = 2;

/// The input mode values, as the header numbers them.
pub const INPUT_PASSTHROUGH: u32 = 0;
pub const INPUT_CAPTURE: u32 = 1;

/// The view kind values, as the header numbers them.
pub const KIND_MAP: u32 = 0;
pub const KIND_VIDEO: u32 = 1;
pub const KIND_WEB: u32 = 2;
pub const KIND_TEXT_INPUT: u32 = 3;
pub const KIND_CUSTOM: u32 = 4;

/// The ABI version this Rust side speaks.
pub const ABI_VERSION: u32 = 1;

/// A participant slot, as the header defines it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloomNativeSlot {
    pub index: u32,
    pub generation: u32,
}

/// A rect in whole device pixels, as the header defines it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloomNativeDeviceRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// What one provider can do, as the header defines it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloomNativeCapabilities {
    /// The modes the provider supports, in its own order of preference.
    pub modes: [u32; 4],
    /// How many entries of `modes` are meaningful.
    pub mode_count: u8,
    pub snapshot: u8,
    pub input_passthrough: u8,
    pub input_capture: u8,
    pub focusable: u8,
    /// A `CLIP_*` value.
    pub clip: u8,
    pub reserved: [u8; 3],
}

/// The state to apply, as the header defines it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloomNativeState {
    pub rect: FloomNativeDeviceRect,
    pub clip: FloomNativeDeviceRect,
    pub has_clip: u8,
    pub visible: u8,
    pub reserved: [u8; 2],
    pub z: u32,
    /// An `INPUT_*` value.
    pub input: u32,
    /// A `MODE_*` value.
    pub mode: u32,
}

/// The provider's entry points, as the header defines it.
///
/// The function pointers are `Option`, so a missing entry is representable —
/// and a provider that leaves one out is refused at registration rather than
/// crashing on the first call.
#[repr(C)]
pub struct FloomNativeProviderV1 {
    pub abi_version: u32,
    /// The provider's own state pointer.
    pub self_: *mut core::ffi::c_void,
    /// The host's window, given once. Optional: a provider that hands the
    /// framework pixels rather than a view never needs one.
    pub attach:
        Option<unsafe extern "C" fn(*mut core::ffi::c_void, *const core::ffi::c_void) -> i32>,
    pub capabilities:
        Option<unsafe extern "C" fn(*mut core::ffi::c_void) -> FloomNativeCapabilities>,
    pub create: Option<
        unsafe extern "C" fn(
            *mut core::ffi::c_void,
            FloomNativeSlot,
            u32,
            *const c_char,
            *mut c_char,
            u32,
        ) -> i32,
    >,
    pub destroy: Option<unsafe extern "C" fn(*mut core::ffi::c_void, FloomNativeSlot)>,
    pub apply: Option<
        unsafe extern "C" fn(*mut core::ffi::c_void, FloomNativeSlot, *const FloomNativeState),
    >,
    pub focus: Option<unsafe extern "C" fn(*mut core::ffi::c_void, FloomNativeSlot)>,
    pub blur: Option<unsafe extern "C" fn(*mut core::ffi::c_void, FloomNativeSlot)>,
}

/// The number the header gives a composition mode.
pub fn mode_number(mode: NativeCompositionMode) -> u32 {
    match mode {
        NativeCompositionMode::Overlay => MODE_OVERLAY,
        NativeCompositionMode::Texture => MODE_TEXTURE,
        NativeCompositionMode::Surface => MODE_SURFACE,
        NativeCompositionMode::Offscreen => MODE_OFFSCREEN,
    }
}

/// The composition mode a number names, or `None` for a number the header
/// does not define.
pub fn mode_from_number(number: u32) -> Option<NativeCompositionMode> {
    Some(match number {
        MODE_OVERLAY => NativeCompositionMode::Overlay,
        MODE_TEXTURE => NativeCompositionMode::Texture,
        MODE_SURFACE => NativeCompositionMode::Surface,
        MODE_OFFSCREEN => NativeCompositionMode::Offscreen,
        _ => return None,
    })
}

/// The clip support a number names.
pub fn clip_from_number(number: u32) -> ClipSupport {
    match number {
        CLIP_PLATFORM => ClipSupport::Platform,
        CLIP_PROVIDER => ClipSupport::Provider,
        _ => ClipSupport::Unsupported,
    }
}

/// The input mode a number names.
pub fn input_from_number(number: u32) -> InputMode {
    match number {
        INPUT_CAPTURE => InputMode::Capture,
        _ => InputMode::Passthrough,
    }
}

/// The input modes a capabilities block describes.
pub fn input_modes(capabilities: &FloomNativeCapabilities) -> InputModes {
    InputModes {
        passthrough: capabilities.input_passthrough != 0,
        capture: capabilities.input_capture != 0,
    }
}

/// A device rect, from the framework's own.
pub fn device_rect(rect: DeviceRect) -> FloomNativeDeviceRect {
    FloomNativeDeviceRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

/// Read a NUL-terminated message out of a buffer the native side filled.
///
/// The buffer is not required to be NUL-terminated: the length is taken as
/// the capacity, and the string is cut at the first NUL inside it. A native
/// side that fills the buffer without terminating it produces a truncated
/// message rather than reading past the end.
pub(crate) fn read_message(buffer: &[c_char]) -> String {
    let bytes: Vec<u8> = buffer.iter().map(|c| *c as u8).collect();
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
