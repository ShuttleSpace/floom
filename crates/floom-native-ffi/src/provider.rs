//! A C provider, presented to the framework as a Rust [`NativeViewProvider`].
//!
//! This is the thin adapter the whole ABI exists for: the native side
//! implements a [`FloomNativeProviderV1`], and the framework drives it
//! through the ordinary Rust trait, so the controller's lifecycle
//! invariants — a `destroy` for every `create`, a recycled slot re-bound
//! before it is applied to, no negative extent — apply to a Swift or Java
//! provider exactly as they apply to a Rust one.

use std::borrow::Cow;
use std::cell::OnceCell;
use std::ffi::{CString, c_char, c_void};

use floom_native::{
    InputMode, NativeCapabilities, NativeCompositionMode, NativeError, NativeState, NativeViewKind,
    NativeViewProvider, to_device,
};

use crate::abi::{
    ABI_VERSION, FloomNativeDeviceRect, FloomNativeProviderV1, FloomNativeSlot, FloomNativeState,
    INPUT_CAPTURE, INPUT_PASSTHROUGH, KIND_CUSTOM, clip_from_number, device_rect, input_modes,
    mode_from_number, mode_number, read_message,
};

/// How large the error buffer handed to `create` is.
///
/// 256 bytes is enough for a sentence and short enough to sit on the stack.
/// A message longer than this is truncated, which is why the ABI says the
/// buffer is "truncated to capacity" rather than "fits".
const ERROR_BUFFER: usize = 256;

/// A provider implemented in C.
///
/// Owns nothing: the vtable and the `self` pointer it points at belong to
/// the native side, which must keep them alive for as long as this value
/// exists.
pub struct CProvider {
    vtable: *const FloomNativeProviderV1,
    /// The modes the native provider published, kept so the framework does not
    /// ask again on every frame.
    ///
    /// A provider's capabilities do not change over its life, and the
    /// controller asks for them on every `apply`. Without this the C adapter
    /// would allocate a list per frame, which is the one thing the borrowed
    /// form of the Rust side exists to avoid.
    modes: OnceCell<Cow<'static, [NativeCompositionMode]>>,
}

impl std::fmt::Debug for CProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CProvider")
            .field("vtable", &self.vtable)
            .finish_non_exhaustive()
    }
}

impl CProvider {
    /// Wrap a native provider.
    ///
    /// # Safety
    ///
    /// `vtable` must point at a `FloomNativeProviderV1` that stays valid, and
    /// whose function pointers stay valid, for as long as the returned value
    /// lives. The `self` pointer inside it must be valid for the same period.
    pub unsafe fn from_raw(vtable: *const FloomNativeProviderV1) -> Self {
        Self {
            vtable,
            modes: OnceCell::new(),
        }
    }

    /// The native provider's own pointer, for handing back to the native side.
    pub fn as_raw(&self) -> *const FloomNativeProviderV1 {
        self.vtable
    }

    /// Tell the provider which window it is compositing into.
    ///
    /// A provider is an instance per window, so the window does not change
    /// over its life and is given once. `host_window` is the platform's
    /// opaque handle: an `NSView *` on macOS, an `ANativeWindow *` on
    /// Android, a `Window` on X11.
    ///
    /// A provider with no `attach` entry point is left alone: one that hands
    /// the framework pixels rather than a view never needs a window.
    ///
    /// # Safety
    ///
    /// `host_window` must be a handle the platform keeps valid for as long
    /// as the provider lives, or null.
    pub unsafe fn with_host(self, host_window: *const c_void) -> Result<Self, CProviderError> {
        let Some(attach) = self.table().attach else {
            return Ok(self);
        };
        // SAFETY: the entry point is present in the vtable, and the handle
        // outlives the provider by the caller's contract.
        let rc = unsafe { attach(self.self_ptr(), host_window) };
        if rc != 0 {
            return Err(CProviderError::AttachRefused(rc));
        }
        Ok(self)
    }

    /// The vtable, as a reference.
    ///
    /// # Safety
    ///
    /// The pointer must be valid; [`CProvider::from_raw`] is the only way to
    /// make one, and it demands exactly that.
    fn table(&self) -> &FloomNativeProviderV1 {
        // SAFETY: the constructor's contract is that the pointer is valid and
        // outlives this value.
        unsafe { &*self.vtable }
    }

