//! Android soft-keyboard control and state, via JNI.
//!
//! # Why not `ANativeActivity_showSoftInput`
//!
//! winit's `request_ime_update` ends up in the NDK's
//! `ANativeActivity_showSoftInput`, a thin wrapper over
//! `InputMethodManager.showSoftInput`. That call needs a view the IME can
//! target, and a `NativeActivity` has none. Measured on API 34, the request
//! travels the whole chain and is then refused:
//!
//! ```text
//! ImeTracker: com.example.floem2:… onRequestShow at ORIGIN_CLIENT_SHOW_SOFT_INPUT
//! InputMethodManager: Ignoring showSoftInput() as view=…NativeContentView is not served.
//! ImeTracker: com.example.floem2:… onFailed at PHASE_CLIENT_VIEW_SERVED
//! ```
//!
//! So the keyboard is driven the way Java apps do it: attach a minimal,
//! invisible, 1×1 `EditText` to the activity and ask `InputMethodManager` to
//! show the IME for *that* view. Flutter, Slint and `gpui-mobile` all use this
//! technique — a `NativeActivity` simply has no editable view for the framework
//! to serve.
//!
//! # State
//!
//! Once the anchor is attached, the keyboard's visibility and height are
//! readable from its root window insets (`WindowInsets.Type.ime()`), so
//! [`soft_keyboard_state`] polls instead of needing a Java-side listener.
//!
//! Polling is a stopgap, and the reason is worth naming: `android-activity`
//! raises `MainEvent::InsetsChanged` when the keyboard appears, but winit's
//! Android backend discards it (`winit-android/src/event_loop.rs`, `TODO:
//! handle Android InsetsChanged notification`). And although the activity
//! reports `adjustResize`, the `NativeActivity` window is *not* actually
//! resized for the keyboard, so no window event follows either. Until one of
//! those is fixed nothing tells an idle app that the keyboard moved — which is
//! the real gap, since an app that is not told cannot avoid the covered region.
//!
//! Measured on API 34: the reader returns `visible: true` with a height of
//! 336.4 logical px (883 device px at 2.625) while the keyboard is up, but only
//! when the app was woken for an unrelated reason.
//!
//! # Scope
//!
//! This drives the keyboard's *presence*, which is what `Ime::set_enabled`
//! promises. Delivering the resulting characters back to the app needs an
//! `InputConnection` (`commitText` / `getExtractedText`) and is **not**
//! implemented here; the anchor's text is never read.

use std::sync::Mutex;

use jni::objects::{Global, JObject, JValue};
use jni::sys::jint;

use super::SoftKeyboardState;
use crate::jni::{self as jni_helpers, JniExt, obj};

/// `Context.INPUT_METHOD_SERVICE`.
const INPUT_METHOD_SERVICE: &str = "input_method";

/// The anchor view, created on first use and kept for the process lifetime.
///
/// It has to be kept: the IME is shown *for this view*, so dropping the handle
/// would mean creating a fresh one on every show — a slow leak of views inside
/// the activity's content view. The activity's view hierarchy owns the view;
/// the global reference is how we address it again.
static IME_ANCHOR: Mutex<Option<Global<JObject<'static>>>> = Mutex::new(None);

/// Borrow the anchor, creating and attaching it on first use.
///
/// A *local* reference is returned even though the stored handle is global:
/// locals are what the shared JNI helpers own, and the helper that lends us the
/// env frees them when it returns. Wrapping the global directly would risk a
/// `DeleteGlobalRef` on drop.
fn anchor<'local>(
    env: &mut jni::Env<'local>,
    stored: &mut Option<Global<JObject<'static>>>,
) -> Result<JObject<'local>, String> {
    if stored.is_none() {
        let view = create_anchor(env)?;
        *stored = Some(env.new_global_ref(&view).e()?);
    }
    let global = stored.as_ref().expect("just stored");
    env.new_local_ref(global.as_obj()).e()
}

/// Build the hidden `EditText` and attach it to the activity.
///
/// ```java
/// EditText v = new EditText(activity);
/// v.setFocusable(true); v.setFocusableInTouchMode(true); v.setAlpha(0f);
/// activity.addContentView(v, new ViewGroup.LayoutParams(1, 1));
/// ```
fn create_anchor<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;

    let class = env
        .find_class(jni::jni_str!("android/widget/EditText"))
        .e()?;
    let view = env
        .new_object(
            &class,
            jni::jni_sig!("(Landroid/content/Context;)V"),
            &[obj(&activity)],
        )
        .e()?;

    // Focusable so `requestFocus` can make it the served view — that is the
    // whole point of the anchor. `alpha = 0` keeps it invisible; alpha does not
    // affect `isShown()`, which is what the IME checks.
    env.call_method(
        &view,
        jni::jni_str!("setFocusable"),
        jni::jni_sig!("(Z)V"),
        &[JValue::Bool(true)],
    )
    .e()?;
    env.call_method(
        &view,
        jni::jni_str!("setFocusableInTouchMode"),
        jni::jni_sig!("(Z)V"),
        &[JValue::Bool(true)],
    )
    .e()?;
    env.call_method(
        &view,
        jni::jni_str!("setAlpha"),
        jni::jni_sig!("(F)V"),
        &[JValue::Float(0.0)],
    )
    .e()?;

    // `addContentView` needs explicit layout params; without them the anchor
    // would be laid out full-screen.
    let params_class = env
        .find_class(jni::jni_str!("android/view/ViewGroup$LayoutParams"))
        .e()?;
    let params = env
        .new_object(
            &params_class,
            jni::jni_sig!("(II)V"),
            &[JValue::Int(1), JValue::Int(1)],
        )
        .e()?;
    env.call_method(
        &activity,
        jni::jni_str!("addContentView"),
        jni::jni_sig!("(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"),
        &[obj(&view), obj(&params)],
    )
    .e()?;

    Ok(view)
}

