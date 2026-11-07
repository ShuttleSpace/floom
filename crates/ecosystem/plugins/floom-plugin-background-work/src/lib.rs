//! Ask the OS to bring this app back at a time.
//!
//! This is the catalog's A8 (`docs/plugin-catalog.md`), which merged
//! `android_alarm_manager_plus` and `workmanager` into one crate on the
//! theory that both answer "make some code run later". Writing the arm
//! found a harder truth, and the crate is the honest narrowing of it.
//!
//! # What an app in this shape can actually be promised
//!
//! The example apps here are `android:hasCode="false"` `NativeActivity`
//! builds: no Java classes of their own. That closes every Flutter-shaped
//! door at once —
//!
//! * an **alarm callback** needs a `<receiver>` in the manifest and Java
//!   code for the system to call;
//! * **WorkManager** needs a `Worker` class, which is Java code;
//! * **iOS `BGTaskScheduler`** needs a task identifier in the `Info.plist`
//!   and an Objective-C handler registered at startup — the Shape-B native
//!   half this plugin family deliberately does not carry
//!   (`docs/plugin-catalog.md` §8.1).
//!
//! The one primitive left standing needs **no Java code at all**: an
//! `AlarmManager` alarm whose payload is a `PendingIntent` that starts the
//! activity itself. The system holds the intent, survives the process, and
//! at the appointed time brings the app up. That is
//! [`schedule_app_launch`], and its semantics are exactly what the name
//! says — the *app's own lifecycle is the callback*: the activity comes to
//! the front (or is recreated), and the app's startup path runs.
//!
//! # What `Ok` means, and what the alarm is worth
//!
//! `Ok(())` claims the **system accepted the ask** — the "true launch"
//! category from the catalog's verification matrix. The alarm uses
//! `setAndAllowWhileIdle`, which is *inexact*: the OS batches alarms to
//! save power, so "at 9:00" is really "around 9:00, in Doze possibly
//! noticeably later". Exact alarms (`setExactAndAllowWhileIdle`) need the
//! `SCHEDULE_EXACT_ALARM` permission on API 31+, which is the app's
//! manifest to write — the crate does not quietly require it. A time
//! already in the past fires immediately; that is the platform's behaviour
//! and this crate reports it rather than second-guessing it.
//!
//! # The notifications recipe
//!
//! This is what un-blocks `floom-plugin-notifications`' Android delayed
//! arm, which refuses because posting *now* at the wrong time is worse
//! than not posting. The recipe, in the app's own code:
//!
//! 1. `schedule_app_launch(at)` — the system brings the app back at `at`;
//! 2. before scheduling, persist "a notification is owed at `at`" (a
//!    `floom-plugin-prefs` row, or the app's own store);
//! 3. in startup, check the row and post the notification *immediately*
//!    (delay `None`), then clear it.
//!
//! The user sees the app come up and the notification with it — heavier
//! than a silent post, and that is the honest cost of a no-Java build.
//!
//! # What this crate deliberately does not do
//!
//! No opportunistic background execution (the workmanager question): the
//! Android and iOS halves are the native classes above, and pretending to
//! a `submit()` that cannot run work would be a stub with a launch button.
//! No in-process timers: a closure that runs later while the process
//! lives is `floom-scheduler`'s job, and calling it "background work"
//! would be the dishonest version of this crate.

use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "android")]
mod android;

/// Ask the OS to start this app at `at`.
///
/// Android only — and that is the honest scope, not a bias: see the crate
/// doc for what iOS and the desktops each say when asked, because their
/// refusals name the platform's actual position rather than a TODO.
///
/// Idempotent in the scheduling sense: a second call *replaces* the first
/// alarm (the intent is the same, so the system reschedules it), and
/// [`cancel_scheduled_app_launch`] withdraws it.
pub fn schedule_app_launch(at: SystemTime) -> Result<(), String> {
    // The epoch conversion is shared by the one arm that has a use for it,
    // so the validation lives here: every platform's refusal is checked
    // before dispatch too, and a nonsense time is nonsense everywhere.
    let millis = at
        .duration_since(UNIX_EPOCH)
        .map_err(|_| format!("schedule_app_launch: {at:?} is before the Unix epoch"))?
        .as_millis() as i64;

    #[cfg(target_os = "android")]
    {
        return android::schedule_app_launch(millis);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = millis;
        #[cfg(target_os = "ios")]
        let reason = "schedule_app_launch: iOS offers an app no way to wake or launch \
                      itself at a chosen time — BGTaskScheduler is opportunistic (the \
                      system decides when, and it cannot launch UI) and timers die with \
                      the process. The nearest capability is a user notification with a \
                      trigger, which is the notifications plugin's own scheduling arm";
        #[cfg(target_env = "ohos")]
        let reason = "schedule_app_launch: OpenHarmony has no scheduled-launch bridge \
                      in this crate yet";
        #[cfg(not(any(target_os = "ios", target_env = "ohos")))]
        let reason = "schedule_app_launch: the session manager owns sleep and wake — \
                      work that must run while this app is closed is a system service \
                      (launchd, a systemd timer, Task Scheduler), which is the app's \
                      deployment concern, not a plugin call";
        Err(reason.to_string())
    }
}

/// Withdraw the ask made by [`schedule_app_launch`].
///
/// Cancelling when nothing is scheduled is `Ok(())`: after the call no
/// alarm is pending, which is the whole contract.
pub fn cancel_scheduled_app_launch() -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        return android::cancel_scheduled_app_launch();
    }
    #[cfg(not(target_os = "android"))]
    {
        // The same platform positions as scheduling: iOS cannot hold an ask
        // to cancel, and the desktops never accepted one.
        #[cfg(target_os = "ios")]
        let reason = "cancel_scheduled_app_launch: iOS has no scheduled-launch ask to \
                      withdraw (see schedule_app_launch)";
        #[cfg(target_env = "ohos")]
        let reason = "cancel_scheduled_app_launch: OpenHarmony has no scheduled-launch \
                      ask to withdraw (see schedule_app_launch)";
        #[cfg(not(any(target_os = "ios", target_env = "ohos")))]
        let reason = "cancel_scheduled_app_launch: the desktops never accepted a \
                      scheduled-launch ask (see schedule_app_launch)";
        Err(reason.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On every non-Android target, the refusal names the platform's actual
    /// position — the iOS one says what the system *can* do instead, the
    /// desktop one says whose job the work is. A refusal that said only
    /// "not supported" would hide both.
    #[cfg(not(target_os = "android"))]
    #[test]
    fn the_refusal_names_the_platforms_position() {
        let err = schedule_app_launch(SystemTime::now()).unwrap_err();
        #[cfg(target_os = "ios")]
        assert!(err.contains("BGTaskScheduler"), "{err}");
        #[cfg(target_env = "ohos")]
        assert!(err.contains("OpenHarmony"), "{err}");
        #[cfg(not(any(target_os = "ios", target_env = "ohos")))]
        assert!(err.contains("system service"), "{err}");
    }

    /// A time before the epoch is nonsense on every platform, and the
    /// nonsense is caught before any platform is reached.
    #[test]
    fn a_time_before_the_epoch_is_refused_everywhere() {
        let before = UNIX_EPOCH - std::time::Duration::from_secs(1);
        let err = schedule_app_launch(before).unwrap_err();
        assert!(err.contains("before the Unix epoch"), "{err}");
    }
}
