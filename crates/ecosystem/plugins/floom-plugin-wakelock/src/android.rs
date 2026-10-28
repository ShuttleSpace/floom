//! Android screen-awake via `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON`.
//!
//! `activity.getWindow().addFlags(FLAG_KEEP_SCREEN_ON)` — no permission,
//! because the flag's meaning is scoped to the app's own visible window:
//! the system keeps the display on *while this window is on screen*, and
//! forgets the ask the moment it is not. That is precisely the guarantee
//! the crate documents, which is why the flag (not a wake lock, which is a
//! resource with a permission and a lifecycle) is the right arm.
//!
//! The flag is read back through `getAttributes().flags`, so
//! `is_keep_screen_on` reports the window's actual state rather than a
//! shadow of what this crate last said.

use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};
use jni::strings::JNIString;

/// `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON` — a window flag, not a
/// power-manager wake lock.
const FLAG_KEEP_SCREEN_ON: i32 = 0x80;

pub fn keep_screen_on(on: bool) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let window = get_window(env)?;
        let method = if on { "addFlags" } else { "clearFlags" };
        env.call_method(
            &window,
            JNIString::from(method),
            jni::jni_sig!("(I)V"),
            &[JValue::Int(FLAG_KEEP_SCREEN_ON)],
        )
        .e()?;
        Ok(())
    })
}

pub fn is_keep_screen_on() -> bool {
    jni_helpers::with_env(|env| {
        let window = get_window(env)?;
        let attrs = env
            .call_method(
                &window,
                jni::jni_str!("getAttributes"),
                jni::jni_sig!("()Landroid/view/WindowManager$LayoutParams;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        if attrs.is_null() {
            return Ok(false);
        }
        // `flags` is a public field of the LayoutParams the window handed
        // back — the window's own truth, not this crate's memory of it.
        let flags = env
            .get_field(&attrs, jni::jni_str!("flags"), jni::jni_sig!("I"))
            .and_then(|v| v.i())
            .e()?;
        Ok((flags & FLAG_KEEP_SCREEN_ON) != 0)
    })
    .unwrap_or(false)
}

/// `activity.getWindow()`.
fn get_window<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let window = env
        .call_method(
            &activity,
            jni::jni_str!("getWindow"),
            jni::jni_sig!("()Landroid/view/Window;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    if window.is_null() {
        return Err("keep_screen_on: the activity has no window yet".into());
    }
    Ok(window)
}
