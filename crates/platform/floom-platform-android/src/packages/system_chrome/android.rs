// SPDX-License-Identifier: MIT
//! Android system-bar styling via `Window` / `WindowInsetsController`.
//!
//! Colors go through `Window.setStatusBarColor` / `setNavigationBarColor` and
//! icon brightness through `WindowInsetsController.setSystemBarsAppearance`.
//! The `WindowInsetsController` calls are API 30+; on older runtimes the lookup
//! fails and the legacy `View.setSystemUiVisibility` flags are used instead.

use jni::objects::JValue;
use jni::sys::jint;

use super::{BarIconBrightness, SystemChromeStyle};
use crate::jni::{self as jni_helpers, JniExt};

/// `APPEARANCE_LIGHT_STATUS_BARS` / `APPEARANCE_LIGHT_NAVIGATION_BARS`.
const LIGHT_STATUS_BARS: jint = 0x01;
const LIGHT_NAVIGATION_BARS: jint = 0x10;

/// Legacy `SYSTEM_UI_FLAG_LIGHT_STATUS_BAR` / `..._NAVIGATION_BAR`.
const FLAG_LIGHT_STATUS_BAR: jint = 0x0000_2000;
const FLAG_LIGHT_NAVIGATION_BAR: jint = 0x0000_1000;

pub fn set_system_chrome_style(style: SystemChromeStyle) -> Result<(), String> {
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

        if let Some(color) = style.status_bar_color {
            set_color(env, &window, jni::jni_str!("setStatusBarColor"), color)?;
        }
        if let Some(color) = style.navigation_bar_color {
            set_color(env, &window, jni::jni_str!("setNavigationBarColor"), color)?;
        }

        let mut appearance: jint = 0;
        if style.status_bar_icon_brightness == BarIconBrightness::Dark {
            appearance |= LIGHT_STATUS_BARS;
        }
        if style.navigation_bar_icon_brightness == BarIconBrightness::Dark {
            appearance |= LIGHT_NAVIGATION_BARS;
        }

        // API 30+: the controller is the supported route. A missing class or
        // method on older runtimes leaves a pending exception; clear it and
        // fall through to the legacy flags.
        let controller = env
            .call_method(
                &window,
                jni::jni_str!("getInsetsController"),
                jni::jni_sig!("()Landroid/view/WindowInsetsController;"),
                &[],
            )
            .and_then(|v| v.l())
            .ok();
        if env.exception_check() {
            env.exception_clear();
        }
        match controller {
            Some(controller) if !controller.is_null() => {
                env.call_method(
                    &controller,
                    jni::jni_str!("setSystemBarsAppearance"),
                    jni::jni_sig!("(II)V"),
                    &[
                        JValue::Int(appearance),
                        JValue::Int(LIGHT_STATUS_BARS | LIGHT_NAVIGATION_BARS),
                    ],
                )
                .e()?;
            }
            _ => set_legacy_icon_flags(env, &activity, appearance)?,
        }
        Ok(())
    })
}

fn set_color(
    env: &mut jni::Env,
    window: &jni::objects::JObject,
    method: &jni::strings::JNIStr,
    color: floom_types::Color,
) -> Result<(), String> {
    // Android colors are packed ARGB ints.
    let argb = i32::from_le_bytes([color.b, color.g, color.r, color.a]);
    env.call_method(window, method, jni::jni_sig!("(I)V"), &[JValue::Int(argb)])
        .e()?;
    Ok(())
}

fn set_legacy_icon_flags(
    env: &mut jni::Env,
    activity: &jni::objects::JObject,
    appearance: jint,
) -> Result<(), String> {
    let decor = env
        .call_method(
            activity,
            jni::jni_str!("getWindow"),
            jni::jni_sig!("()Landroid/view/Window;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    let decor = env
        .call_method(
            &decor,
            jni::jni_str!("getDecorView"),
            jni::jni_sig!("()Landroid/view/View;"),
            &[],
        )
        .and_then(|v| v.l())
        .e()?;
    let current: jint = env
        .call_method(
            &decor,
            jni::jni_str!("getSystemUiVisibility"),
            jni::jni_sig!("()I"),
            &[],
        )
        .and_then(|v| v.i())
        .unwrap_or(0);
    let light_status = appearance & LIGHT_STATUS_BARS != 0;
    let light_nav = appearance & LIGHT_NAVIGATION_BARS != 0;
    let mut flags = current & !(FLAG_LIGHT_STATUS_BAR | FLAG_LIGHT_NAVIGATION_BAR);
    if light_status {
        flags |= FLAG_LIGHT_STATUS_BAR;
    }
    if light_nav {
        flags |= FLAG_LIGHT_NAVIGATION_BAR;
    }
    env.call_method(
        &decor,
        jni::jni_str!("setSystemUiVisibility"),
        jni::jni_sig!("(I)V"),
        &[JValue::Int(flags)],
    )
    .e()?;
    Ok(())
}
