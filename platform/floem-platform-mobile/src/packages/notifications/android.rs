//! Android notifications via `NotificationManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::Notification;

pub fn are_notifications_enabled() -> bool {
    // TODO: `NotificationManager.areNotificationsEnabled()` (API 24+).
    false
}

pub fn request_authorization() -> Result<(), String> {
    // TODO: launch the system notification-permission intent. Not satisfiable
    // synchronously here.
    Err("notification permission requires an Android prompt".into())
}

pub fn schedule(_notification: &Notification) -> Result<(), String> {
    // TODO: build a `NotificationCompat.Builder` + `NotificationManager.notify`
    // (or `AlarmManager` for delayed delivery).
    Err("notifications require an Android notification channel".into())
}
