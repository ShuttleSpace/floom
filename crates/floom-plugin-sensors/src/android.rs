//! Android sensor availability via `SensorManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Why `getDefaultSensor` per kind
//!
//! `getSensorList(TYPE_ALL)` returns every sensor the device has, which on a
//! modern phone is a long list of vendor-specific parts — several accelerometers,
//! a handful of "wakeup" duplicates, and composite sensors that report derived
//! values. Matching those back onto six crate-level kinds means deciding which
//! of three accelerometers is *the* accelerometer. `getDefaultSensor(type)` is
//! the platform's own answer to that question: it returns the sensor the system
//! would hand an app that asked for that type, or `null` when the device has
//! none. That is exactly the fact this crate reports.
//!
//! # The host test
//!
//! The crate's host test asserts that a desktop reports no sensors. That is not
//! a statement about this module — it only holds on the desktop arm. On a device
//! every kind whose hardware exists comes back `true`, which is what the gallery
//! example prints.

use super::SensorKind;
use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

/// `android.hardware.Sensor` type constants.
///
/// These are `static final int`s on `Sensor`, stable since API 1 (barometer
/// since API 3, light since API 3), so they are named here rather than fetched
/// through JNI: a lookup that fails would turn "the device has no barometer"
/// into "the device has no barometer", which is the same answer for the wrong
/// reason and would hide a broken bridge.
const TYPE_ACCELEROMETER: i32 = 1;
const TYPE_MAGNETIC_FIELD: i32 = 2;
const TYPE_GYROSCOPE: i32 = 4;
const TYPE_LIGHT: i32 = 5;
const TYPE_PRESSURE: i32 = 6;
const TYPE_PROXIMITY: i32 = 8;

pub fn available_sensors() -> Vec<SensorKind> {
    SensorKind::all()
        .iter()
        .copied()
        .filter(|kind| is_sensor_available(*kind))
        .collect()
}

pub fn is_sensor_available(kind: SensorKind) -> bool {
    jni_helpers::with_env(|env| Ok(has_sensor(env, sensor_type(kind))?)).unwrap_or(false)
}

/// The `Sensor.TYPE_*` constant for a crate kind.
fn sensor_type(kind: SensorKind) -> i32 {
    match kind {
        SensorKind::Accelerometer => TYPE_ACCELEROMETER,
        SensorKind::Gyroscope => TYPE_GYROSCOPE,
        SensorKind::Magnetometer => TYPE_MAGNETIC_FIELD,
        SensorKind::Barometer => TYPE_PRESSURE,
        SensorKind::Proximity => TYPE_PROXIMITY,
        SensorKind::AmbientLight => TYPE_LIGHT,
    }
}

/// `SensorManager.getDefaultSensor(type) != null`.
fn has_sensor(env: &mut jni::Env, sensor_type: i32) -> Result<bool, String> {
    let manager = sensor_manager(env)?;
    let sensor = env
        .call_method(
            &manager,
            jni::jni_str!("getDefaultSensor"),
            jni::jni_sig!("(I)Landroid/hardware/Sensor;"),
            &[JValue::Int(sensor_type)],
        )
        .e()?
        .l()
        .e()?;
    // `getDefaultSensor` returns `null` when no sensor of that type exists; it
    // does not throw, so a null here is the platform's own "not present".
    Ok(!sensor.is_null())
}

/// `Context.getSystemService(SensorManager.class)`.
fn sensor_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let class = env
        .find_class(jni::jni_str!("android/hardware/SensorManager"))
        .e()?;
    env.call_method(
        &activity,
        jni::jni_str!("getSystemService"),
        jni::jni_sig!("(Ljava/lang/Class;)Ljava/lang/Object;"),
        &[JValue::Object(&JObject::from(class))],
    )
    .e()?
    .l()
    .e()
}
