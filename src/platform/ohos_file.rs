//! OpenHarmony file dialogs via the `@ohos.file.picker` Document/Folder pickers.
//!
//! OpenHarmony has no native `rfd` equivalent. Opening or saving a file means
//! launching a system `DocumentViewPicker` / `DocumentSavePicker` /
//! `FolderPicker` from ArkTS, which resolves asynchronously with the chosen
//! file URI(s). This mirrors Android's SAF bridge (`android_file.rs`):
//!
//! - Rust fires the picker through the shared N-API bridge
//!   ([`crate::platform::ohos_bridge`]) and stores the pending callback keyed by
//!   a request id.
//! - When the picker resolves, the ArkTS side calls the exported N-API function
//!   [`on_file_picked`] with the request id and the chosen URI(s); we build a
//!   [`FileInfo`] and invoke the callback (deferred to Floem's event loop via an
//!   ext action, as on Android).
//!
//! **Note on paths:** the picker returns `file://` URIs or sandbox paths, not
//! plain filesystem paths in all cases. Each returned URI string is used as a
//! `FileInfo.path` entry; apps should open it through the appropriate ArkTS /
//! `fileio` API rather than treating it as a raw path.
//!
//! # Status
//! Compiled only for `target_env = "ohos"`. **UNVERIFIED** against a real OHOS
//! SDK build (see `docs/ohos.md`).

#![cfg(target_env = "ohos")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use napi::bindgen_prelude::JsFunction;
use napi::JsObject;
use napi_derive::napi;
use napi::{Env, Result as NapiResult};
use parking_lot::Mutex;

use floem_legacy_reactive::Scope;

use crate::ext_event::create_ext_action;
use crate::platform::file::{FileDialogOptions, FileInfo};
use crate::platform::ohos_bridge;

static REQUEST_SEQ: AtomicU32 = AtomicU32::new(1);
static PENDING: Mutex<Option<PendingRequest>> = Mutex::new(None);

struct PendingRequest {
    request_id: u32,
    callback: Box<dyn FnOnce(Option<FileInfo>) + Send + 'static>,
}

enum PickerMode {
    Open,
    OpenTree,
    Create,
}

/// Open a file (or directory) using the system document/folder picker.
pub fn open_file(
    options: FileDialogOptions,
    file_info_action: impl Fn(Option<FileInfo>) + Send + 'static,
) {
    let mode = if options.select_directories {
        PickerMode::OpenTree
    } else {
        PickerMode::Open
    };
    launch(mode, options, file_info_action);
}

/// Open a system file save dialog (document-save picker).
pub fn save_as(
    options: FileDialogOptions,
    file_info_action: impl Fn(Option<FileInfo>) + Send + 'static,
) {
    launch(PickerMode::Create, options, file_info_action);
}

fn launch(
    mode: PickerMode,
    options: FileDialogOptions,
    file_info_action: impl Fn(Option<FileInfo>) + Send + 'static,
) {
    // Defer the user callback to Floem's event loop (the picker result may
    // arrive on the ArkTS thread, not Floem's).
    let send = create_ext_action(Scope::new(), move |info: Option<FileInfo>| {
        file_info_action(info);
    });
    let cb: Box<dyn FnOnce(Option<FileInfo>) + Send + 'static> =
        Box::new(move |info| send(info));

    let request_id = REQUEST_SEQ.fetch_add(1, Ordering::SeqCst);
    let method = match mode {
        PickerMode::Open | PickerMode::OpenTree => "openFile",
        PickerMode::Create => "saveFile",
    };

    let launched = ohos_bridge::call_bridge(|env, obj| {
        let func: JsFunction = obj.get_named_property(method)?;
        let opts_obj = build_options_object(&env, &options)?;
        // (requestId, options) — ArkTS echoes requestId back in on_file_picked.
        // `u32` is not `NapiRaw`, so wrap it as a `JsNumber` first; both args go
        // through `into_unknown()` to satisfy `call`'s homogeneous `&[V]` slice
        // (`V: NapiRaw`).
        let req = env.create_uint32(request_id)?;
        func.call(None, &[req.into_unknown(), opts_obj.into_unknown()])?;
        Ok(())
    });

    match launched {
        Some(()) => {
            *PENDING.lock() = Some(PendingRequest {
                request_id,
                callback: cb,
            });
        }
        None => {
            // Bridge unavailable or launch failed: resolve with None rather
            // than leaking the pending callback.
            cb(None);
        }
    }
}

/// Build the ArkTS-side options object from a [`FileDialogOptions`].
fn build_options_object(env: &Env, o: &FileDialogOptions) -> NapiResult<JsObject> {
    let mut obj = env.create_object()?;
    if let Some(t) = &o.title {
        obj.set_named_property("title", env.create_string(t)?)?;
    }
    if let Some(n) = &o.default_name {
        obj.set_named_property("defaultName", env.create_string(n)?)?;
    }
    obj.set_named_property("selectDirectories", env.get_boolean(o.select_directories)?)?;
    obj.set_named_property("multiSelection", env.get_boolean(o.multi_selection)?)?;
    if let Some(dir) = &o.starting_directory {
        obj.set_named_property("startingDirectory", env.create_string(&dir.to_string_lossy())?)?;
    }
    // TODO(OHOS): map `allowed_types` (Vec<FileSpec>) to an OHOS picker filter
    // (MIME / extension list) — `@ohos.file.picker` has limited filtering.
    Ok(obj)
}

/// Forward a picker result to the pending file-dialog callback.
///
/// Called from ArkTS after the `DocumentViewPicker` / `DocumentSavePicker` /
/// `FolderPicker` resolves, with `(requestId, uris)`. `uris` is `null` when the
/// user cancels.
// TODO(OHOS): verify the `#[napi]` import path and module registration.
#[napi]
pub fn on_file_picked(request_id: u32, uris: Option<Vec<String>>) -> NapiResult<()> {
    let pending = PENDING.lock().take();
    let Some(pending) = pending else {
        return Ok(());
    };
    if pending.request_id != request_id {
        // Not our request; restore it for another handler.
        *PENDING.lock() = Some(pending);
        return Ok(());
    }

    let info = uris.map(|paths| FileInfo {
        path: paths.into_iter().map(PathBuf::from).collect(),
        format: None,
    });
    (pending.callback)(info);
    Ok(())
}
