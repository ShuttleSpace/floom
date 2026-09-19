//! Local notifications — permission, enablement and scheduling.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `UNUserNotificationCenter` (iOS) and `NotificationManager` (Android). The
//! host arm refuses every call that needs a real notification pipeline.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

/// A local notification to display or schedule.
#[derive(Debug, Clone, Default)]
pub struct Notification {
    /// Title text.
    pub title: String,
    /// Body text.
    pub body: String,
    /// Badge count to apply to the app icon (`None` = leave unchanged).
    pub badge: Option<u32>,
    /// Whether to play the default notification sound.
    pub sound: bool,
    /// If set, schedule the notification this many seconds from now instead of
    /// delivering it immediately.
    pub delay_seconds: Option<u64>,
}

/// Whether the app is allowed to present notifications.
pub fn are_notifications_enabled() -> bool {
    #[cfg(target_os = "android")]
    {
        android::are_notifications_enabled()
    }
    #[cfg(target_os = "ios")]
    {
        ios::are_notifications_enabled()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        false
    }
}

/// Request permission to present notifications.
pub fn request_authorization() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::request_authorization()
    }
    #[cfg(target_os = "ios")]
    {
        ios::request_authorization()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        Err("notifications are only available on iOS and Android".into())
    }
}

/// Display or schedule a notification.
pub fn schedule(notification: &Notification) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::schedule(notification)
    }
    #[cfg(target_os = "ios")]
    {
        ios::schedule(notification)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = notification;
        Err("notifications are only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_reports_disabled() {
        assert!(!are_notifications_enabled());
    }

    #[test]
    fn host_requests_refused() {
        assert!(request_authorization().is_err());
        let n = Notification {
            title: "hi".into(),
            body: "there".into(),
            ..Default::default()
        };
        assert!(schedule(&n).is_err());
    }
}
