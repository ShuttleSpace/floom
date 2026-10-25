//! Android scheduled launch via `AlarmManager` + an activity `PendingIntent`.
//!
//! The Java sequence:
//!
//! ```java
//! Intent i = new Intent(activity, activity.getClass());
//! PendingIntent p = PendingIntent.getActivity(activity, 0, i,
//!                       FLAG_UPDATE_CURRENT | FLAG_IMMUTABLE);
//! AlarmManager am = (AlarmManager) activity.getSystemService("alarm");
//! am.setAndAllowWhileIdle(RTC_WAKEUP, atMillis, p);   // schedule
//! am.cancel(p);                                        // cancel
//! ```
//!
//! # Why an activity intent and not a broadcast
//!
//! A broadcast would need a `<receiver>` — Java code the `hasCode="false"`
//! example apps do not have and cannot declare behaviour for. An activity
//! `PendingIntent` is the one payload the system can deliver with no code
//! of ours: it starts the activity, and the app's own lifecycle is the
//! callback. The crate doc spells out what that costs — the app comes to
//! the front, visibly.
//!
//! # The flags
//!
//! `FLAG_IMMUTABLE` because API 23+ requires PendingIntent mutability to be
//! stated and an immutable one is what this crate means; `FLAG_UPDATE_CURRENT`
//! so a second schedule with the same intent *replaces* the first alarm
//! rather than being dropped as a duplicate. Cancelling rebuilds the same
//! intent, which is how `AlarmManager.cancel` matches the alarm to withdraw.
//!
//! # `setAndAllowWhileIdle`
//!
//! The inexact, Doze-tolerant alarm. It needs no permission; its exact
//! sibling would need `SCHEDULE_EXACT_ALARM` on API 31+, which is the app's
//! manifest to write. The method arrived in API 23 — on anything older the
//! call throws, the bridge surfaces it as the call's `Err`, and the message
//! is the platform's own.

use floom_platform_mobile::jni::{self as jni_helpers, JniExt, obj};
use jni::objects::{JObject, JValue};

/// `Context.ALARM_SERVICE`.
const ALARM_SERVICE: &str = "alarm";

/// `AlarmManager.RTC_WAKEUP` — wake the device to fire.
const RTC_WAKEUP: i32 = 0;

/// `PendingIntent.FLAG_UPDATE_CURRENT | FLAG_IMMUTABLE`.
const PENDING_FLAGS: i32 = 0x0800_0000 | 0x0400_0000;

pub fn schedule_app_launch(at_millis: i64) -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let (alarm, pending) = alarm_and_intent(env)?;
        env.call_method(
            &alarm,
            jni::jni_str!("setAndAllowWhileIdle"),
            jni::jni_sig!("(IJLandroid/app/PendingIntent;)V"),
            &[
                JValue::Int(RTC_WAKEUP),
                JValue::Long(at_millis),
                obj(&pending),
            ],
        )
        .e()?;
        Ok(())
    })
}

pub fn cancel_scheduled_app_launch() -> Result<(), String> {
    jni_helpers::with_env(|env| {
        let (alarm, pending) = alarm_and_intent(env)?;
        env.call_method(
            &alarm,
            jni::jni_str!("cancel"),
            jni::jni_sig!("(Landroid/app/PendingIntent;)V"),
            &[obj(&pending)],
        )
        .e()?;
        Ok(())
    })
}

/// The `AlarmManager` and the activity `PendingIntent` both calls need.
///
/// One builder serves schedule and cancel because they must agree on the
/// intent — `cancel` withdraws whatever `setAndAllowWhileIdle` filed for
/// *this* activity intent, and a rebuilt-but-equivalent intent matches.
fn alarm_and_intent<'env>(
    env: &mut jni::Env<'env>,
) -> Result<(JObject<'env>, JObject<'env>), String> {
    let activity = jni_helpers::activity(env)?;

    // `new Intent(activity, activity.getClass())` — an intent that starts
    // this very activity, which is the whole payload.
    let class = env
        .call_method(
            &activity,
            jni::jni_str!("getClass"),
            jni::jni_sig!("()Ljava/lang/Class;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    let intent_class = env
        .find_class(jni::jni_str!("android/content/Intent"))
        .e()?;
    let intent = env
        .new_object(
            &intent_class,
            jni::jni_sig!("(Landroid/content/Context;Ljava/lang/Class;)V"),
            &[obj(&activity), obj(&class)],
        )
        .e()?;

    let pending = {
        let pending_class = env
            .find_class(jni::jni_str!("android/app/PendingIntent"))
            .e()?;
        let value = env
            .call_static_method(
                &pending_class,
                jni::jni_str!("getActivity"),
                jni::jni_sig!(
                    "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;"
                ),
                &[obj(&activity), JValue::Int(0), obj(&intent), JValue::Int(PENDING_FLAGS)],
            )
            .and_then(|v| v.l())
            .e()?;
        if value.is_null() {
            return Err("schedule_app_launch: PendingIntent.getActivity returned null".into());
        }
        value
    };

    let alarm = {
        let service: JObject = env.new_string(ALARM_SERVICE).e()?.into();
        let manager = env
            .call_method(
                &activity,
                jni::jni_str!("getSystemService"),
                jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                &[obj(&service)],
            )
            .and_then(|v| v.l())
            .e()?;
        if manager.is_null() {
            return Err("schedule_app_launch: no AlarmManager service".into());
        }
        manager
    };

    Ok((alarm, pending))
}
