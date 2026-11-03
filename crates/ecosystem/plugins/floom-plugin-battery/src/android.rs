//! Android battery state, read by `BatteryPlugin.java`.
//!
//! The capability is Java: `BatteryManager` and `Settings.Global` are Android
//! APIs, and the Java half is where the platform's own rules live — the sticky
//! broadcast, the level/scale pair, the saver-mode key. This file only carries
//! the answer across JNI, so the Rust half stays the same size on every
//! platform.

use super::BatteryState;
use floom_platform_mobile::jni::{self as bridge, JniExt};
use jni::objects::{JDoubleArray, JValue};

/// `BatteryPlugin`'s class, as `ClassLoader.loadClass` expects it. It must match
/// the package and class name under `android/src/main/java/`.
const CLASS: &str = "dev.floom.plugin.battery.BatteryPlugin";

pub fn battery_state() -> BatteryState {
    bridge::with_env(read_state).unwrap_or_default()
}

pub fn is_low_power_mode() -> bool {
    bridge::with_env(|env| {
        let class = class(env)?;
        let context = bridge::activity(env)?;
        let low_power = env
            .call_static_method(
                &class,
                jni::jni_str!("isLowPowerMode"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        Ok(low_power)
    })
    .unwrap_or(false)
}

fn read_state(env: &mut jni::Env<'_>) -> Result<BatteryState, String> {
    let class = class(env)?;
    let context = bridge::activity(env)?;
    let snapshot = env
        .call_static_method(
            &class,
            jni::jni_str!("snapshot"),
            jni::jni_sig!("(Landroid/content/Context;)[D"),
            &[JValue::Object(&context)],
        )
        .e()?
        .l()
        .e()?;

    // An empty array is the Java half's "nothing measured", which is a
    // different answer from a battery at some fraction.
    let snapshot: JDoubleArray = env.cast_local::<JDoubleArray>(snapshot).e()?;
    let len = snapshot.len(env).e()?;
    if len < 3 {
        return Ok(BatteryState::default());
    }

    let mut values = [0.0f64; 3];
    snapshot.get_region(env, 0, &mut values).e()?;
    Ok(BatteryState {
        level: Some((values[0] as f32).clamp(0.0, 1.0)),
        is_charging: values[1] != 0.0,
        is_full: values[2] != 0.0,
    })
}

fn class<'local>(env: &mut jni::Env<'local>) -> Result<jni::objects::JClass<'local>, String> {
    bridge::app_class(env, CLASS)
}
