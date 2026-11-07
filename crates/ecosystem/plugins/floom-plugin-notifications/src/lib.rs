//! Local notifications — permission, enablement and scheduling.
//!
//! Adapted from `gpui-mobile` (Apache-2.0). The native arms read
//! `UNUserNotificationCenter` (iOS and macOS) and `NotificationManager`
//! (Android). Windows and Linux refuse and name the door: WinRT's toasts need
//! an app identity, and Linux has no cross-desktop API (a freedesktop
//! notification daemon speaks D-Bus).
//!
//! A **capability plugin**: one capability, one crate, opt-in by
//! dependency. The SDK does not carry it — an app that wants this
//! capability adds this crate, and an app that does not, does not link
//! it.
//!
//! Where an arm is not wired yet, the call reports that plainly rather
//! than returning a value that looks measured.

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;
#[cfg(target_env = "ohos")]
mod ohos;

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
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        apple::are_notifications_enabled()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::are_notifications_enabled()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
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
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        apple::request_authorization()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::request_authorization()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        Err(unsupported())
    }
}

/// Display or schedule a notification.
pub fn schedule(notification: &Notification) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::schedule(notification)
    }
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        apple::schedule(notification)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::schedule(notification)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    )))]
    {
        let _ = notification;
        Err(unsupported())
    }
}

#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_env = "ohos"
)))]
fn unsupported() -> String {
    if cfg!(target_os = "windows") {
        "notifications: Windows toasts need an app identity (a Start-menu shortcut \
         with an AppUserModelID) and the WinRT `windows` crate; this arm is not written"
            .into()
    } else {
        "notifications: Linux has no cross-desktop notification API; a freedesktop \
         notification daemon speaks D-Bus (`org.freedesktop.Notifications`), which \
         this arm has not implemented"
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_without_a_notification_pipeline_reports_disabled() {
        assert!(!are_notifications_enabled());
    }

    #[test]
    fn a_host_without_a_notification_pipeline_refuses_to_post() {
        assert!(request_authorization().is_err());
        let n = Notification {
            title: "hi".into(),
            body: "there".into(),
            ..Default::default()
        };
        assert!(schedule(&n).is_err());
    }

    /// A bare `cargo test` binary is not inside a `.app`, so this is the
    /// unbundled refusal — the same one a plain `cargo run` gets on macOS.
    #[cfg(target_os = "macos")]
    #[test]
    fn an_unbundled_macos_process_says_why_it_cannot_post() {
        let err = request_authorization().unwrap_err();
        assert!(err.contains("bundle identifier"), "{err}");
    }
}
