#![cfg(target_os = "android")]

//! Receiving files shared into the app from another Android app.
//!
//! When the user shares a file (or opens a file with Floem via the share sheet /
//! "open with"), the Android system delivers an `Intent` to the activity:
//!
//! - `ACTION_SEND` — a single file in `Intent.EXTRA_STREAM` (a `content://` Uri).
//! - `ACTION_SEND_MULTIPLE` — several files in `EXTRA_STREAM` (an `ArrayList`).
//! - `ACTION_VIEW` — a single file in `Intent.getData()`.
//!
//! android-activity 0.6 has **no** intent API, so Floem reads the intent through
//! the shared JNI bridge (`crate::platform::android_jni`) on the `Activity` object
//! it already holds:
//!
//! - **Launch intent** (the app was started by the share): `Activity.getIntent()`
//!   is read in `Application::new_android` via [`read_launch_intent`]. This works
//!   with the default `android.app.NativeActivity` — no Java subclass required.
//! - **Runtime intent** (the app was already running and the user shared again):
//!   the app's `FloemNativeActivity` must override `onNewIntent` and forward the
//!   `Intent` to [`dispatch_android_intent`] (same pattern as the file-dialog
//!   result hook).
//!
//! Delivered files are surfaced to the app through a handler registered with
//! [`set_shared_files_handler`]. If the handler is registered after the launch
//! intent has already been parsed, the queued files are flushed immediately.
//!
//! **Note on paths:** Android shares `content://` URIs, not filesystem paths.
//! Each received file carries its URI string; apps should open it via
//! `ContentResolver` (using the JNI bridge or a helper) rather than treating it
//! as a plain file path.

use jni::{
    objects::{JObject, JString, JValue},
    sys::jobject,
};
use parking_lot::Mutex;

use crate::platform::android_jni::AndroidJni;
use crate::platform::SharedFile;

type SharedFilesHandler = Box<dyn Fn(Vec<SharedFile>) + 'static>;

// Android delivers shared files on the main looper thread only, so the handler is
// not required to be `Send`. A `thread_local` avoids the `Sync` bound a `static
// Mutex` would impose on a `!Send` handler (Floem's UI signals such as `RwSignal`
// are `!Send`), while still giving a single global registration point per thread.
thread_local! {
    static HANDLER: Mutex<Option<SharedFilesHandler>> = Mutex::new(None);
    // Files that arrived before a handler was registered; flushed on registration.
    static PENDING: Mutex<Vec<Vec<SharedFile>>> = Mutex::new(Vec::new());
}

/// Register the handler invoked when files are shared into the app.
///
/// If a launch intent was already parsed before this call, the queued files are
/// delivered to `handler` immediately (in order). Subsequent shares (runtime
/// `onNewIntent`) invoke `handler` directly.
pub fn set_shared_files_handler(handler: SharedFilesHandler) {
    let queued: Vec<SharedFile> = PENDING.with(|pending| {
        let mut pending = pending.lock();
        pending.drain(..).flatten().collect()
    });
    HANDLER.with(|h| *h.lock() = Some(handler));
    if !queued.is_empty() {
        invoke_handler(&queued);
    }
}

/// Deliver shared files to the registered handler, or queue them if none is set.
fn on_shared_files(files: Vec<SharedFile>) {
    if files.is_empty() {
        return;
    }
    let has_handler = HANDLER.with(|h| h.lock().is_some());
    if has_handler {
        invoke_handler(&files);
    } else {
        PENDING.with(|pending| pending.lock().push(files));
    }
}

/// Invoke the handler. The handler must not re-enter the `set_shared_files_handler`
/// / shared-file APIs (it would deadlock the `HANDLER` lock).
fn invoke_handler(files: &[SharedFile]) {
    HANDLER.with(|h| {
        if let Some(handler) = h.lock().as_ref() {
            handler(files.to_vec());
        }
    });
}

/// Read the launch intent (`Activity.getIntent()`) and deliver any shared files.
///
/// Called from `Application::new_android` after the JNI bridge is initialized.
/// Safe to call with the default `NativeActivity`; no Java subclass required.
pub(crate) fn read_launch_intent() {
    let files = AndroidJni::with_env(|env, activity| {
        let intent = env
            .call_method(activity, "getIntent", "()Landroid/content/Intent;", &[])
            .ok()?
            .l()
            .ok()?;
        if intent.is_null() {
            return None;
        }
        Some(parse_intent(env, intent))
    });
    if let Some(files) = files.flatten() {
        on_shared_files(files);
    }
}

