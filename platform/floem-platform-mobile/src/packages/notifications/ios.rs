//! iOS notifications via `UNUserNotificationCenter`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::Notification;

pub fn are_notifications_enabled() -> bool {
    // TODO: `UNUserNotificationCenter.getNotificationSettings()` and inspect
    // `authorizationStatus` (async, via the completion block).
    false
}

pub fn request_authorization() -> Result<(), String> {
    // TODO: `requestAuthorization(options:)` async via the completion block.
    Err("notification permission requires an iOS prompt".into())
}

pub fn schedule(_notification: &Notification) -> Result<(), String> {
    // TODO: build an `UNNotificationRequest` (with `UNCalendarNotificationTrigger`
    // for delayed delivery) and add it to `UNUserNotificationCenter`.
    Err("notifications require an iOS notification center".into())
}
