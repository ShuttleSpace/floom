//! Android local notifications via `NotificationsPlugin`.
//!
//! The capability is Java: `NotificationManager` and the notification builder
//! are Android APIs, and `NotificationsPlugin` owns them. This file only
//! carries the ask across JNI.
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
use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `NotificationsPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.notifications.NotificationsPlugin";

/// `Build.VERSION_CODES.TIRAMISU`, the API level that introduced the runtime
/// permission.
const API_TIRAMISU: i32 = 33;

pub fn are_notifications_enabled() -> bool {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let enabled = env
            .call_static_method(
                &class,
                jni::jni_str!("areEnabled"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        Ok(enabled)
    })
    .unwrap_or(false)
}

pub fn request_authorization() -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let asked = env
            .call_static_method(
                &class,
                jni::jni_str!("requestAuthorization"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        if !asked {
            // On API < 33 this is "notifications are switched off for this app";
            // on API >= 33 it is "no Activity to prompt from". Both are honest
            // failures to make progress, not a grant.
            return Err(
                "notifications: the permission could not be requested — either this \
                 device is below API 33 and the app's notifications are switched off \
                 in Settings, or there is no Activity context to prompt from"
                    .into(),
            );
        }
        Ok(())
    })
}

pub fn schedule(notification: &Notification) -> Result<(), String> {
    if notification.title.is_empty() {
        return Err("notifications: a notification needs a title".into());
    }
    if notification.delay_seconds.is_some() {
        return Err("notifications: delayed delivery needs AlarmManager (and \
             SCHEDULE_EXACT_ALARM on API 31+), which this plugin does not wire. \
             Pass delay_seconds: None to deliver immediately, or combine \
             floom-plugin-background-work::schedule_app_launch with a startup \
             check that posts immediately — the recipe is in that crate's docs"
            .into());
    }
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let title = env.new_string(&notification.title).e()?;
        let body = env.new_string(&notification.body).e()?;
        let badge = notification.badge.unwrap_or(0) as i32;
        let posted = env
            .call_static_method(
                &class,
                jni::jni_str!("schedule"),
                jni::jni_sig!("(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;IZ)Z"),
                &[
                    JValue::Object(&context),
                    JValue::Object(&title),
                    JValue::Object(&body),
                    JValue::Int(badge),
                    JValue::Bool(notification.sound),
                ],
            )
            .e()?
            .z()
            .e()?;
        if posted {
            Ok(())
        } else {
            Err("notifications: NotificationsPlugin.schedule reported failure".into())
        }
    })
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