    /// The provider's own state pointer.
    fn self_ptr(&self) -> *mut c_void {
        self.table().self_
    }

    /// The slot, as the ABI names it.
    fn slot(slot: floom_native::NativeSlot) -> FloomNativeSlot {
        FloomNativeSlot {
            index: slot.id.index(),
            generation: slot.generation,
        }
    }
}

/// Why a C provider could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CProviderError {
    /// The provider's ABI version is not one this side speaks.
    ///
    /// The two sides exchange pointers and integers whose meaning is defined
    /// by that version, so a mismatch means the definitions do not line up.
    AbiMismatch {
        /// What the provider declared.
        provider: u32,
        /// What this side speaks.
        expected: u32,
    },
    /// The vtable is missing an entry point.
    ///
    /// Named rather than numbered, because "the provider has no `apply`" is
    /// a thing a person can fix and "entry 3 is null" is not.
    MissingEntry(&'static str),
    /// The provider refused the host's window.
    ///
    /// Carries the code the provider returned, because the provider is the
    /// only one that knows what the code means.
    AttachRefused(i32),
}

impl std::fmt::Display for CProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AbiMismatch { provider, expected } => write!(
                f,
                "the native provider speaks ABI version {provider}, this side speaks {expected}"
            ),
            Self::MissingEntry(name) => {
                write!(f, "the native provider has no `{name}` entry point")
            }
            Self::AttachRefused(code) => write!(
                f,
                "the native provider refused the host's window with code {code}"
            ),
        }
    }
}

impl std::error::Error for CProviderError {}

/// Check that a native provider can be used at all.
///
/// The version and the entry points are checked **before** the provider is
/// driven, so a mismatch is a refusal at registration rather than a crash
/// on the first call.
///
/// # Safety
///
/// `vtable` must either be null or point to a `FloomNativeProviderV1` that
/// stays valid and readable for the duration of this call. The provider
/// publishes the table, so only its own loader can promise this; nothing here
/// can tell a stale pointer from a live one.
pub unsafe fn check(vtable: *const FloomNativeProviderV1) -> Result<(), CProviderError> {
    if vtable.is_null() {
        return Err(CProviderError::MissingEntry("the vtable itself"));
    }
    // SAFETY: the caller promises a valid vtable; `check` is the gate that
    // every other entry point goes through, and it reads the fields before
    // any of them are called.
    let table = unsafe { &*vtable };
    if table.abi_version != ABI_VERSION {
        return Err(CProviderError::AbiMismatch {
            provider: table.abi_version,
            expected: ABI_VERSION,
        });
    }
    if table.capabilities.is_none() {
        return Err(CProviderError::MissingEntry("capabilities"));
    }
    if table.create.is_none() {
        return Err(CProviderError::MissingEntry("create"));
    }
    if table.destroy.is_none() {
        return Err(CProviderError::MissingEntry("destroy"));
    }
    if table.apply.is_none() {
        return Err(CProviderError::MissingEntry("apply"));
    }
    Ok(())
}

impl NativeViewProvider for CProvider {
    fn capabilities(&self) -> NativeCapabilities {
        let modes = self.modes.get_or_init(|| {
            // SAFETY: `check` guarantees the entry point is present, and the
            // provider's own state pointer is valid for as long as this value.
            let raw = unsafe {
                let table = self.table();
                (table.capabilities.expect("checked at registration"))(table.self_)
            };
            // The native side publishes its modes in its own order of
            // preference, and the order is part of what it said: the first
            // mode is its own choice, which `preferred` reports.
            let mut modes: Vec<NativeCompositionMode> = Vec::new();
            for number in raw.modes.iter().take(raw.mode_count as usize) {
                if let Some(mode) = mode_from_number(*number) {
                    modes.push(mode);
                }
            }
            Cow::Owned(modes)
        });
        NativeCapabilities {
            modes: modes.clone(),
            snapshot: self.raw_capabilities().snapshot != 0,
            input: input_modes(&self.raw_capabilities()),
            focusable: self.raw_capabilities().focusable != 0,
            clip: clip_from_number(self.raw_capabilities().clip as u32),
        }
    }

