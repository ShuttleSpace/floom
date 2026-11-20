//! Android URL opening via `Intent.ACTION_VIEW`.
//!
//! The Java sequence is the one every Android app uses:
//!
//! ```java
//! Intent i = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
//! activity.startActivity(i);
//! ```
//!
//! # Why no chooser
//!
//! `floom-plugin-share` wraps its intent in `Intent.createChooser` because a
//! share button means "pick one". This crate deliberately does not: the point
//! of opening a URL is the *default* handler — the browser the user chose,
//! the mail app they set. Wrapping `ACTION_VIEW` in a chooser would turn
//! every link into a prompt, which is not what "open" means.
//!
//! # Where platform refusals surface
//!
//! `startActivity` throws `ActivityNotFoundException` when no app handles the
//! scheme, and `FileUriExposedException` for cross-app `file://` URIs (API
//! 24+). The JNI bridge surfaces a thrown exception as the call's `Err`, so
//! the caller hears the platform's reason verbatim — this arm does not
//! translate it, because the reason is already the truth.

use floom_platform_android::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::{JObject, JValue};

pub fn open_url(url: &str) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let intent_class = env
            .find_class(jni::jni_str!("android/content/Intent"))
            .e()?;
        let uri_class = env.find_class(jni::jni_str!("android/net/Uri")).e()?;

        // `Uri.parse(url)` — the Intent constructor takes a `Uri`, not a
        // string, so the URL is parsed on the Java side where the platform's
        // own parser does the work.
        let url_arg: JObject = env.new_string(url).e()?.into();
        let uri = env
            .call_static_method(
                &uri_class,
                jni::jni_str!("parse"),
                jni::jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[obj(&url_arg)],
            )
            .and_then(|v| v.l())
            .e()?;

        // `ACTION_VIEW` is a `static final String` on `Intent`, fetched by
        // name so the wire format is the platform's, not a literal copy.
        let action = env
            .get_static_field(
                &intent_class,
                jni::jni_str!("ACTION_VIEW"),
                jni::jni_sig!("Ljava/lang/String;"),
            )
            .e()?
            .l()
            .e()?;

        // `new Intent(action, uri)`.
        let intent = env
            .new_object(
                &intent_class,
                jni::jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
                &[JValue::Object(&action), JValue::Object(&uri)],
            )
            .e()?;

        // `startActivity` returns `void`, so only the error path matters —
        // and a thrown exception arrives as that error.
        env.call_method(
            &activity,
            jni::jni_str!("startActivity"),
            jni::jni_sig!("(Landroid/content/Intent;)V"),
            &[JValue::Object(&intent)],
        )
        .e()?;
        Ok(())
    })
}
