// SPDX-License-Identifier: MIT
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
//! ImeTracker: com.example.floom2:… onRequestShow at ORIGIN_CLIENT_SHOW_SOFT_INPUT
//! InputMethodManager: Ignoring showSoftInput() as view=…NativeContentView is not served.
//! ImeTracker: com.example.floom2:… onFailed at PHASE_CLIENT_VIEW_SERVED
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
//! promises, **and reads back what the IME produced into it** — see
//! [`take_ime_input`]. The characters never travel through winit: the anchor is
//! a real `EditText`, so the framework already gave it an `InputConnection` and
//! the IME's `commitText`/`setComposingText` land in its `Editable` on their
//! own. Reading that (rather than hand-building an `InputConnection`, which
//! needs a Java subclass and therefore a custom `Activity`) is the whole trick.
//!
//! It is a poll, for the same reason [`soft_keyboard_state`] is: nothing tells
//! us a character arrived. The cost is bounded the same way too — an idle app
//! gets no frames, so an idle app polls nothing.

use std::sync::Mutex;

use jni::objects::{Global, JObject, JString, JValue};
use jni::sys::jint;

use super::{ImeInput, SoftKeyboardState, newly_committed, split_composition};
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

/// What the last read found, split the way the IME means it.
///
/// Both halves are needed to answer "what is new": committed text grows, and a
/// preedit appears, changes, and then disappears — either by being committed or
/// by being abandoned, which look identical from the outside until you compare
/// against the previous preedit.
#[derive(Default)]
struct LastText {
    committed: String,
    preedit: String,
}

static LAST_TEXT: Mutex<LastText> = Mutex::new(LastText {
    committed: String::new(),
    preedit: String::new(),
});

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
            // Nothing is composing once the keyboard is gone, so start the next
            // session from an empty anchor. Without this, text left behind would
            // come back as a spurious commit on the next show.
            reset_text(env, &view)?;
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

/// Read whatever the IME has put into the anchor since the last call.
///
/// Returns nothing when the anchor does not exist (nothing ever asked for the
/// keyboard) or when the read fails, and never errors: this runs from the frame
/// loop, and a platform read that cannot be done is not a reason to stop a
/// frame. A failure is reported once per distinct error so a device run can see
/// it in logcat without flooding it.
pub fn take_ime_input() -> Vec<ImeInput> {
    let stored = match IME_ANCHOR.lock() {
        Ok(stored) => stored,
        Err(_) => return Vec::new(),
    };
    let Some(anchor) = stored.as_ref() else {
        return Vec::new();
    };

    let read = jni_helpers::with_env(|env| read_text(env, anchor));
    let (text, composing_start, composing_end) = match read {
        Ok(read) => read,
        Err(err) => {
            report_once(&err);
            return Vec::new();
        }
    };

    let (committed, preedit) = split_composition(&text, composing_start, composing_end);

    let mut last = match LAST_TEXT.lock() {
        Ok(last) => last,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();

    if committed != last.committed {
        // An append is a commit; a rewrite is not, and is left to the key path
        // (an Android delete arrives as `KEYCODE_DEL`, not as text). A field the
        // IME itself emptied (or `reset` below) counts as neither.
        if let Some(new_text) = newly_committed(&last.committed, &committed) {
            if !new_text.is_empty() {
                out.push(ImeInput::Commit(new_text.to_string()));
            }
        }
        last.committed = committed;
    }

    // A preedit is reported on every change *including its disappearance*: an
    // editor holding underlined text has to be told to drop it, and "the preedit
    // is now empty" is the only message that says so.
    if preedit != last.preedit {
        out.push(ImeInput::Preedit(preedit.clone()));
        last.preedit = preedit;
    }

    out
}

/// Drop the anchor's contents and forget what was read from it.
///
/// Called when the keyboard is hidden: with the keyboard gone nothing is
/// composing, and text left behind would otherwise be reported as a fresh
/// commit the next time the keyboard is shown.
fn reset_text(env: &mut jni::Env, view: &JObject) -> Result<(), String> {
    let empty = env.new_string("").e()?;
    env.call_method(
        view,
        jni::jni_str!("setText"),
        jni::jni_sig!("(Ljava/lang/CharSequence;)V"),
        &[obj(&empty)],
    )
    .e()?;
    if let Ok(mut last) = LAST_TEXT.lock() {
        last.committed.clear();
        last.preedit.clear();
    }
    Ok(())
}

/// `(text, composing_start, composing_end)` as the anchor reports them.
///
/// The span comes from `Spannable.getComposingSpanStart/End` — the same pair a
/// Java `TextWatcher` would consult. `-1` means "no composition", which
/// [`split_composition`] treats as "all committed".
fn read_text(
    env: &mut jni::Env,
    anchor: &Global<JObject<'static>>,
) -> Result<(String, i32, i32), String> {
    let view = env.new_local_ref(anchor.as_obj()).e()?;
    let editable = env
        .call_method(
            &view,
            jni::jni_str!("getText"),
            jni::jni_sig!("()Landroid/text/Editable;"),
            &[],
        )
        .and_then(|value| value.l())
        .e()?;
    if editable.is_null() {
        return Err("anchor has no Editable".into());
    }

    let as_string: JObject = env
        .call_method(
            &editable,
            jni::jni_str!("toString"),
            jni::jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|value| value.l())
        .e()?;
    if as_string.is_null() {
        return Err("anchor Editable is not a String".into());
    }
    // Turbofish is required: `cast_local` returns `To::Kind<'local>`, which
    // cannot be inferred from the binding.
    let string: JString = env.cast_local::<JString>(as_string).e()?;
    let text = string.try_to_string(env).e()?;

    // `BaseInputConnection` is where the composing span lives in the public
    // SDK: the same-named methods on `Spannable` are AOSP-internal and absent
    // from `android.jar` (`Method not found: getComposingSpanStart`, verified
    // on API 34). Static, so it is called on the class.
    let base = env
        .find_class(jni::jni_str!(
            "android/view/inputmethod/BaseInputConnection"
        ))
        .e()?;
    let span = |env: &mut jni::Env, name| -> Result<i32, String> {
        env.call_static_method(
            &base,
            name,
            jni::jni_sig!("(Landroid/text/Spannable;)I"),
            &[obj(&editable)],
        )
        .and_then(|value| value.i())
        .e()
    };
    let start = span(env, jni::jni_str!("getComposingSpanStart"))?;
    let end = span(env, jni::jni_str!("getComposingSpanEnd"))?;

    Ok((text, start, end))
}

/// Print an IME read failure once per distinct message.
fn report_once(err: &str) {
    use std::sync::OnceLock;
    static REPORTED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    let reported = REPORTED.get_or_init(|| Mutex::new(Vec::new()));
    let Ok(mut reported) = reported.lock() else {
        return;
    };
    if reported.iter().any(|seen| seen == err) {
        return;
    }
    reported.push(err.to_string());
    // stderr, like everything else here: logcat shows it as `RustStdoutStderr`
    // whatever the compile-time log level is.
    eprintln!("floom2-android: IME text read failed: {err}");
}
