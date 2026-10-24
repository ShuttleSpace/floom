//! iOS local notifications via `UNUserNotificationCenter`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Reading the authorization status synchronously
//!
//! `UNUserNotificationCenter` only exposes its settings through
//! `getNotificationSettingsWithCompletionHandler:`. That does not make the
//! question unanswerable from a synchronous API — the handler runs on
//! UserNotifications' own queue, not on the main thread, so a dispatch
//! semaphore can wait for it without deadlocking. That is what
//! [`are_notifications_enabled`] does: it blocks the *calling* thread until the
//! handler has delivered `UNAuthorizationStatus`, and returns the real value.
//!
//! The one rule that keeps this honest: the wait is bounded. A handler that
//! never runs (a suspended daemon) must not hang the app forever, so the
//! semaphore is acquired with a timeout and a timeout reports `false` rather
//! than a stale `true`.
//!
//! # The prompt is requested, not awaited
//!
//! `requestAuthorizationWithOptions:completionHandler:` puts a system prompt on
//! screen. The completion block is delivered later, on UserNotifications' own
//! queue, so the prompt does not need the main thread blocked — but a
//! synchronous function still has nowhere to *return* a value that arrives
//! afterwards.
//!
//! So [`request_authorization`] issues the request and returns `Ok` meaning "the
//! OS has been asked". It does **not** mean "the user agreed": the caller polls
//! [`are_notifications_enabled`], which reads the real status. Returning `Ok`
//! for a grant would be the lie this plugin family exists to avoid; returning
//! `Err` for a prompt that *did* appear would be the opposite error, and was
//! what this arm did before.
//!
//! The completion block is a no-op rather than a semaphore wait: blocking here
//! would only be waiting for an answer the function cannot return anyway.

use super::Notification;
use block2::RcBlock;
use dispatch2::{DispatchSemaphore, DispatchTime};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::CString;

// `UNUserNotificationCenter` and the request classes are reached through
// `objc_getClass`, which only finds them if UserNotifications is loaded.
#[link(name = "UserNotifications", kind = "framework")]
unsafe extern "C" {}

/// `UNAuthorizationStatus`.
///
/// Only the three values that mean "the app may post" are named: `notDetermined`
/// and `denied` both fold into the `false` that an unmatched value already
/// produces, so naming them would be two constants nothing reads.
const UNAUTHORIZATION_STATUS_AUTHORIZED: i64 = 2;
const UNAUTHORIZATION_STATUS_PROVISIONAL: i64 = 3;
const UNAUTHORIZATION_STATUS_EPHEMERAL: i64 = 4;

/// How long to wait for the settings handler.
///
/// The handler is delivered on a system queue and normally runs in well under a
/// millisecond; five seconds is long enough that a timeout means the daemon is
/// not responding, not that the machine is busy.
const SETTINGS_TIMEOUT_SECONDS: i64 = 5;

pub fn are_notifications_enabled() -> bool {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    unsafe {
        let center: *mut AnyObject =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return false;
        }

        // `1` so the handler's `signal` releases the waiter.
        let semaphore = DispatchSemaphore::new(0);
        // The block owns one reference to the semaphore and this function owns
        // the other: `DispatchRetained` is reference-counted and `Clone`, so the
        // handler can outlive the closure's capture without a lifetime dance.
        let handler_semaphore = semaphore.clone();
        // The answer travels through an `Arc<AtomicBool>` captured by the block
        // rather than a process-wide static: two threads asking at once would
        // otherwise read each other's result, and the flag would outlive the
        // call that owns it.
        let enabled = Arc::new(AtomicBool::new(false));
        let writer = Arc::clone(&enabled);

        let block = RcBlock::new(move |settings: *mut AnyObject| {
            if settings.is_null() {
                handler_semaphore.signal();
                return;
            }
            let status: i64 = msg_send![settings, authorizationStatus];
            // `provisional` and `ephemeral` both mean the app may post without
            // the user having answered a prompt: provisional is the quiet
            // "deliver to Notification Center" grant, ephemeral is the App Clip
            // grant. Both are "enabled" for the caller's purposes.
            let is_enabled = matches!(
                status,
                UNAUTHORIZATION_STATUS_AUTHORIZED
                    | UNAUTHORIZATION_STATUS_PROVISIONAL
                    | UNAUTHORIZATION_STATUS_EPHEMERAL
            );
            writer.store(is_enabled, Ordering::Release);
            handler_semaphore.signal();
        });

        let _: () = msg_send![
            center,
            getNotificationSettingsWithCompletionHandler: &*block
        ];

        // A timeout means the settings never arrived; reporting `false` is the
        // pessimistic answer, and the caller can retry.
        let timeout = match DispatchTime::try_from(std::time::Duration::from_secs(
            SETTINGS_TIMEOUT_SECONDS as u64,
        )) {
            Ok(timeout) => timeout,
            // Unreachable for a five-second interval, but a `None` here must not
            // turn into an unbounded wait.
            Err(()) => return false,
        };
        match semaphore.try_acquire(timeout) {
            Ok(guard) => {
                guard.release();
                enabled.load(Ordering::Acquire)
            }
            Err(_) => false,
        }
    }
}

