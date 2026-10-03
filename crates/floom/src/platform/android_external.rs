#![cfg(target_os = "android")]

//! Open external URLs / apps on Android via an `ACTION_VIEW` intent.
//!
//! Android has no `open`/`xdg-open` equivalent; opening a URL means building an
//! `Intent` with `ACTION_VIEW` and a `Uri` parsed from the URL, then calling
//! `Context.startActivity`. This is fire-and-forget (no result is needed), so
//! it is wired directly through the shared JNI bridge in
//! [`crate::platform::android_jni`].

use jni::objects::JValue;

use crate::platform::android_jni::AndroidJni;

/// Open `url` in the default external handler (browser for `http(s)://`,
/// appropriate app for other schemes).
///
/// Returns an error string on failure (e.g. no app handles the scheme, or the
/// JNI bridge was not initialized via `Application::new_android`).
pub(crate) fn open_url(url: &str) -> Result<(), String> {
    AndroidJni::with_env(|env, activity| {
        // Uri uri = Uri.parse(url);
        let url_j = env
            .new_string(url)
            .map_err(|e| e.to_string())?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&url_j)],
            )
            .map_err(|e| e.to_string())?
            .l()
            .map_err(|e| e.to_string())?;
        if uri.is_null() {
            return Err("Uri.parse returned null".to_string());
        }

        // Intent intent = new Intent(Intent.ACTION_VIEW, uri);
        let action = env
            .get_static_field("android/content/Intent", "ACTION_VIEW", "Ljava/lang/String;")
            .map_err(|e| e.to_string())?
            .l()
            .map_err(|e| e.to_string())?;
        let intent = env
            .new_object(
                "android/content/Intent",
                "(Ljava/lang/String;Landroid/net/Uri;)V",
                &[JValue::Object(&action), JValue::Object(&uri)],
            )
            .map_err(|e| e.to_string())?;

        // activity.startActivity(intent);  (Activity is a Context)
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .unwrap_or(Err("Android JNI bridge not initialized".to_string()))
}
