#![cfg(target_os = "ios")]

//! Receiving files/URLs shared into the app from another iOS app.
//!
//! When the user shares a file (or opens a file / URL with Floom via the iOS
//! share sheet / "Open In"), the system delivers it to the app through the
//! `UIApplicationDelegate` methods `application:didFinishLaunchingWithOptions:`
//! (cold-start — the app is launched *via* the share sheet) and
//! `application:openURL:options:` (warm-start — the app is already running).
//!
//! winit's fork ([`floem-winit`]) owns the `UIApplicationDelegate` (class
//! `WinitApplicationDelegate`). It used to ignore both share paths, so floom
//! patches the fork to forward the shared URL through a C callback registered
//! via `winit::set_on_open_url` (see floom issue #109). This module registers
//! that callback (`install_openurl_hook`, called from `Application::new_ios`)
//! and converts the forwarded `NSURL` into a [`SharedFile`].
//!
//! Because the callback is registered **before** `EventLoop::run()` (and thus
//! before `UIApplicationMain`), both cold- and warm-start shares are caught —
//! there is no longer a `class_addMethod`/`dispatch_async_f` injection race, and
//! no cold-start gap.

use std::cell::RefCell;
use std::ffi::c_void;
use std::path::Path;

use objc2_foundation::NSURL;

use crate::platform::SharedFile;

type SharedFilesHandler = Box<dyn Fn(Vec<SharedFile>) + 'static>;

// iOS delivers shared files on the main thread only, so the handler is not
// required to be `Send`. A `thread_local` avoids the `Sync` bound a `static`
// would impose on a `!Send` handler (Floom's UI signals such as `RwSignal` are
// `!Send`), while still giving a single global registration point per thread.
thread_local! {
    static HANDLER: RefCell<Option<SharedFilesHandler>> = RefCell::new(None);
    // Files that arrived before a handler was registered; flushed on registration.
    static PENDING: RefCell<Vec<Vec<SharedFile>>> = RefCell::new(Vec::new());
}

/// Register the handler invoked when files/URLs are shared into the app.
///
/// If a share arrived before this call (rare on iOS — only a warm-start share
/// that raced registration), the queued items are delivered to `handler`
/// immediately. Subsequent shares invoke `handler` directly.
pub fn set_shared_files_handler(handler: SharedFilesHandler) {
    let queued: Vec<SharedFile> = PENDING.with(|pending| {
        let mut pending = pending.borrow_mut();
        pending.drain(..).flatten().collect()
    });
    HANDLER.with(|h| *h.borrow_mut() = Some(handler));
    if !queued.is_empty() {
        invoke_handler(&queued);
    }
}

/// Deliver shared files to the registered handler, or queue them if none is set.
fn on_shared_files(files: Vec<SharedFile>) {
    if files.is_empty() {
        return;
    }
    let has_handler = HANDLER.with(|h| h.borrow().is_some());
    if has_handler {
        invoke_handler(&files);
    } else {
        PENDING.with(|pending| pending.borrow_mut().push(files));
    }
}

/// Invoke the handler. The handler must not re-enter the shared-file APIs (it
/// would deadlock the `HANDLER` borrow).
fn invoke_handler(files: &[SharedFile]) {
    HANDLER.with(|h| {
        if let Some(handler) = h.borrow().as_ref() {
            handler(files.to_vec());
        }
    });
}

/// Install the shared-files hook by registering floom's callback with winit's
/// fork (see `winit::set_on_open_url`). Safe to call multiple times; registers
/// once.
///
/// Must be called on the main thread (iOS UI requirement) and **before**
/// `EventLoop::run()` so the callback is in place before `UIApplicationMain`
/// delivers a cold-start share. winit forwards every shared `NSURL` (cold- and
/// warm-start) to [`open_url_trampoline`], which converts it to a [`SharedFile`]
/// and delivers it to the registered handler.
pub fn install_openurl_hook() {
    unsafe {
        winit::set_on_open_url(Some(open_url_trampoline));
    }
}

/// Trampoline registered with winit's `set_on_open_url`. winit passes the raw
/// `NSURL` pointer; we convert it to a [`SharedFile`] and forward it. The
/// `NSURL` is valid for the duration of this synchronous call (iOS owns it for
/// the selector's scope), so we read it without retaining.
unsafe extern "C" fn open_url_trampoline(url_ptr: *mut c_void) {
    if !url_ptr.is_null() {
        let nsurl = unsafe { &*(url_ptr as *const NSURL) };
        let shared = nsurl_to_shared(nsurl);
        on_shared_files(vec![shared]);
    }
}

/// Convert an incoming `NSURL` into a [`SharedFile`].
///
/// - File URLs (`file://…`, e.g. "Open In" / share of a real file) → the
///   decoded filesystem path as `uri`, with the file name as `display_name`.
/// - Other URLs (`http(s)://`, custom schemes, universal links) → the absolute
///   URL string as `uri` (iOS does not surface a MIME type here, so it is `None`).
fn nsurl_to_shared(nsurl: &NSURL) -> SharedFile {
    if nsurl.isFileURL() {
        if let Some(path) = nsurl.to_file_path() {
            let uri = path.to_string_lossy().into_owned();
            let display_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            return SharedFile {
                uri,
                mime_type: None,
                display_name,
            };
        }
    }
    // Fallback / non-file URL: keep the absolute URL string.
    let uri = nsurl
        .absoluteString()
        .map(|s| s.to_string())
        .unwrap_or_default();
    SharedFile {
        uri,
        mime_type: None,
        display_name: None,
    }
}
