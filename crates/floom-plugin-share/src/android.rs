//! Android share via `Intent.ACTION_SEND` and a chooser.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Text works, files do not
//!
//! `share_text` is a plain `ACTION_SEND` with `EXTRA_TEXT`, which needs no
//! permission and no manifest entry — the system chooser takes over and the
//! caller's job is done the moment `startActivity` returns.
//!
//! `share_files` cannot work in this app shape and says so. Since API 24,
//! handing another app a `file://` URI throws `FileUriExposedException`; the
//! supported route is a `content://` URI from a `FileProvider`, which is a
//! subclass of `androidx.core.content.FileProvider` — a Java class that must be
//! declared as a `<provider>` in the manifest and backed by an `androidx.core`
//! dependency. The example apps here are `android:hasCode="false"`
//! `NativeActivity` builds with no androidx and no `<provider>`, so there is no
//! provider authority to hand out. Returning `Err` with that reason is the
//! honest answer: the alternative is an `Intent` the system rejects with an
//! exception the caller never sees, or a `file://` URI that crashes the app.
//!
//! Wiring files up is therefore an app-level change (add androidx, add the
//! `<provider>` with `android:authorities="${applicationId}.fileprovider"`, and
//! pass that authority down) rather than something this plugin can do from the
//! Rust side.

use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

pub fn share_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        // An empty `EXTRA_TEXT` shows a chooser with nothing to share, and the
        // target app gets a blank body. That is a caller bug worth naming.
        return Err("share_text: the text to share is empty".into());
    }
    jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let intent = send_intent(env)?;

        // `intent.setType("text/plain")` — the MIME type is what makes the
        // chooser offer messaging and mail apps rather than only file handlers.
        let mime = env.new_string("text/plain").e()?;
        env.call_method(
            &intent,
            jni::jni_str!("setType"),
            jni::jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
            &[JValue::Object(&mime)],
        )
        .e()?;

        // `EXTRA_TEXT` is a `static final String` on `Intent`; it is fetched by
        // name so the wire format is the platform's, not a literal copy.
        let extra_text = static_string(env, "EXTRA_TEXT")?;
        let body = env.new_string(text).e()?;
        env.call_method(
            &intent,
            jni::jni_str!("putExtra"),
            jni::jni_sig!("(Ljava/lang/String;Ljava/lang/CharSequence;)Landroid/content/Intent;"),
            &[JValue::Object(&extra_text), JValue::Object(&body)],
        )
        .e()?;

        start_chooser(env, &activity, &intent)
    })
}

pub fn share_files(paths: &[&str]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("share_files: no paths given".into());
    }
    // The paths are checked before the reason is reported: a caller that passed
    // a typo deserves to hear about the typo, not about FileProvider.
    for path in paths {
        if !std::path::Path::new(path).exists() {
            return Err(format!("share_files: no such file: {path}"));
        }
    }
    Err(
        "share_files: sharing files needs a content:// URI from a FileProvider, \
         which this app does not declare (it is an androidx class and the app is \
         hasCode=\"false\"). Add androidx.core plus a <provider> with \
         android:authorities=\"${applicationId}.fileprovider\" to the manifest, \
         then pass that authority to this plugin. Text sharing works today."
            .into(),
    )
}

/// `new Intent(Intent.ACTION_SEND)`.
fn send_intent<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let intent_class = env
        .find_class(jni::jni_str!("android/content/Intent"))
        .e()?;
    let action = env
        .get_static_field(
            &intent_class,
            jni::jni_str!("ACTION_SEND"),
            jni::jni_sig!("Ljava/lang/String;"),
        )
        .e()?
        .l()
        .e()?;
    env.new_object(
        &intent_class,
        jni::jni_sig!("(Ljava/lang/String;)V"),
        &[JValue::Object(&action)],
    )
    .e()
}

/// Wrap the intent in a chooser and start it.
///
/// `Intent.createChooser` is used rather than `startActivity(intent)` so the
/// user gets the system picker: `ACTION_SEND` on a device with one matching app
/// would otherwise launch it directly, which is not what a "share" button means.
fn start_chooser(
    env: &mut jni::Env,
    activity: &JObject<'_>,
    intent: &JObject<'_>,
) -> Result<(), String> {
    let intent_class = env
        .find_class(jni::jni_str!("android/content/Intent"))
        .e()?;
    let title = env.new_string("Share via").e()?;
    let chooser = env
        .call_static_method(
            &intent_class,
            jni::jni_str!("createChooser"),
            jni::jni_sig!(
                "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;"
            ),
            &[JValue::Object(intent), JValue::Object(&title)],
        )
        .e()?
        .l()
        .e()?;
    // `startActivity` returns `void`, so there is no value to convert — only
    // the error path matters.
    env.call_method(
        activity,
        jni::jni_str!("startActivity"),
        jni::jni_sig!("(Landroid/content/Intent;)V"),
        &[JValue::Object(&chooser)],
    )
    .e()?;
    Ok(())
}

/// A `static final String` field of `android.content.Intent`.
fn static_string<'local>(
    env: &mut jni::Env<'local>,
    name: &str,
) -> Result<JObject<'local>, String> {
    let class = env
        .find_class(jni::jni_str!("android/content/Intent"))
        .e()?;
    env.get_static_field(
        &class,
        jni::strings::JNIString::from(name),
        jni::jni_sig!("Ljava/lang/String;"),
    )
    .e()?
    .l()
    .e()
}
