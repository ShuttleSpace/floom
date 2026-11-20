// SPDX-License-Identifier: MIT
//! Android safe-area insets via the activity's `WindowInsets`.
//!
//! Reads the system-bar insets (status bar top, navigation bar bottom/left/
//! right) and the display cutout's safe insets from the decor view's
//! `WindowInsets`, converts device pixels to logical pixels using the display
//! density, and takes the larger of system-bar and cutout insets per side.
//! Any failure degrades to zero insets — a layout read must never stop a frame.

use floom_types::Insets;
use jni::objects::JValue;
use jni::sys::jint;

use crate::jni::{self as jni_helpers, JniExt};

pub fn safe_area_insets() -> Insets {
    jni_helpers::with_env(|env| {
        let activity = jni_helpers::activity(env)?;
        let window = env
            .call_method(
                &activity,
                jni::jni_str!("getWindow"),
                jni::jni_sig!("()Landroid/view/Window;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        let decor = env
            .call_method(
                &window,
                jni::jni_str!("getDecorView"),
                jni::jni_sig!("()Landroid/view/View;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        let insets = env
            .call_method(
                &decor,
                jni::jni_str!("getRootWindowInsets"),
                jni::jni_sig!("()Landroid/view/WindowInsets;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        if insets.is_null() {
            return Ok(Insets::ZERO);
        }

        // WindowInsets.Type.systemBars() returns the combined type mask, so a
        // platform renumbering does not break the getInsets call.
        let types = env
            .find_class(jni::jni_str!("android/view/WindowInsets$Type"))
            .e()?;
        let system_bars: jint = env
            .call_static_method(
                &types,
                jni::jni_str!("systemBars"),
                jni::jni_sig!("()I"),
                &[],
            )
            .and_then(|v| v.i())
            .e()?;
        let bars = env
            .call_method(
                &insets,
                jni::jni_str!("getInsets"),
                jni::jni_sig!("(I)Landroid/graphics/Insets;"),
                &[JValue::Int(system_bars)],
            )
            .and_then(|v| v.l())
            .e()?;

        let mut top = read_int_field(env, &bars, jni::jni_str!("top"))?;
        let mut right = read_int_field(env, &bars, jni::jni_str!("right"))?;
        let mut bottom = read_int_field(env, &bars, jni::jni_str!("bottom"))?;
        let mut left = read_int_field(env, &bars, jni::jni_str!("left"))?;

        // The camera cutout is not part of systemBars; read its safe insets and
        // take the larger value per side.
        let cutout = env
            .call_method(
                &insets,
                jni::jni_str!("getDisplayCutout"),
                jni::jni_sig!("()Landroid/view/DisplayCutout;"),
                &[],
            )
            .and_then(|v| v.l())
            .e()?;
        if !cutout.is_null() {
            top = top.max(call_int(env, &cutout, jni::jni_str!("getSafeInsetTop"))?);
            right = right.max(call_int(env, &cutout, jni::jni_str!("getSafeInsetRight"))?);
            bottom = bottom.max(call_int(env, &cutout, jni::jni_str!("getSafeInsetBottom"))?);
            left = left.max(call_int(env, &cutout, jni::jni_str!("getSafeInsetLeft"))?);
        }

        // Insets are device pixels; this package reports logical pixels.
        let density = display_density(env).unwrap_or(1.0);
        let d = |px: jint| f64::from(px) / density;
        Ok(Insets {
            top: d(top),
            right: d(right),
            bottom: d(bottom),
            left: d(left),
        })
    })
    .unwrap_or_default()
}

fn read_int_field(
    env: &mut jni::Env,
    obj: &jni::objects::JObject,
    name: &jni::strings::JNIStr,
) -> Result<jint, String> {
    if obj.is_null() {
        return Ok(0);
    }
    env.get_field(obj, name, jni::jni_sig!("I"))
        .and_then(|v| v.i())
        .e()
}

fn call_int(
    env: &mut jni::Env,
    obj: &jni::objects::JObject,
    method: &jni::strings::JNIStr,
) -> Result<jint, String> {
    env.call_method(obj, method, jni::jni_sig!("()I"), &[])
        .and_then(|v| v.i())
        .e()
}

/// `activity.getResources().getDisplayMetrics().density`.
fn display_density(env: &mut jni::Env) -> Result<f64, String> {
    let activity = jni_helpers::activity(env)?;
    let resources = env
        .call_method(
            &activity,
            jni::jni_str!("getResources"),
            jni::jni_sig!("()Landroid/content/res/Resources;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    let metrics = env
        .call_method(
            &resources,
            jni::jni_str!("getDisplayMetrics"),
            jni::jni_sig!("()Landroid/util/DisplayMetrics;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    let density: f32 = env
        .get_field(&metrics, jni::jni_str!("density"), jni::jni_sig!("F"))
        .and_then(|v| v.f())
        .unwrap_or(1.0);
    Ok(f64::from(density))
}
