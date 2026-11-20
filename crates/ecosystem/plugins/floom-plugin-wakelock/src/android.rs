//! Android screen-awake via `WakelockPlugin`.
//!
//! The capability is Java: `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON` is
//! an Android API, and `WakelockPlugin` owns it. This file only carries the
//! ask across JNI.

use floom_platform_android::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `WakelockPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.wakelock.WakelockPlugin";

/// `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON` — a window flag, not a
/// power-manager wake lock.
const FLAG_KEEP_SCREEN_ON: i32 = 0x80;

pub fn keep_screen_on(on: bool) -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        env.call_static_method(
            &class,
            jni::jni_str!("keepScreenOn"),
            jni::jni_sig!("(Landroid/content/Context;Z)Z"),
            &[JValue::Object(&context), JValue::Bool(on)],
        )
        .e()?;
        Ok(())
    })
}

pub fn is_keep_screen_on() -> bool {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let ok = env
            .call_static_method(
                &class,
                jni::jni_str!("isKeepScreenOn"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        Ok(ok)
    })
    .unwrap_or(false)
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
