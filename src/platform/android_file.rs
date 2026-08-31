#![cfg(target_os = "android")]

//! Android file dialogs via the Storage Access Framework (SAF).
//!
//! Android has no native `rfd` equivalent. Opening or saving a file means
//! launching a system picker with an intent — `ACTION_OPEN_DOCUMENT`,
//! `ACTION_OPEN_DOCUMENT_TREE`, or `ACTION_CREATE_DOCUMENT` — via
//! `Activity.startActivityForResult`, then receiving the chosen `Uri` back in
//! `Activity.onActivityResult`.
//!
//! Because `onActivityResult` is a Java method on the `Activity`, the app must
//! forward it to Floem. The app's `android_main` host should subclass
//! `android.app.NativeActivity` (see `examples/android/FloemNativeActivity.java`)
//! and, from its `onActivityResult`, call the native method that invokes
//! [`dispatch_activity_result`]. Floem stores the pending callback keyed by
//! request code and invokes it when the result arrives — on the main looper
//! thread, which is also Floem's event-loop thread, so the reactive update is
//! safe.
//!
//! **Note on paths:** SAF returns `content://` URIs, not filesystem paths. The
//! selected `Uri` string is returned as the `FileInfo.path` entry; apps on
//! Android should detect the `content://` scheme and open the document via
//! `ContentResolver` rather than treating it as a plain file path.

use std::path::PathBuf;

use jni::{
    objects::{JObject, JString, JValue},
    sys::{jint, jobject},
};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use floem_reactive::Scope;

use crate::ext_event::create_ext_action;
use crate::platform::android_jni::AndroidJni;
use crate::platform::file::{FileDialogOptions, FileInfo};

static REQUEST_SEQ: AtomicU32 = AtomicU32::new(1);
static PENDING: Mutex<Option<PendingRequest>> = Mutex::new(None);

struct PendingRequest {
    request_code: u32,
    callback: Box<dyn FnOnce(Option<FileInfo>) + Send + 'static>,
}

enum PickerMode {
    Open,
    OpenTree,
    Create,
}

/// Open a file (or directory) using the system document picker (SAF).
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

/// Open a system file save dialog (SAF create-document).
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
    let send = create_ext_action(Scope::new(), move |info: Option<FileInfo>| {
        file_info_action(info);
    });
    let cb: Box<dyn FnOnce(Option<FileInfo>) + Send + 'static> =
        Box::new(move |info| send(info));

    let request_code = REQUEST_SEQ.fetch_add(1, Ordering::SeqCst) as i32;
    let launched = AndroidJni::with_env(|env, activity| {
        build_and_start(env, activity, mode, &options, request_code)
    });

    match launched {
        Some(Ok(())) => {
            *PENDING.lock() = Some(PendingRequest {
                request_code: request_code as u32,
                callback: cb,
            });
        }
        _ => {
            // JNI bridge uninitialized or the launch failed: resolve with None
            // instead of leaking the pending callback.
            cb(None);
        }
    }
}

fn build_and_start(
    env: &mut jni::JNIEnv,
    activity: &JObject<'static>,
    mode: PickerMode,
    options: &FileDialogOptions,
    request_code: i32,
) -> Result<(), String> {
    let action_const = match mode {
        PickerMode::OpenTree => "ACTION_OPEN_DOCUMENT_TREE",
        PickerMode::Create => "ACTION_CREATE_DOCUMENT",
        PickerMode::Open => "ACTION_OPEN_DOCUMENT",
    };
    let action = intent_string_const(env, action_const)?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|e| e.to_string())?;

    if matches!(mode, PickerMode::Open) {
        let cat = intent_string_const(env, "CATEGORY_OPENABLE")?;
        env.call_method(
            &intent,
            "addCategory",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&cat)],
        )
        .map_err(|e| e.to_string())?;
    }

    // `*/*` by default; a real app would map `allowed_types` to a MIME type.
    let star = env
        .new_string("*/*")
        .map_err(|e| e.to_string())?;
    env.call_method(
        &intent,
        "setType",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&star)],
    )
    .map_err(|e| e.to_string())?;

    if matches!(mode, PickerMode::Open) && options.multi_selection {
        let allow = intent_string_const(env, "EXTRA_ALLOW_MULTIPLE")?;
        env.call_method(
            &intent,
            "putExtra",
            "(Ljava/lang/String;Z)Landroid/content/Intent;",
            &[JValue::Object(&allow), JValue::Bool(1)],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(title) = options.title.as_ref() {
        let extra = intent_string_const(env, "EXTRA_TITLE")?;
        let t = env
            .new_string(title.clone())
            .map_err(|e| e.to_string())?;
        env.call_method(
            &intent,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&extra), JValue::Object(&t)],
        )
        .map_err(|e| e.to_string())?;
    }

    if matches!(mode, PickerMode::Create) {
        if let Some(name) = options.default_name.as_ref() {
            let extra = intent_string_const(env, "EXTRA_TITLE")?;
            let n = env
                .new_string(name.clone())
                .map_err(|e| e.to_string())?;
            env.call_method(
                &intent,
                "putExtra",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&extra), JValue::Object(&n)],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    env.call_method(
        activity,
        "startActivityForResult",
        "(Landroid/content/Intent;I)V",
        &[JValue::Object(&intent), JValue::Int(request_code)],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Look up a `public static final String` constant on `android.content.Intent`.
fn intent_string_const<'a>(
    env: &mut jni::JNIEnv<'a>,
    name: &str,
) -> Result<JObject<'a>, String> {
    env.get_static_field("android/content/Intent", name, "Ljava/lang/String;")
        .map_err(|e| e.to_string())?
        .l()
        .map_err(|e| e.to_string())
}