    fn create(
        &mut self,
        slot: floom_native::NativeSlot,
        kind: NativeViewKind,
    ) -> Result<(), NativeError> {
        let (kind_number, kind_name) = match kind {
            NativeViewKind::Map => (crate::abi::KIND_MAP, None),
            NativeViewKind::Video => (crate::abi::KIND_VIDEO, None),
            NativeViewKind::Web => (crate::abi::KIND_WEB, None),
            NativeViewKind::TextInput => (crate::abi::KIND_TEXT_INPUT, None),
            NativeViewKind::Custom(name) => (KIND_CUSTOM, Some(name.to_string())),
        };

        // A NUL inside the name would truncate it, so it is removed rather
        // than passed through to be cut short at the wrong place.
        let name_c = kind_name
            .as_ref()
            .map(|name| CString::new(name.replace('\0', "")).unwrap_or_default());
        let name_ptr = name_c
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());

        let mut buffer = [0 as c_char; ERROR_BUFFER];
        // SAFETY: `check` guarantees the entry point is present; the slot and
        // the name pointer are valid for the duration of the call; the buffer
        // is `ERROR_BUFFER` bytes and is passed with its length.
        let rc = unsafe {
            let table = self.table();
            (table.create.expect("checked at registration"))(
                table.self_,
                Self::slot(slot),
                kind_number,
                name_ptr,
                buffer.as_mut_ptr(),
                ERROR_BUFFER as u32,
            )
        };
        if rc != 0 {
            return Err(NativeError::Provider(read_message(&buffer).into()));
        }
        Ok(())
    }

    fn destroy(&mut self, slot: floom_native::NativeSlot) {
        // SAFETY: `check` guarantees the entry point is present, and the
        // provider's state is valid for as long as this value.
        unsafe {
            let table = self.table();
            (table.destroy.expect("checked at registration"))(table.self_, Self::slot(slot));
        }
    }

    fn apply(&mut self, slot: floom_native::NativeSlot, state: NativeState) {
        let device = device_rect(state.device_rect());
        let clip = state
            .clip
            .map(|clip| device_rect(to_device(clip, state.scale)))
            .unwrap_or(FloomNativeDeviceRect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            });
        let native_state = FloomNativeState {
            rect: device,
            clip,
            has_clip: u8::from(state.clip.is_some()),
            visible: u8::from(state.visible),
            reserved: [0; 2],
            z: state.z,
            input: match state.input {
                InputMode::Passthrough => INPUT_PASSTHROUGH,
                InputMode::Capture => INPUT_CAPTURE,
            },
            mode: mode_number(state.mode),
        };
        // SAFETY: `check` guarantees the entry point is present; the state
        // pointer is valid for the duration of the call.
        unsafe {
            let table = self.table();
            (table.apply.expect("checked at registration"))(
                table.self_,
                Self::slot(slot),
                &native_state,
            );
        }
    }

    fn focus(&mut self, slot: floom_native::NativeSlot) {
        let Some(focus) = self.table().focus else {
            return;
        };
        // SAFETY: the entry point is present when the provider declares
        // itself focusable, and the controller refuses to focus one that does
        // not.
        unsafe {
            focus(self.self_ptr(), Self::slot(slot));
        }
    }

    fn blur(&mut self, slot: floom_native::NativeSlot) {
        let Some(blur) = self.table().blur else {
            return;
        };
        // SAFETY: as `focus`.
        unsafe {
            blur(self.self_ptr(), Self::slot(slot));
        }
    }

    fn snapshot(
        &mut self,
        _slot: floom_native::NativeSlot,
    ) -> Option<floom_native::NativeSnapshot> {
        // The C ABI has no snapshot entry point in v1: freezing a participant
        // into a texture needs the host's own capture path, and a provider
        // whose host cannot capture is the common and honest case. A C
        // provider therefore declares `snapshot: false` and is never asked.
        None
    }

    fn restore(&mut self, _slot: floom_native::NativeSlot) {
        // Nothing was frozen, so there is nothing to restore. The controller
        // only calls this for a slot it froze, and `snapshot` above never
        // freezes one.
    }
}

impl CProvider {
    /// The native provider's capabilities, as the ABI reports them.
    fn raw_capabilities(&self) -> crate::abi::FloomNativeCapabilities {
        // SAFETY: `check` guarantees the entry point is present, and the
        // provider's own state pointer is valid for as long as this value.
        unsafe {
            let table = self.table();
            (table.capabilities.expect("checked at registration"))(table.self_)
        }
    }
}