pub fn request_authorization() -> Result<(), String> {
    unsafe {
        let center: *mut AnyObject =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return Err("notifications: no UNUserNotificationCenter".into());
        }

        // `UNAuthorizationOptions`: alert (1) | sound (2) | badge (4). The same
        // set the Android arm's channel asks for.
        let options: u64 = 1 | 2 | 4;

        // The block must exist — the parameter is non-null — but its answer is
        // exactly what this function cannot return, so it does nothing. A
        // semaphore wait here would block for a value that is then dropped.
        //
        // `BOOL` is a `signed char`, not a Rust `bool`, so the parameter is
        // `objc2::runtime::Bool` — the Rust `bool` is one byte and would not
        // match the block's encoding.
        let block = RcBlock::new(|_granted: objc2::runtime::Bool, _error: *mut AnyObject| {});
        let _: () = msg_send![
            center,
            requestAuthorizationWithOptions: options,
            completionHandler: &*block
        ];
        Ok(())
    }
}

pub fn schedule(notification: &Notification) -> Result<(), String> {
    if notification.title.is_empty() {
        return Err("notifications: a notification needs a title".into());
    }
    unsafe {
        let content = notification_content(notification)?;
        let request = notification_request(content, notification)?;
        let center: *mut AnyObject =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return Err("notifications: no UNUserNotificationCenter".into());
        }
        // `addNotificationRequest:withCompletionHandler:` with a null block:
        // the handler would only report submission errors, and there is nothing
        // the caller could do with one from here.
        let _: () = msg_send![
            center,
            addNotificationRequest: request,
            withCompletionHandler: std::ptr::null_mut::<AnyObject>()
        ];
        Ok(())
    }
}

/// A configured `UNMutableNotificationContent`.
unsafe fn notification_content(notification: &Notification) -> Result<*mut AnyObject, String> {
    let content: *mut AnyObject = msg_send![class!(UNMutableNotificationContent), new];
    if content.is_null() {
        return Err("notifications: could not create the content object".into());
    }
    // `ns_string` and `default_sound` are `unsafe fn`s of this module; edition
    // 2024 requires the calls inside an `unsafe fn` to say so.
    unsafe {
        let title = ns_string(&notification.title)?;
        let _: () = msg_send![content, setTitle: title];
        let body = ns_string(&notification.body)?;
        let _: () = msg_send![content, setBody: body];

        if let Some(badge) = notification.badge {
            // `badge` is an `NSNumber`, so it is boxed rather than passed as an
            // integer — `setBadge:` takes an object.
            let number: *mut AnyObject =
                msg_send![class!(NSNumber), numberWithUnsignedInt: badge as u32];
            let _: () = msg_send![content, setBadge: number];
        }
        if notification.sound {
            let _: () = msg_send![content, setSound: default_sound()];
        }
    }
    Ok(content)
}

/// `+[UNNotificationSound defaultSound]`.
unsafe fn default_sound() -> *mut AnyObject {
    msg_send![class!(UNNotificationSound), defaultSound]
}

/// A `UNNotificationRequest` with an immediate or delayed trigger.
unsafe fn notification_request(
    content: *mut AnyObject,
    notification: &Notification,
) -> Result<*mut AnyObject, String> {
    // A stable identifier means a second call with the same content replaces the
    // pending request instead of queuing a duplicate — the same rule the
    // Android arm applies with its tag.
    let identifier = unsafe { ns_string("floom-notification")? };

    let trigger: *mut AnyObject = match notification.delay_seconds {
        None => std::ptr::null_mut(),
        Some(seconds) => {
            // `UNTimeIntervalNotificationTrigger` refuses a non-positive
            // interval at runtime, so the floor is one second rather than zero.
            let interval = (seconds.max(1)) as f64;
            let fires_repeating = false;
            let trigger: *mut AnyObject = msg_send![
                class!(UNTimeIntervalNotificationTrigger),
                triggerWithTimeInterval: interval,
                repeats: fires_repeating
            ];
            if trigger.is_null() {
                return Err("notifications: could not create the time trigger".into());
            }
            trigger
        }
    };

    let request: *mut AnyObject = msg_send![
        class!(UNNotificationRequest),
        requestWithIdentifier: identifier,
        content: content,
        trigger: trigger
    ];
    if request.is_null() {
        return Err("notifications: could not create the notification request".into());
    }
    Ok(request)
}

/// `+[NSString stringWithUTF8String:]`.
unsafe fn ns_string(value: &str) -> Result<*mut AnyObject, String> {
    let c = CString::new(value)
        .map_err(|_| "notifications: a field contains a NUL byte".to_string())?;
    let string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
    if string.is_null() {
        return Err("notifications: could not create an NSString".into());
    }
    Ok(string)
}
