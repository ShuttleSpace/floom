//! Android local notifications via `NotificationManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # The permission is a request, not a reading
//!
//! Since API 33, `POST_NOTIFICATIONS` is a runtime permission. Asking for it
//! goes through `Activity.requestPermissions`, and the answer arrives in
//! `onRequestPermissionsResult` — a callback this crate does not take.
//!
//! What it can do honestly:
//!
//! * `are_notifications_enabled` reads `NotificationManager.areNotificationsEnabled()`,
//!   which is the system's own answer and is exact on every API level.
//! * `request_authorization` **issues the real request**. `Ok` means "the OS has
//!   been asked", not "the user agreed": the prompt is up, the answer lands in a
//!   callback nobody here receives, and the caller polls
//!   `are_notifications_enabled()`. That is a different claim from returning
//!   `Ok` for a grant, which would be the lie this plugin family exists to
//!   avoid.
//!
//! Below API 33 the permission does not exist, so the app may post without
//! asking and the request is a no-op `Ok`.
//!
//! # Scheduling
//!
//! `delay_seconds: Some(n)` needs `AlarmManager`, which is a different
//! mechanism with its own permission story (`SCHEDULE_EXACT_ALARM` since API
//! 31). This arm delivers immediately and refuses a delayed notification
//! plainly, because delivering it *now* would be a notification the user sees at
//! the wrong time — worse than not seeing it.

use super::Notification;
use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

/// `Manifest.permission.POST_NOTIFICATIONS`.
const POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";

/// `Build.VERSION_CODES.TIRAMISU`, the API level that introduced the runtime
/// permission.
const API_TIRAMISU: i32 = 33;

pub fn are_notifications_enabled() -> bool {
    jni_helpers::with_env(|env| {
        let manager = notification_manager(env)?;
        let enabled = env
            .call_method(
                &manager,
                jni::jni_str!("areNotificationsEnabled"),
                jni::jni_sig!("()Z"),
                &[],
            )
            .e()?
            .z()
            .e()?;
        Ok(enabled)
    })
    .unwrap_or(false)
}

pub fn request_authorization() -> Result<(), String> {
    jni_helpers::with_env(|env| {
        // On API 33+ a grant is required before anything appears, so a request
        // that was already answered "yes" needs no prompt.
        if sdk_int(env)? < API_TIRAMISU {
            return Ok(());
        }
        let activity = jni_helpers::activity(env)?;
        let permission = env.new_string(POST_NOTIFICATIONS).e()?;
        let granted = env
            .call_method(
                &activity,
                jni::jni_str!("checkSelfPermission"),
                jni::jni_sig!("(Ljava/lang/String;)I"),
                &[JValue::Object(&permission)],
            )
            .e()?
            .i()
            .e()?;
        if granted == 0 {
            // `PERMISSION_GRANTED`: already answered yes, nothing to ask.
            return Ok(());
        }

        // `Activity.requestPermissions(String[], int)`. The request code is
        // arbitrary: the result is delivered to `onRequestPermissionsResult`,
        // which this plugin does not implement, so the value is never read back.
        // The prompt still appears, which is the whole job of this call.
        let permission_array = env
            .new_object_array(1, jni::jni_str!("java/lang/String"), &permission)
            .e()?;
        env.call_method(
            &activity,
            jni::jni_str!("requestPermissions"),
            jni::jni_sig!("([Ljava/lang/String;I)V"),
            &[
                JValue::Object(&JObject::from(permission_array)),
                JValue::Int(0),
            ],
        )
        .e()?;
        Ok(())
    })
}

/// `Build.VERSION.SDK_INT`.
fn sdk_int(env: &mut jni::Env) -> Result<i32, String> {
    let version = env
        .find_class(jni::jni_str!("android/os/Build$VERSION"))
        .e()?;
    env.get_static_field(&version, jni::jni_str!("SDK_INT"), jni::jni_sig!("I"))
        .e()?
        .i()
        .e()
}

