//! The `nativeview` plugin — a capability that is a **native view**.
//!
//! The capability is a view the framework does not paint: an `NSView` the
//! Objective-C half creates and puts into the host's window. This crate is the
//! Rust half of it.
//!
//! # Two contracts
//!
//! | | asks | shape |
//! |---|---|---|
//! | the plugin's own ABI (`native/common/floom_nativeview.h`) | is the capability available? | four functions; the plugin is a singleton |
//! | the provider ABI (`floom-native-ffi`) | how is the native view created, placed and torn down? | a versioned struct of function pointers; the provider is an instance per window |
//!
//! A plugin whose capability is a native view implements both.
//!
//! # Which side owns what
//!
//! The plugin owns the *capability*; the host owns the *transport*. The
//! Objective-C half reaches the host's window through the provider's
//! `attach`, and the framework drives the provider through the ordinary Rust
//! [`NativeViewProvider`] trait — so the controller's lifecycle invariants
//! apply to this plugin exactly as they apply to a Rust one.
//!
//! # How the two halves find each other
//!
//! The native half hands its vtable to this one from a constructor, and this
//! crate hands the provider to the host on request. Nothing is looked up by
//! name at runtime: the symbols resolve when the crate is linked.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::OnceLock;

use floom_native_ffi::abi::{FloomNativeProviderV1, FloomNativeSlot};
use floom_native_ffi::floom_native::NativeViewProvider;
use floom_native_ffi::{CProvider, CProviderError};

// The native half's own four functions.
unsafe extern "C" {
    fn floom_nativeview_version() -> *const c_char;
    fn floom_nativeview_enable() -> c_int;
    fn floom_nativeview_disable();
    fn floom_nativeview_capability() -> c_int;
    fn floom_nativeview_load_url(
        slot: FloomNativeSlot,
        url: *const c_char,
        out_error: *mut c_char,
        out_capacity: u32,
    ) -> c_int;
}

/// The provider vtable, once the native half has handed it over.
static PROVIDER: OnceLock<usize> = OnceLock::new();

/// Receives the provider vtable from the native half.
///
/// Called from the native half's constructor, so it runs before `main` and
/// before any app asks for the provider. The pointer is stored rather than
/// used: the native half owns the vtable, and it is still there when the
/// app asks.
#[unsafe(no_mangle)]
pub extern "C" fn floom_nativeview_register_provider(provider: *const FloomNativeProviderV1) {
    if provider.is_null() {
        return;
    }
    let _ = PROVIDER.set(provider as usize);
}