/// Forward an `Activity.onActivityResult` to the pending file-dialog callback.
///
/// # Safety
/// Called from the app's native `onActivityResult` hook. `data` is a JNI
/// `jobject` for the result `Intent` (may be null). The call must happen on a
/// thread attached to the JVM — `onActivityResult` always runs on the main
/// looper, which is already attached.
pub unsafe fn dispatch_activity_result(
    request_code: u32,
    result_code: i32,
    data: jobject,
) {
    let pending = PENDING.lock().take();
    let Some(pending) = pending else { return };
    if pending.request_code != request_code {
        // Not our request; restore it for another handler.
        *PENDING.lock() = Some(pending);
        return;
    }

    // `Activity.RESULT_OK == -1`; anything else (canceled, etc.) → None.
    if result_code != -1 {
        (pending.callback)(None);
        return;
    }

    let uris = AndroidJni::with_env(|env, _activity| {
        if data.is_null() {
            return None;
        }
        let data_obj = unsafe { JObject::from_raw(data) };
        parse_picked_uris(env, data_obj)
    });
    match uris.flatten() {
        Some(paths) => (pending.callback)(Some(FileInfo {
            path: paths,
            format: None,
        })),
        None => (pending.callback)(None),
    }
}

/// Extract selected `Uri`(s) from the result `Intent` as `content://` paths.
fn parse_picked_uris<'a>(env: &mut jni::JNIEnv<'a>, data: JObject<'a>) -> Option<Vec<PathBuf>> {
    let mut paths = Vec::new();

    // Single: Intent.getData() → Uri
    let uri = env
        .call_method(&data, "getData", "()Landroid/net/Uri;", &[])
        .ok()?
        .l()
        .ok()?;
    if !uri.is_null() {
        if let Some(p) = uri_to_path(env, uri) {
            paths.push(p);
        }
    } else {
        // Multiple: Intent.getClipData() → ClipData → items
        let clip = env
            .call_method(&data, "getClipData", "()Landroid/content/ClipData;", &[])
            .ok()?
            .l()
            .ok()?;
        if !clip.is_null() {
            let count = env
                .call_method(&clip, "getItemCount", "()I", &[])
                .ok()?
                .i()
                .ok()?;
            for i in 0..count {
                let item = env
                    .call_method(
                        &clip,
                        "getItemAt",
                        "(I)Landroid/content/ClipData$Item;",
                        &[JValue::Int(i)],
                    )
                    .ok()?
                    .l()
                    .ok()?;
                if item.is_null() {
                    continue;
                }
                let item_uri = env
                    .call_method(&item, "getUri", "()Landroid/net/Uri;", &[])
                    .ok()?
                    .l()
                    .ok()?;
                if !item_uri.is_null() {
                    if let Some(p) = uri_to_path(env, item_uri) {
                        paths.push(p);
                    }
                }
            }
        }
    }

    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

/// Convert a `Uri` to its string form (returned as a `content://` path).
///
/// Shared by the file-dialog and shared-file-intent modules.
pub(crate) fn uri_to_path(env: &mut jni::JNIEnv, uri: JObject) -> Option<PathBuf> {
    let s = env
        .call_method(&uri, "toString", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    if s.is_null() {
        return None;
    }
    let js = JString::from(s);
    let chars = env.get_string(&js).ok()?;
    let text: String = chars.into();
    Some(PathBuf::from(text))
}

// Keep `jint` referenced for documentation parity with the JNI signature used
// by the app-side native method.
#[allow(dead_code)]
fn _assert_jint(_: jint) {}
