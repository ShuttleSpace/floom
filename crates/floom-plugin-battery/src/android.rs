//! Android battery state via `BatteryManager`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # Why this is a sticky broadcast and not a listener
//!
//! `ACTION_BATTERY_CHANGED` is a *sticky* broadcast: the system keeps the last
//! one around, and `registerReceiver(null, filter)` hands it back synchronously
//! without registering anything. That is exactly the shape this crate needs — a
//! snapshot call with no subscription and no callback — so no receiver is
//! registered and none has to be unregistered.
//!
//! The alternative, `BatteryManager.BATTERY_PROPERTY_CAPACITY`, looks simpler
//! but reports a *whole* percent and returns `Integer.MIN_VALUE` on devices that
//! do not implement the property. The sticky intent carries `EXTRA_LEVEL` and
//! `EXTRA_SCALE`, which is the same pair the system UI divides, so the level
//! this crate reports is the one the status bar shows.

use super::BatteryState;
use floom_platform_mobile::jni::{self as jni_helpers, JniExt};
use jni::objects::{JObject, JValue};

/// `BatteryManager.EXTRA_LEVEL` — the current charge, in units of `EXTRA_SCALE`.
const EXTRA_LEVEL: &str = "level";
/// `BatteryManager.EXTRA_SCALE` — the maximum the level can reach (normally 100).
const EXTRA_SCALE: &str = "scale";
/// `BatteryManager.EXTRA_PLUGGED` — how the device is powered; `0` means unplugged.
const EXTRA_PLUGGED: &str = "plugged";
/// `BatteryManager.EXTRA_STATUS` — charging state; `5` is `BATTERY_STATUS_FULL`.
const EXTRA_STATUS: &str = "status";

/// `BatteryManager.BATTERY_STATUS_FULL`.
const BATTERY_STATUS_FULL: i32 = 5;

pub fn battery_state() -> BatteryState {
    let measured = jni_helpers::with_env(read_state);
    // A failed read is reported as `level: None`, not as a full battery. The
    // crate's whole point is that an unmeasured value must not look measured.
    measured.unwrap_or_default()
}

pub fn is_low_power_mode() -> bool {
    jni_helpers::with_env(|env| {
        // `BATTERY_SAVER_MODE` is a `Settings.Global` int: `1` when the user (or
        // the system, below the threshold) has turned battery saver on. Reading
        // it needs no permission, unlike most of `Settings`.
        let resolver = content_resolver(env)?;
        let key = java_string(env, "battery_saver_mode")?;
        let value = env
            .call_method(
                &resolver,
                jni::jni_str!("getInt"),
                jni::jni_sig!("(Landroid/content/ContentResolver;Ljava/lang/String;I)I"),
                &[
                    JValue::Object(&resolver),
                    JValue::Object(&key),
                    JValue::Int(0),
                ],
            )
            .e()?;
        Ok(value.i().unwrap_or(0) == 1)
    })
    .unwrap_or(false)
}

/// Read the sticky `ACTION_BATTERY_CHANGED` intent and turn it into a state.
fn read_state(env: &mut jni::Env) -> Result<BatteryState, String> {
    let activity = jni_helpers::activity(env)?;

    // `IntentFilter(Intent.ACTION_BATTERY_CHANGED)` — the action is a constant on
    // `Intent`, so it is fetched rather than hard-coded, and a renamed constant
    // would fail loudly here instead of silently reading the wrong broadcast.
    let filter = new_intent_filter(env)?;
    let intent = env
        .call_method(
            &activity,
            jni::jni_str!("registerReceiver"),
            jni::jni_sig!(
                "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;"
            ),
            &[JValue::Object(&JObject::null()), JValue::Object(&filter)],
        )
        .e()?
        .l()
        .e()?;

    if intent.is_null() {
        // No sticky broadcast yet: the system has not posted one since boot, so
        // there is genuinely nothing measured to report.
        return Ok(BatteryState::default());
    }

    let level = extra_int(env, &intent, EXTRA_LEVEL)?;
    let scale = extra_int(env, &intent, EXTRA_SCALE)?;
    let plugged = extra_int(env, &intent, EXTRA_PLUGGED)?;
    let status = extra_int(env, &intent, EXTRA_STATUS)?;

    // A scale of zero would divide by zero; the system always sends a positive
    // one, and a missing one means the intent is malformed rather than that the
    // battery is at 0%.
    let level = match (level, scale) {
        (Some(level), Some(scale)) if scale > 0 => (level as f32 / scale as f32).clamp(0.0, 1.0),
        _ => return Ok(BatteryState::default()),
    };

    Ok(BatteryState {
        level: Some(level),
        // `plugged` is a bitmask (AC / USB / wireless), so any non-zero value
        // means power is flowing.
        is_charging: plugged.unwrap_or(0) != 0,
        is_full: status == Some(BATTERY_STATUS_FULL),
    })
}

/// `new IntentFilter(Intent.ACTION_BATTERY_CHANGED)`.
fn new_intent_filter<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let intent_class = env
        .find_class(jni::jni_str!("android/content/Intent"))
        .e()?;
    let action = env
        .get_static_field(
            &intent_class,
            jni::jni_str!("ACTION_BATTERY_CHANGED"),
            jni::jni_sig!("Ljava/lang/String;"),
        )
        .e()?
        .l()
        .e()?;
    let filter_class = env
        .find_class(jni::jni_str!("android/content/IntentFilter"))
        .e()?;
    env.new_object(
        &filter_class,
        jni::jni_sig!("(Ljava/lang/String;)V"),
        &[JValue::Object(&action)],
    )
    .e()
}

/// `intent.getIntExtra(name, default)`.
///
/// The name is a runtime string because the extras are addressed by string
/// constants; the signature is a literal because `jni_sig!` takes only literals.
fn extra_int(env: &mut jni::Env, intent: &JObject<'_>, name: &str) -> Result<Option<i32>, String> {
    // The key is built first: `call_method` already borrows `env` mutably, so
    // building the string inside the argument list would be a second borrow.
    let key = java_string(env, name)?;
    let value = env
        .call_method(
            intent,
            jni::jni_str!("getIntExtra"),
            jni::jni_sig!("(Ljava/lang/String;I)I"),
            &[JValue::Object(&key), JValue::Int(-1)],
        )
        .e()?;
    match value.i() {
        // `-1` is the sentinel passed above, so it means "absent" rather than
        // "the value is minus one" — no battery extra is ever negative.
        Ok(v) if v >= 0 => Ok(Some(v)),
        _ => Ok(None),
    }
}

/// The app's `ContentResolver`, via `Context.getContentResolver()`.
fn content_resolver<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    env.call_method(
        &activity,
        jni::jni_str!("getContentResolver"),
        jni::jni_sig!("()Landroid/content/ContentResolver;"),
        &[],
    )
    .e()?
    .l()
    .e()
}

/// A `java.lang.String` from a Rust string.
fn java_string<'local>(env: &mut jni::Env<'local>, value: &str) -> Result<JObject<'local>, String> {
    env.new_string(value).map(JObject::from).e()
}
