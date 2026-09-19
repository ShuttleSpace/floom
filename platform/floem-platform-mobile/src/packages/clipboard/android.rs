//! Android clipboard via JNI (`android.content.ClipboardManager`).
//!
//! There is no NDK clipboard API, so this is the only route to the system
//! clipboard. The Java sequence is small:
//!
//! ```java
//! ClipboardManager cm = (ClipboardManager) activity.getSystemService("clipboard");
//! cm.setPrimaryClip(ClipData.newPlainText("floem", text));      // set
//! ClipData clip = cm.getPrimaryClip();                         // get
//! CharSequence cs = clip.getItemAt(0).getText();
//! ```
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use crate::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::{JObject, JString, JValue};
use jni::sys::jint;

/// `Context.CLIPBOARD_SERVICE`.
const CLIPBOARD_SERVICE: &str = "clipboard";

/// Label attached to clips we write. Android shows it only in clipboard
/// previews on newer versions; it is not part of the payload.
const CLIP_LABEL: &str = "floem";

/// The `ClipboardManager`, or `Err` if the service is unavailable.
///
/// `getSystemService` is declared to return `Object`, and the platform
/// guarantees a `ClipboardManager` for this service name, so no explicit
/// `checkcast` is needed — `invokevirtual` resolves the manager methods by
/// name + signature at run time.
fn clipboard_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    // `JString` is a subclass of `JObject`; `JValue::Object` wants the parent,
    // so widen it here rather than at each call site.
    let service: JObject = env.new_string(CLIPBOARD_SERVICE).e()?.into();
    let manager = env
        .call_method(
            &activity,
            jni::jni_str!("getSystemService"),
            jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[obj(&service)],
        )
        .and_then(|value| value.l())
        .e()?;
    if manager.is_null() {
        return Err("ClipboardManager service not available".into());
    }
    Ok(manager)
}

pub fn set_text(text: &str) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let manager = clipboard_manager(env)?;

        // `ClipData.newPlainText(CharSequence label, CharSequence text)` is a
        // static factory on `android.content.ClipData`.
        let clip_data = env
            .find_class(jni::jni_str!("android/content/ClipData"))
            .e()?;
        let label: JObject = env.new_string(CLIP_LABEL).e()?.into();
        let payload: JObject = env.new_string(text).e()?.into();
        let clip = env
            .call_static_method(
                &clip_data,
                jni::jni_str!("newPlainText"),
                jni::jni_sig!(
                    "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"
                ),
                &[obj(&label), obj(&payload)],
            )
            .and_then(|value| value.l())
            .e()?;
        if clip.is_null() {
            return Err("ClipData.newPlainText returned null".into());
        }

        env.call_method(
            &manager,
            jni::jni_str!("setPrimaryClip"),
            jni::jni_sig!("(Landroid/content/ClipData;)V"),
            &[obj(&clip)],
        )
        .e()?;
        Ok(())
    })
}

pub fn get_text() -> Result<Option<String>, String> {
    jni_helpers::with_env(|env| {
        let manager = clipboard_manager(env)?;

        // Any throw past here would leave a pending exception that breaks the
        // next JNI call, so every early return clears one first.
        let clip = match env
            .call_method(
                &manager,
                jni::jni_str!("getPrimaryClip"),
                jni::jni_sig!("()Landroid/content/ClipData;"),
                &[],
            )
            .and_then(|value| value.l())
        {
            Ok(clip) => clip,
            Err(_) => {
                // Typically a `SecurityException` because the app is not
                // focused (Android 10+). "No readable clipboard" is not an
                // error for the caller.
                env.exception_clear();
                return Ok(None);
            }
        };
        if clip.is_null() {
            return Ok(None);
        }

        let count: jint = env
            .call_method(
                &clip,
                jni::jni_str!("getItemCount"),
                jni::jni_sig!("()I"),
                &[],
            )
            .and_then(|value| value.i())
            .unwrap_or(0);
        if count <= 0 {
            return Ok(None);
        }

        let item = env
            .call_method(
                &clip,
                jni::jni_str!("getItemAt"),
                jni::jni_sig!("(I)Landroid/content/ClipData$Item;"),
                &[JValue::Int(0)],
            )
            .and_then(|value| value.l())
            .e()?;
        if item.is_null() {
            return Ok(None);
        }

        // `getText` is the fast path; it is null for a clip carrying a URI or
        // an Intent instead of text.
        let text = env
            .call_method(
                &item,
                jni::jni_str!("getText"),
                jni::jni_sig!("()Ljava/lang/CharSequence;"),
                &[],
            )
            .and_then(|value| value.l())
            .e()?;
        if text.is_null() {
            return Ok(None);
        }

        // The concrete `CharSequence` may be a `SpannedString`, so go through
        // `toString()` and then cast: `IsInstanceOf(String)` is only guaranteed
        // after that call. `cast_local` consumes the object, so hand over the
        // owned one and let the cast re-wrap it.
        let as_string: JObject = env
            .call_method(
                &text,
                jni::jni_str!("toString"),
                jni::jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .and_then(|value| value.l())
            .e()?;
        if as_string.is_null() {
            return Ok(None);
        }
        // Turbofish is required: the target type cannot be inferred from the
        // binding, because `cast_local` returns `To::Kind<'local>`.
        let string: JString = env.cast_local::<JString>(as_string).e()?;
        Ok(Some(string.try_to_string(env).e()?))
    })
}
