//! Android scheduled launch via `BackgroundWorkPlugin`.
//!
//! The capability is Java: `AlarmManager` and the activity `PendingIntent`
//! are Android APIs, and `BackgroundWorkPlugin` owns them. This file only
//! carries the ask across JNI.

use floom_platform_android::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `BackgroundWorkPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.background_work.BackgroundWorkPlugin";

/// `AlarmManager.RTC_WAKEUP` — wake the device to fire.
const RTC_WAKEUP: i32 = 0;

/// `PendingIntent.FLAG_UPDATE_CURRENT | FLAG_IMMUTABLE`.
const PENDING_FLAGS: i32 = 0x0800_0000 | 0x0400_0000;

pub fn schedule_app_launch(at_millis: i64) -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let ok = env
            .call_static_method(
                &class,
                jni::jni_str!("scheduleAppLaunch"),
                jni::jni_sig!("(Landroid/content/Context;J)Z"),
                &[JValue::Object(&context), JValue::Long(at_millis)],
            )
            .e()?
            .z()
            .e()?;
        if ok {
            Ok(())
        } else {
            Err("background-work: scheduleAppLaunch returned false".into())
        }
    })
}

pub fn cancel_scheduled_app_launch() -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let ok = env
            .call_static_method(
                &class,
                jni::jni_str!("cancelScheduledAppLaunch"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        if ok {
            Ok(())
        } else {
            Err("background-work: cancelScheduledAppLaunch returned false".into())
        }
    })
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