pub fn schedule(notification: &Notification) -> Result<(), String> {
    if notification.title.is_empty() {
        return Err("notifications: a notification needs a title".into());
    }
    if notification.delay_seconds.is_some() {
        return Err("notifications: delayed delivery needs AlarmManager (and \
             SCHEDULE_EXACT_ALARM on API 31+), which this plugin does not wire. \
             Pass delay_seconds: None to deliver immediately."
            .into());
    }
    jni_helpers::with_env(|env| {
        let manager = notification_manager(env)?;
        let builder = notification_builder(env)?;

        set_char_sequence(env, &builder, "setTitle", &notification.title)?;
        set_char_sequence(env, &builder, "setContentText", &notification.body)?;

        if let Some(badge) = notification.badge {
            env.call_method(
                &builder,
                jni::jni_str!("setNumber"),
                jni::jni_sig!("(I)Landroid/app/Notification$Builder;"),
                &[JValue::Int(badge as i32)],
            )
            .e()?;
        }

        // The channel is what makes the notification appear at all on API 26+;
        // a builder without one is dropped by the system with a log line.
        let channel = channel_id(env, notification)?;
        env.call_method(
            &builder,
            jni::jni_str!("setChannelId"),
            jni::jni_sig!("(Ljava/lang/String;)Landroid/app/Notification$Builder;"),
            &[JValue::Object(&channel)],
        )
        .e()?;

        if notification.sound {
            // `NotificationChannel` carries the sound on API 26+, so this is
            // only meaningful on the pre-26 path; on 26+ the channel's
            // importance is what decides it.
            //
            // The class is resolved into a local first: `get_static_field`
            // already borrows `env` mutably, so looking the class up inline
            // would be a second borrow.
            let notification_class = env
                .find_class(jni::jni_str!("android/app/Notification"))
                .e()?;
            let default_sound = env
                .get_static_field(
                    &notification_class,
                    jni::jni_str!("DEFAULT_SOUND"),
                    jni::jni_sig!("Landroid/net/Uri;"),
                )
                .e()?
                .l()
                .e()?;
            env.call_method(
                &builder,
                jni::jni_str!("setSound"),
                jni::jni_sig!("(Landroid/net/Uri;)Landroid/app/Notification$Builder;"),
                &[JValue::Object(&default_sound)],
            )
            .e()?;
        }

        let built = env
            .call_method(
                &builder,
                jni::jni_str!("build"),
                jni::jni_sig!("()Landroid/app/Notification;"),
                &[],
            )
            .e()?
            .l()
            .e()?;

        // The tag is the notification's identity: reusing one replaces the
        // previous notification instead of stacking duplicates, which is what a
        // caller that reschedules the same thing expects.
        let tag = env.new_string("floom-notification").e()?;
        env.call_method(
            &manager,
            jni::jni_str!("notify"),
            jni::jni_sig!("(Ljava/lang/String;ILandroid/app/Notification;)V"),
            &[JValue::Object(&tag), JValue::Int(1), JValue::Object(&built)],
        )
        .e()?;
        Ok(())
    })
}

/// `Context.getSystemService(NotificationManager.class)`.
fn notification_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let class = env
        .find_class(jni::jni_str!("android/app/NotificationManager"))
        .e()?;
    env.call_method(
        &activity,
        jni::jni_str!("getSystemService"),
        jni::jni_sig!("(Ljava/lang/Class;)Ljava/lang/Object;"),
        &[JValue::Object(&JObject::from(class))],
    )
    .e()?
    .l()
    .e()
}

/// `new Notification.Builder(context, channelId)`.
///
/// The two-argument constructor is API 26+; the one-argument form is deprecated
/// and produces a notification with no channel, which the system drops. The
/// example app's `minSdkVersion` is 24, so the class is still checked for
/// existence before the call and a missing channel is reported rather than
/// silently dropped.
fn notification_builder<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let class = env
        .find_class(jni::jni_str!("android/app/Notification$Builder"))
        .e()?;
    let channel = env.new_string("floom").e()?;
    let builder = env
        .new_object(
            &class,
            jni::jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
            &[JValue::Object(&activity), JValue::Object(&channel)],
        )
        .e()?;
    Ok(builder)
}

/// A `CharSequence` setter on the builder (`setTitle` / `setContentText`).
fn set_char_sequence(
    env: &mut jni::Env,
    builder: &JObject<'_>,
    method: &str,
    value: &str,
) -> Result<(), String> {
    // The method name is a runtime string here because it is shared by the two
    // setters; the signature is a literal because `jni_sig!` only takes one.
    let name = jni::strings::JNIString::from(method);
    let text = env.new_string(value).e()?;
    env.call_method(
        builder,
        name,
        jni::jni_sig!("(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;"),
        &[JValue::Object(&text)],
    )
    .e()?;
    Ok(())
}

/// The notification channel id, creating the channel on API 26+.
///
/// A channel is a persistent system object with a user-visible importance
/// setting; creating it every call is idempotent (the system keeps the user's
/// choice if it already exists), which is why it is done inline instead of
/// tracked in a flag.
fn channel_id<'local>(
    env: &mut jni::Env<'local>,
    notification: &Notification,
) -> Result<JObject<'local>, String> {
    let manager = notification_manager(env)?;
    // The id is handed back to the caller as a `JObject` to pass to
    // `setChannelId`, so it is built as one rather than converted at the end.
    let id = JObject::from(env.new_string("floom").e()?);
    let name = JObject::from(env.new_string("Floom").e()?);

    // `IMPORTANCE_DEFAULT` = 3. On pre-26 this field does not exist on
    // `NotificationManager`, so the class is probed first.
    if let Ok(channel_class) = env.find_class(jni::jni_str!("android/app/NotificationChannel")) {
        let channel = env
            .new_object(
                &channel_class,
                jni::jni_sig!("(Ljava/lang/String;Ljava/lang/CharSequence;I)V"),
                &[JValue::Object(&id), JValue::Object(&name), JValue::Int(3)],
            )
            .e()?;
        if notification.sound {
            env.call_method(
                &channel,
                jni::jni_str!("enableVibration"),
                jni::jni_sig!("(Z)V"),
                &[JValue::Bool(true)],
            )
            .e()?;
        }
        // `createNotificationChannel` returns `void`, so only the error path
        // carries anything.
        env.call_method(
            &manager,
            jni::jni_str!("createNotificationChannel"),
            jni::jni_sig!("(Landroid/app/NotificationChannel;)V"),
            &[JValue::Object(&channel)],
        )
        .e()?;
    }
    Ok(id)
}