/// The plugin's version, as the native half reports it.
pub fn version() -> String {
    // SAFETY: the native half returns a NUL-terminated string that stays
    // valid until the next call into this plugin.
    let ptr = unsafe { floom_nativeview_version() };
    if ptr.is_null() {
        return env!("CARGO_PKG_VERSION").to_string();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Claims the plugin's platform state.
///
/// Called once by the host, through [`floom_platform::register_plugin`].
pub fn enable() -> Result<(), String> {
    // SAFETY: called on the host thread during startup, before any frame.
    let rc = unsafe { floom_nativeview_enable() };
    if rc != 0 {
        return Err(format!("the native half refused to enable with code {rc}"));
    }
    Ok(())
}

/// Releases whatever [`enable`] claimed.
pub fn disable() {
    // SAFETY: the native half tolerates a disable without a matching enable.
    unsafe { floom_nativeview_disable() };
}

/// The plugin's capability id.
pub fn capability() -> i32 {
    // SAFETY: the native half returns a plain integer.
    unsafe { floom_nativeview_capability() }
}

/// Loads `url` into the web participant bound to `slot`.
///
/// A separate entry point from the provider ABI because the two answer
/// different questions: the provider places a composited rectangle, and *what
/// a web view shows* is not part of that contract. The host calls this after
/// binding a [`NativeViewKind::Web`] participant.
///
/// `Ok` claims WebKit accepted the request — the load *started*. Whether the
/// page arrives is the page's own business and arrives asynchronously; a
/// caller that needs the load to finish waits for WebKit's own delegate, which
/// this ABI deliberately does not wrap.
pub fn load_url(slot: floom_native_ffi::floom_native::NativeSlot, url: &str) -> Result<(), String> {
    let Ok(c_url) = std::ffi::CString::new(url) else {
        return Err(format!("url contains a NUL byte: {url:?}"));
    };
    // The native half writes a NUL-terminated message on refusal; 256 bytes is
    // enough for every reason it can produce and keeps this off the allocator.
    let mut error = [0i8; 256];
    // SAFETY: `c_url` is NUL-terminated and outlives the call; `error` is a
    // live buffer of the length passed; `ffi_slot` is a `#[repr(C)]` copy of
    // the slot, which is what crosses the boundary (see [`ffi_slot`]).
    let rc = unsafe {
        floom_nativeview_load_url(
            ffi_slot(slot),
            c_url.as_ptr(),
            error.as_mut_ptr(),
            error.len() as u32,
        )
    };
    if rc == 0 {
        return Ok(());
    }
    // SAFETY: the native half NUL-terminates whatever it writes.
    let message = unsafe { CStr::from_ptr(error.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    Err(if message.is_empty() {
        format!("the native half refused to load the url with code {rc}")
    } else {
        message
    })
}

/// The slot, in the layout the C ABI declares.
///
/// `NativeSlot` is a Rust struct with no `#[repr(C)]`, so passing it straight
/// to a `extern "C"` function is undefined behaviour — the layout is
/// unspecified and the callee reads a different pair of integers than the
/// caller wrote. This conversion is the boundary, and it is the same one
/// `floom-native-ffi` performs for the vtable's own arguments.
fn ffi_slot(slot: floom_native_ffi::floom_native::NativeSlot) -> FloomNativeSlot {
    FloomNativeSlot {
        index: slot.id.index(),
        generation: slot.generation,
    }
}

/// Registers this plugin with the process-wide table.
pub fn register() {
    floom_platform::register_plugin("nativeview", enable);
}

/// The provider for this plugin, if the native half has handed one over.
///
/// A host that wants a native view from this plugin asks for it here and
/// hands it to [`floom_app::WidgetHost::register_native_provider`] with the
/// kinds this plugin's views answer for.
///
/// `host_window` is the platform's opaque window handle: an `NSWindow *` on
/// macOS. It is the same handle the host would give its own provider, so
/// the plugin's view lands in the same window as the framework's surface.
///
/// # Safety
///
/// `host_window` must be a live window handle of the kind the native half
/// expects — an `NSWindow *` on macOS — and must stay valid for as long as the
/// returned provider is driven. Nothing here can check either claim: the handle
/// arrives as a bare pointer from the host, and the type is opaque by design.
pub unsafe fn provider(host_window: *const c_void) -> Result<CProvider, CProviderError> {
    // Stored as a `usize` rather than a pointer: a raw pointer is neither
    // `Send` nor `Sync`, and a `static` needs both. The pointer came from the
    // native half and stays valid for the life of the process, so the round
    // trip through an integer loses nothing.
    let Some(raw) = PROVIDER.get().copied() else {
        return Err(CProviderError::MissingEntry(
            "the native half registered no provider",
        ));
    };
    let vtable = raw as *const FloomNativeProviderV1;
    // SAFETY: the vtable is the native half's, and the native half keeps it
    // for the life of the process.
    let provider = unsafe { CProvider::from_raw(vtable) };
    // SAFETY: the handle outlives the provider by the caller's contract.
    unsafe { provider.with_host(host_window) }
}

/// The diagnostics the native half publishes.
///
/// Plain numbers, so a host can check what the native side did without
/// knowing anything about AppKit. The counts are cumulative since the
/// process started.
pub mod diagnostics {
    use floom_native_ffi::abi::FloomNativeSlot;

    unsafe extern "C" {
        fn floom_nativeview_created_count() -> i32;
        fn floom_nativeview_destroyed_count() -> i32;
        fn floom_nativeview_applied_count() -> i32;
        fn floom_nativeview_focused_count() -> i32;
        fn floom_nativeview_refused_count() -> i32;
        fn floom_nativeview_live_count() -> i32;
        fn floom_nativeview_slot_is_web(slot: FloomNativeSlot) -> i32;
        fn floom_nativeview_slot_is_attached(slot: FloomNativeSlot) -> i32;
        fn floom_nativeview_last_frame(x: *mut f64, y: *mut f64, w: *mut f64, h: *mut f64);
    }

    /// How many participants the native half created.
    pub fn created() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_nativeview_created_count() }
    }

    /// How many `create` calls were refused for a kind this plugin cannot
    /// build.
    ///
    /// The number that stops a refusal from being invisible: this plugin draws
    /// web views and refuses everything else, so a non-zero count on a host
    /// that asked for a camera is the true reason nothing appeared.
    pub fn refused() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_nativeview_refused_count() }
    }

    /// How many participants are alive right now.
    pub fn live() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_nativeview_live_count() }
    }

    /// Whether the participant in `slot` holds a real `WKWebView`.
    pub fn slot_is_web(slot: floom_native_ffi::floom_native::NativeSlot) -> bool {
        // SAFETY: the native half returns a plain integer, and `ffi_slot`
        // crosses the boundary in the ABI's own layout.
        unsafe { floom_nativeview_slot_is_web(super::ffi_slot(slot)) != 0 }
    }

    /// How many participants the native half tore down.
    pub fn destroyed() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_nativeview_destroyed_count() }
    }

    /// How many placements the native half applied.
    pub fn applied() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_nativeview_applied_count() }
    }

    /// How many times the native half took focus.
    pub fn focused() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_nativeview_focused_count() }
    }

    /// The last frame the native half placed, in points.
    pub fn last_frame() -> (f64, f64, f64, f64) {
        let (mut x, mut y, mut w, mut h) = (0.0, 0.0, 0.0, 0.0);
        // SAFETY: the four pointers are valid and the native half writes one
        // double through each.
        unsafe {
            floom_nativeview_last_frame(&mut x, &mut y, &mut w, &mut h);
        }
        (x, y, w, h)
    }

    /// Whether the native view is really in the host window's content view.
    ///
    /// The check that matters: a provider that created a view and did not
    /// put it anywhere has not contributed a native view, and the difference
    /// is invisible from the framework's side.
    ///
    /// Per slot, because a scene can hold several participants at once and
    /// "some view is attached" says nothing about the one being asked about.
    /// There is deliberately no whole-table version: the old one answered for
    /// whichever slot happened to be created last, which was a true statement
    /// about a view nobody had asked about.
    pub fn slot_is_attached(slot: floom_native_ffi::floom_native::NativeSlot) -> bool {
        // SAFETY: the native half returns a plain integer, and `ffi_slot`
        // crosses the boundary in the ABI's own layout.
        unsafe { floom_nativeview_slot_is_attached(super::ffi_slot(slot)) != 0 }
    }
}

/// The provider, boxed, for a host that wants the trait object.
///
/// # Safety
///
/// Same contract as [`provider`]: `host_window` must be a live handle of the
/// kind the native half expects, and must outlive the returned provider.
pub unsafe fn boxed_provider(
    host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    // SAFETY: forwarded to `provider`, whose contract this function repeats.
    Ok(Box::new(unsafe { provider(host_window) }?))
}
