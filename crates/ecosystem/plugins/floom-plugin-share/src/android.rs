//! Android share via `SharePlugin`.
//!
//! The capability is Java: building an `ACTION_SEND` intent, wrapping it in a
//! chooser and starting it are Android APIs, and `SharePlugin` owns them. This
//! file only carries the ask across JNI.
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

use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `SharePlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.share.SharePlugin";

pub fn share_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        // An empty `EXTRA_TEXT` shows a chooser with nothing to share, and the
        // target app gets a blank body. That is a caller bug worth naming.
        return Err("share_text: the text to share is empty".into());
    }
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let body = env.new_string(text).e()?;
        let shared = env
            .call_static_method(
                &class,
                jni::jni_str!("shareText"),
                jni::jni_sig!("(Landroid/content/Context;Ljava/lang/String;)Z"),
                &[JValue::Object(&context), JValue::Object(&body)],
            )
            .e()?
            .z()
            .e()?;
        if shared {
            Ok(())
        } else {
            Err("share_text: SharePlugin.shareText reported the chooser did not start".into())
        }
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

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