/// The `InputMethodManager`, or `Err` if the service is unavailable.
fn input_method_manager<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let activity = jni_helpers::activity(env)?;
    let service: JObject = env.new_string(INPUT_METHOD_SERVICE).e()?.into();
    let manager = env
        .call_method(
            &activity,
            jni::jni_str!("getSystemService"),
            jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[obj(&service)],
        )
        .and_then(|value| value.l())
        .e()?;
    if manager.is_null() {
        return Err("InputMethodManager service not available".into());
    }
    Ok(manager)
}

/// Show or hide the soft keyboard.
///
/// A `false` return from the framework means "nothing to show/hide right now"
/// — not an error for the caller, so the boolean results are discarded. `Err`
/// means a JNI call itself failed.
pub fn set_visible(visible: bool) -> Result<(), String> {
    let mut stored = IME_ANCHOR
        .lock()
        .map_err(|_| "IME anchor mutex poisoned".to_string())?;
    jni_helpers::with_env(|env| {
        let view = anchor(env, &mut stored)?;
        let manager = input_method_manager(env)?;

        if visible {
            // The IME follows the focused view, so focus before asking.
            env.call_method(
                &view,
                jni::jni_str!("requestFocus"),
                jni::jni_sig!("()Z"),
                &[],
            )
            .e()?;
            env.call_method(
                &manager,
                jni::jni_str!("showSoftInput"),
                jni::jni_sig!("(Landroid/view/View;I)Z"),
                &[obj(&view), JValue::Int(0)],
            )
            .e()?;
        } else {
            let token = env
                .call_method(
                    &view,
                    jni::jni_str!("getWindowToken"),
                    jni::jni_sig!("()Landroid/os/IBinder;"),
                    &[],
                )
                .and_then(|value| value.l())
                .e()?;
            env.call_method(
                &manager,
                jni::jni_str!("hideSoftInputFromWindow"),
                jni::jni_sig!("(Landroid/os/IBinder;I)Z"),
                &[obj(&token), JValue::Int(0)],
            )
            .e()?;
        }
        Ok(())
    })
}

/// Live keyboard state, read from the anchor's root window insets.
///
/// Returns the hidden default when the anchor does not exist yet (nothing has
/// asked for the keyboard) or when the state cannot be read — this is a poll
/// for layout, so a failure degrades to "keyboard hidden" rather than
/// propagating. That matches the host arm, which always reports hidden.
pub fn soft_keyboard_state() -> SoftKeyboardState {
    let stored = match IME_ANCHOR.lock() {
        Ok(stored) => stored,
        Err(_) => return SoftKeyboardState::default(),
    };
    if stored.is_none() {
        // No anchor ⇒ the keyboard was never requested, so it cannot be up.
        return SoftKeyboardState::default();
    }
    let view = match stored.as_ref() {
        Some(view) => view,
        None => return SoftKeyboardState::default(),
    };
    jni_helpers::with_env(|env| read_state(env, view)).unwrap_or_default()
}

fn read_state(
    env: &mut jni::Env,
    anchor: &Global<JObject<'static>>,
) -> Result<SoftKeyboardState, String> {
    let view = env.new_local_ref(anchor.as_obj()).e()?;
    let insets = env
        .call_method(
            &view,
            jni::jni_str!("getRootWindowInsets"),
            jni::jni_sig!("()Landroid/view/WindowInsets;"),
            &[],
        )
        .and_then(|value| value.l())
        .e()?;
    if insets.is_null() {
        // Not attached yet; the attach completes on the next layout pass.
        return Ok(SoftKeyboardState::default());
    }

    // `WindowInsets.Type.ime()` is a static method returning the type mask.
    // Calling it (rather than hard-coding the constant) keeps this correct if
    // the platform ever renumbers the types.
    let types = env
        .find_class(jni::jni_str!("android/view/WindowInsets$Type"))
        .e()?;
    let ime_type = env
        .call_static_method(&types, jni::jni_str!("ime"), jni::jni_sig!("()I"), &[])
        .and_then(|value| value.i())
        .e()?;

    let visible = env
        .call_method(
            &insets,
            jni::jni_str!("isVisible"),
            jni::jni_sig!("(I)Z"),
            &[JValue::Int(ime_type)],
        )
        .and_then(|value| value.z())
        .unwrap_or(false);
    if !visible {
        return Ok(SoftKeyboardState::default());
    }

    let geometry = env
        .call_method(
            &insets,
            jni::jni_str!("getInsets"),
            jni::jni_sig!("(I)Landroid/graphics/Insets;"),
            &[JValue::Int(ime_type)],
        )
        .and_then(|value| value.l())
        .e()?;
    let bottom: jint = env
        .get_field(&geometry, jni::jni_str!("bottom"), jni::jni_sig!("I"))
        .and_then(|value| value.i())
        .unwrap_or(0);

    // Insets are device pixels; this package reports logical pixels.
    let density = display_density(env).unwrap_or(1.0);
    Ok(SoftKeyboardState {
        visible: true,
        height: if density > 0.0 {
            f64::from(bottom) / density
        } else {
            f64::from(bottom)
        },
    })
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
        .and_then(|value| value.l())
        .e()?;
    let metrics = env
        .call_method(
            &resources,
            jni::jni_str!("getDisplayMetrics"),
            jni::jni_sig!("()Landroid/util/DisplayMetrics;"),
            &[],
        )
        .and_then(|value| value.l())
        .e()?;
    let density: f32 = env
        .get_field(&metrics, jni::jni_str!("density"), jni::jni_sig!("F"))
        .and_then(|value| value.f())
        .unwrap_or(1.0);
    Ok(f64::from(density))
}
