//! OpenHarmony notifications (stub, no API yet).

use super::Notification;

pub fn are_notifications_enabled() -> bool {
    false
}

pub fn request_authorization() -> Result<(), String> {
    Err("notifications: OpenHarmony currently does not provide a system notification API".into())
}

pub fn schedule(notification: &Notification) -> Result<(), String> {
    Err("notifications: OpenHarmony currently does not provide a system notification API".into())
}
