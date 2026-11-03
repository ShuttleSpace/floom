//! Android haptics and vibration via `VibrationPlugin`.
//!
//! The capability is Java: `Vibrator`, `VibrationEffect` and the view
//! hierarchy are Android APIs, and `VibrationPlugin` owns them. This file
//! only carries the ask across JNI.

use super::HapticFeedback;
use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::JValue;

/// `VibrationPlugin`'s class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.vibration.VibrationPlugin";

pub fn vibrate(duration_ms: u32) -> Result<(), String> {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        env.call_static_method(
            &class,
            jni::jni_str!("vibrate"),
            jni::jni_sig!("(Landroid/content/Context;J)Z"),
            &[JValue::Object(&context), JValue::Long(duration_ms as i64)],
        )
        .e()?;
        Ok(())
    })
}

pub fn haptic_feedback(feedback: HapticFeedback) -> Result<(), String> {
    let constant: i32 = match feedback {
        HapticFeedback::Light => 1,     // VIRTUAL_KEY
        HapticFeedback::Medium => 1,    // VIRTUAL_KEY
        HapticFeedback::Heavy => 0,     // LONG_PRESS
        HapticFeedback::Selection => 3, // KEYBOARD_TAP
        HapticFeedback::Success => 1,   // VIRTUAL_KEY
        HapticFeedback::Warning => 0,   // LONG_PRESS
        HapticFeedback::Error => 0,     // LONG_PRESS
    };

    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        env.call_static_method(
            &class,
            jni::jni_str!("hapticFeedback"),
            jni::jni_sig!("(Landroid/content/Context;I)Z"),
            &[JValue::Object(&context), JValue::Int(constant)],
        )
        .e()?;
        Ok(())
    })
}

pub fn can_vibrate() -> bool {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let ok = env
            .call_static_method(
                &class,
                jni::jni_str!("hasVibrator"),
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
