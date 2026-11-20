//! Android directories via `Context`.
//!
//! The mapping is the crate contract's Android column:
//!
//! | [`AppDirectory`] | Java call | Notes |
//! |---|---|---|
//! | `ApplicationSupport` | `getFilesDir()` | app-private internal storage |
//! | `Documents` | `getFilesDir()` | a phone's "documents" *is* app-private storage; the shared user-visible folder is the file-picker capability, not a path |
//! | `Cache` | `getCacheDir()` | internal cache, evictable |
//! | `Downloads` | — | redirected at [`super::external_files_dir`]'s territory; no internal equivalent |
//!
//! Every read goes through the JNI bridge `floom-platform-android` owns, and
//! every failure is a `None` rather than an `Err`: "this app's directory on
//! this device" is a question the caller cannot retry into a different answer,
//! and a missing-but-expected directory is an answer, not a failure.
//!
//! # `getFilesDir` can return null
//!
//! Its documented null case is "internal storage is not yet mounted" — rare,
//! but real on a device that just booted. The arm reports it as `None`, which
//! is the honest "no answer right now", not an error the caller would have to
//! distinguish from a broken bridge.

use super::AppDirectory;
use floom_platform_android::jni::{self as jni_helpers, JniExt};
use jni::objects::JObject;
use jni::strings::JNIString;
use std::path::PathBuf;

pub fn app_dir(dir: AppDirectory) -> Option<PathBuf> {
    jni_helpers::with_env(|env| read_dir(env, dir))
        .ok()
        .flatten()
}

pub fn external_files_dir() -> Option<PathBuf> {
    jni_helpers::with_env(read_external).ok().flatten()
}

fn read_dir(env: &mut jni::Env, dir: AppDirectory) -> Result<Option<PathBuf>, String> {
    let activity = jni_helpers::activity(env)?;
    // All three answers come from `Context` as a `java.io.File`; only the
    // method name differs, so one call shape serves the arm. The name is a
    // runtime `JNIString` because it is picked by a match, and `jni_str!`
    // only takes literals.
    let method = match dir {
        AppDirectory::ApplicationSupport | AppDirectory::Documents => "getFilesDir",
        AppDirectory::Cache => "getCacheDir",
        AppDirectory::Downloads => return Ok(None),
    };
    let file = env
        .call_method(
            &activity,
            JNIString::from(method),
            jni::jni_sig!("()Ljava/io/File;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    file_to_path(env, &file)
}

fn read_external(env: &mut jni::Env) -> Result<Option<PathBuf>, String> {
    let activity = jni_helpers::activity(env)?;
    // `getExternalFilesDir(null)`: the null type argument asks for the root of
    // the app's external storage. The documented null *return* means "external
    // storage is not available" (removed SD card, pre-API-19 shared storage
    // not yet mounted), which this arm reports as `None` rather than an
    // `Err` — the state is a fact about the device, not a failed call.
    let file = env
        .call_method(
            &activity,
            jni::jni_str!("getExternalFilesDir"),
            jni::jni_sig!("(Ljava/lang/String;)Ljava/io/File;"),
            &[jni::objects::JValue::Object(&JObject::null())],
        )
        .and_then(|v| v.l())
        .e()?;
    file_to_path(env, &file)
}

/// A `java.io.File` as a path, with `null` meaning "no answer".
fn file_to_path(env: &mut jni::Env, file: &JObject<'_>) -> Result<Option<PathBuf>, String> {
    if file.is_null() {
        return Ok(None);
    }
    let path = env
        .call_method(
            file,
            jni::jni_str!("getAbsolutePath"),
            jni::jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    if path.is_null() {
        return Ok(None);
    }
    let string = env
        .as_cast::<jni::objects::JString<'_>>(&path)
        .map_err(|e| e.to_string())?;
    Ok(string.try_to_string(env).ok().map(PathBuf::from))
}
