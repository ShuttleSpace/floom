//! Android preferences via `SharedPreferences`.
//!
//! The Java sequence, per call:
//!
//! ```java
//! SharedPreferences sp = activity.getSharedPreferences(store, MODE_PRIVATE);
//! sp.getString(key, null);                    // get
//! sp.edit().putString(key, value).commit();  // set
//! sp.edit().remove(key).commit();            // remove
//! ```
//!
//! # Why `commit()` and not `apply()`
//!
//! `apply()` writes on a background thread and returns immediately — an
//! `Ok` from it would claim a write that may not have happened yet, and the
//! last value of a session could be lost on process death. `commit()` blocks
//! and returns whether the file was written. Preferences are small and writes
//! are rare; the honesty is worth the milliseconds, and the crate doc says
//! `Ok` means *stored*.
//!
//! # Why the store is re-fetched per call
//!
//! `getSharedPreferences` caches by name on the Java side, so re-reaching it
//! costs a map lookup — and holding the returned object in Rust would mean
//! holding a Java reference across calls, which the bridge deliberately does
//! not do. The cheap reach is the honest shape.

use floom_platform_mobile::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::JObject;

/// `Context.MODE_PRIVATE` — the only mode that has existed since API 17
/// deprecated the others.
const MODE_PRIVATE: i32 = 0;

pub fn get(store: &str, key: &str) -> Option<String> {
    jni_helpers::with_env(|env| {
        let sp = shared_prefs(env, store)?;
        let key: JObject = env.new_string(key).e()?.into();
        // `getString(key, null)` — the null default is what makes "absent"
        // arrive as a null object rather than as a string to interpret.
        let value = env
            .call_method(
                &sp,
                jni::jni_str!("getString"),
                jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"),
                &[obj(&key), obj(&JObject::null())],
            )
            .and_then(|v| v.l())
            .e()?;
        if value.is_null() {
            return Ok(None);
        }
        let string = env
            .as_cast::<jni::objects::JString<'_>>(&value)
            .map_err(|e| e.to_string())?;
        Ok(string.try_to_string(env).ok())
    })
    .ok()
    .flatten()
}

pub fn set(store: &str, key: &str, value: &str) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let sp = shared_prefs(env, store)?;
        let editor = env
            .call_method(
                &sp,
                jni::jni_str!("edit"),
                jni::jni_sig!("()Landroid/content/SharedPreferences$Editor;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        if editor.is_null() {
            return Err("prefs: SharedPreferences.edit() returned null".into());
        }
        let key: JObject = env.new_string(key).e()?.into();
        let value: JObject = env.new_string(value).e()?.into();
        env.call_method(
            &editor,
            jni::jni_str!("putString"),
            jni::jni_sig!(
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;"
            ),
            &[obj(&key), obj(&value)],
        )
        .e()?;
        commit(env, &editor)
    })
}

pub fn remove(store: &str, key: &str) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let sp = shared_prefs(env, store)?;
        let editor = env
            .call_method(
                &sp,
                jni::jni_str!("edit"),
                jni::jni_sig!("()Landroid/content/SharedPreferences$Editor;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        if editor.is_null() {
            return Err("prefs: SharedPreferences.edit() returned null".into());
        }
        let key: JObject = env.new_string(key).e()?.into();
        env.call_method(
            &editor,
            jni::jni_str!("remove"),
            jni::jni_sig!("(Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;"),
            &[obj(&key)],
        )
        .e()?;
        commit(env, &editor)
    })
}

/// `editor.commit()` — the boolean is the write's own answer.
fn commit(env: &mut jni::Env, editor: &JObject<'_>) -> Result<(), String> {
    let wrote = env
        .call_method(editor, jni::jni_str!("commit"), jni::jni_sig!("()Z"), &[])
        .and_then(|v| v.z())
        .e()?;
    if wrote {
        Ok(())
    } else {
        Err("prefs: SharedPreferences.commit() reported the write failed".into())
    }
}

/// `activity.getSharedPreferences(store, MODE_PRIVATE)`.
fn shared_prefs<'local>(
    env: &mut jni::Env<'local>,
    store: &str,
) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let name: JObject = env.new_string(store).e()?.into();
    let sp = env
        .call_method(
            &activity,
            jni::jni_str!("getSharedPreferences"),
            jni::jni_sig!("(Ljava/lang/String;I)Landroid/content/SharedPreferences;"),
            &[obj(&name), jni::objects::JValue::Int(MODE_PRIVATE)],
        )
        .and_then(|v| v.l())
        .e()?;
    if sp.is_null() {
        return Err("prefs: getSharedPreferences returned null".into());
    }
    Ok(sp)
}
