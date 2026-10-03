//! Receiving files shared into the app from another OpenHarmony app.
//!
//! When the user shares a file (or opens a file with Floom via the share sheet /
//! "open with"), the system delivers it to the app's `Ability` through the
//! `Want`:
//!
//! - **Cold start** — `Ability.onCreate(want)`: the app was launched *by* the
//!   share.
//! - **Warm start** — `Ability.onNewWant(want)`: the app was already running
//!   and the user shared again.
//!
//! The `Want` carries the shared URIs in its parameters (e.g.
//! `ability.params.stream` / `ability.params.content`, or a single `uri`). The
//! ArkTS ability extracts them and calls the exported N-API function
//! [`on_shared_files`], which surfaces them to the app through a handler
//! registered with [`set_shared_files_handler`]. If the handler is registered
//! after a cold-start share has already been parsed, the queued files are
//! flushed immediately (same as Android / iOS).
//!
//! # Status
//! Compiled only for `target_env = "ohos"`. **UNVERIFIED** against a real OHOS
//! SDK build (see `docs/ohos.md`).

#![cfg(target_env = "ohos")]

use std::cell::RefCell;

use napi::Result as NapiResult;
use napi_derive::napi;

use crate::platform::SharedFile;

type SharedFilesHandler = Box<dyn Fn(Vec<SharedFile>) + 'static>;

// OpenHarmony delivers shared files on the main (Ability) thread, so the handler
// is not required to be `Send`. A `thread_local` avoids the `Sync` bound a
// `static Mutex` would impose on a `!Send` handler (Floom's UI signals such as
// `RwSignal` are `!Send`), while still giving a single global registration
// point per thread.
thread_local! {
    static HANDLER: RefCell<Option<SharedFilesHandler>> = RefCell::new(None);
    // Files that arrived before a handler was registered; flushed on registration.
    static PENDING: RefCell<Vec<Vec<SharedFile>>> = RefCell::new(Vec::new());
}

/// Register the handler invoked when files are shared into the app.
///
/// If a share arrived before this call (the cold-start case), the queued files
/// are delivered to `handler` immediately. Subsequent shares invoke `handler`
/// directly.
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
///
/// Named `deliver_shared_files` to avoid colliding with the `#[napi]`-exported
/// `on_shared_files` FFI entry point below (both live in this module).
fn deliver_shared_files(files: Vec<SharedFile>) {
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

/// Forward shared files from the ArkTS `Ability` lifecycle into the handler.
///
/// Called by the ArkTS side from `onCreate(want)` / `onNewWant(want)` after it
/// has extracted the shared URIs from the `Want`. The three parallel vectors
/// are zipped positionally; `mime_types` / `display_names` may be shorter or
/// `null`, in which case the missing entries become `None`.
// TODO(OHOS): verify the `#[napi]` import path and module registration.
#[napi]
pub fn on_shared_files(
    uris: Vec<String>,
    mime_types: Option<Vec<Option<String>>>,
    display_names: Option<Vec<Option<String>>>,
) -> NapiResult<()> {
    let files: Vec<SharedFile> = uris
        .into_iter()
        .enumerate()
        .map(|(i, uri)| SharedFile {
            uri,
            mime_type: mime_types
                .as_ref()
                .and_then(|m| m.get(i).cloned().flatten()),
            display_name: display_names
                .as_ref()
                .and_then(|d| d.get(i).cloned().flatten()),
        })
        .collect();
    deliver_shared_files(files);
    Ok(())
}