/// Forward an `Activity.onNewIntent(Intent)` to the shared-file handler.
///
/// # Safety
/// Called from the app's native `onNewIntent` hook. `intent` is a JNI `jobject`
/// for the new `Intent` (non-null when `onNewIntent` fires). The call runs on the
/// main looper, which is already attached to the JVM.
pub unsafe fn dispatch_android_intent(intent: jobject) {
    if intent.is_null() {
        return;
    }
    let files = AndroidJni::with_env(|env, _activity| {
        let intent_obj = unsafe { JObject::from_raw(intent) };
        Some(parse_intent(env, intent_obj))
    });
    if let Some(files) = files.flatten() {
        on_shared_files(files);
    }
}

/// Extract the shared `Uri`(s) from a share `Intent` into `SharedFile`s.
fn parse_intent(env: &mut jni::JNIEnv, intent: JObject) -> Vec<SharedFile> {
    let mut out = Vec::new();
    let mime = get_string_method(env, &intent, "getType", "()Ljava/lang/String;");

    // EXTRA_STREAM — single (ACTION_SEND) or list (ACTION_SEND_MULTIPLE).
    if let Some(uri) = get_single_extra_uri(env, &intent) {
        if let Some(file) = uri_to_shared(env, uri, mime.clone()) {
            out.push(file);
        }
    } else if let Some(list) = get_multi_extra_uris(env, &intent) {
        let count = env
            .call_method(&list, "size", "()I", &[])
            .ok()
            .and_then(|v| v.i().ok())
            .unwrap_or(0);
        for i in 0..count {
            let item = env
                .call_method(&list, "get", "(I)Ljava/lang/Object;", &[JValue::Int(i)])
                .ok()
                .and_then(|v| v.l().ok())
                .filter(|o| !o.is_null());
            if let Some(item) = item {
                if let Some(file) = uri_to_shared(env, item, mime.clone()) {
                    out.push(file);
                }
            }
        }
    }

    // ACTION_VIEW — getData().
    if out.is_empty() {
        if let Some(uri) = get_data_uri(env, &intent) {
            if let Some(file) = uri_to_shared(env, uri, mime) {
                out.push(file);
            }
        }
    }

    out
}

fn get_single_extra_uri<'a>(
    env: &mut jni::JNIEnv<'a>,
    intent: &JObject,
) -> Option<JObject<'a>> {
    let extra = env.new_string("android.intent.extra.STREAM").ok()?;
    env.call_method(
        intent,
        "getParcelableExtra",
        "(Ljava/lang/String;)Landroid/os/Parcelable;",
        &[JValue::Object(&extra)],
    )
    .ok()?
    .l()
    .ok()
    .filter(|o| !o.is_null())
}

fn get_multi_extra_uris<'a>(
    env: &mut jni::JNIEnv<'a>,
    intent: &JObject,
) -> Option<JObject<'a>> {
    let extra = env.new_string("android.intent.extra.STREAM").ok()?;
    env.call_method(
        intent,
        "getParcelableArrayListExtra",
        "(Ljava/lang/String;)Ljava/util/ArrayList;",
        &[JValue::Object(&extra)],
    )
    .ok()?
    .l()
    .ok()
    .filter(|o| !o.is_null())
}

fn get_data_uri<'a>(env: &mut jni::JNIEnv<'a>, intent: &JObject) -> Option<JObject<'a>> {
    env.call_method(intent, "getData", "()Landroid/net/Uri;", &[])
        .ok()?
        .l()
        .ok()
        .filter(|o| !o.is_null())
}

fn uri_to_shared(env: &mut jni::JNIEnv, uri: JObject, mime: Option<String>) -> Option<SharedFile> {
    // Borrow `uri` for the display name BEFORE moving it into `uri_to_path`.
    let display_name = env
        .call_method(&uri, "getLastPathSegment", "()Ljava/lang/String;", &[])
        .ok()
        .and_then(|v| v.l().ok())
        .filter(|o| !o.is_null())
        .and_then(|s| jstring_to_string(env, s));
    let uri_str = crate::platform::android_file::uri_to_path(env, uri)?
        .to_string_lossy()
        .into_owned();
    Some(SharedFile {
        uri: uri_str,
        mime_type: mime,
        display_name,
    })
}

fn get_string_method(
    env: &mut jni::JNIEnv,
    obj: &JObject,
    name: &str,
    sig: &str,
) -> Option<String> {
    env.call_method(obj, name, sig, &[])
        .ok()
        .and_then(|v| v.l().ok())
        .filter(|o| !o.is_null())
        .and_then(|s| jstring_to_string(env, s))
}

fn jstring_to_string(env: &mut jni::JNIEnv, s: JObject) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let js = JString::from(s);
    let chars = env.get_string(&js).ok()?;
    Some(chars.into())
}
