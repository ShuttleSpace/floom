#![cfg(target_env = "ohos")]

//! OpenHarmony soft-keyboard / IME bridge (N-API).
//!
//! Mirrors `platform::ios_ime` but routes through the shared N-API platform
//! bridge instead of a native hidden text field. On OHOS the soft keyboard and
//! candidate UI live in ArkTS (`@kit.IMEKit` / input method extension); Rust
//! only shows/hides the keyboard and receives committed / composed text back
//! through N-API callbacks.
//!
//! Events flow in both directions:
//!
//! * **Rust → ArkTS** (`set_keyboard_visible`): calls `showKeyboard` /
//!   `hideKeyboard` on the registered bridge object.
//! * **ArkTS → Rust** (`on_ime_commit` / `on_ime_preedit` / `on_ime_delete` /
//!   `on_ime_composition_end`): the ArkTS input-method extension pushes edits
//!   into a global queue that the window update loop drains and injects as
//!   [`crate::event::Event::Ime`], exactly like the accessibility-action queue.

use std::sync::Mutex;

use napi::bindgen_prelude::JsFunction;
use napi_derive::napi;

use crate::embedder::ImePurpose;
use crate::event::ImeEvent;
use crate::platform::ohos_bridge;

/// Queue of IME edits produced by ArkTS and drained by the window update loop.
///
/// OHOS is single-window, so a single global queue is sufficient. (If OHOS
/// ever supports multiple `Window`s, this must become per-window — see
/// `docs/ohos.md` "single-window assumption".)
static IME_QUEUE: Mutex<Vec<ImeEvent>> = Mutex::new(Vec::new());

/// Show or hide the OHOS soft keyboard for the active window.
///
/// Called from [`crate::window::WindowHandle`] when a `text_input` gains or
/// loses focus (via the `SetImeAllowed` update message). A no-op if the ArkTS
/// bridge has not been registered yet.
pub(crate) fn set_keyboard_visible(visible: bool) {
    let method = if visible { "showKeyboard" } else { "hideKeyboard" };
    ohos_bridge::call_bridge(|_env, obj| {
        let func: JsFunction = obj.get_named_property(method)?;
        // `showKeyboard` / `hideKeyboard` take no arguments.
        func.call_without_args(None)?;
        Ok(())
    });
}

/// Forward the IME caret / candidate rectangle to ArkTS so the soft-keyboard
/// candidate window can avoid overlapping the caret.
///
/// Called from [`crate::window::WindowHandle`]'s `SetImeCursorArea` update
/// handler (OHOS only). A no-op if the ArkTS bridge has not been registered
/// yet. Coordinates are logical pixels; they are rounded to integers because
/// OHOS `InputMethodController.updateCursor` takes integer device pixels.
pub(crate) fn set_ime_cursor_area(x: f64, y: f64, width: f64, height: f64) {
    ohos_bridge::call_bridge(|env, obj| {
        let func: JsFunction = obj.get_named_property("updateImeCursorArea")?;
        let args = [
            env.create_int64(x as i64)?.into_unknown(),
            env.create_int64(y as i64)?.into_unknown(),
            env.create_int64(width as i64)?.into_unknown(),
            env.create_int64(height as i64)?.into_unknown(),
        ];
        let _ = func.call(None, &args)?;
        Ok(())
    });
}

/// Forward the semantic IME purpose (text / password / number / …) to ArkTS so
/// the soft keyboard can adapt (secure entry, numeric keypad, …).
///
/// Called from [`crate::window::WindowHandle`]'s `SetImePurpose` update handler
/// (OHOS only). The purpose is encoded as a stable `i64` discriminant
/// (`ImePurpose` variant order) so ArkTS can map it back without a string
/// round-trip.
pub(crate) fn set_ime_purpose(purpose: ImePurpose) {
    let code: i64 = match purpose {
        ImePurpose::Text => 0,
        ImePurpose::Password => 1,
        ImePurpose::Number => 2,
        ImePurpose::Email => 3,
        ImePurpose::Url => 4,
        ImePurpose::Search => 5,
    };
    ohos_bridge::call_bridge(|env, obj| {
        let func: JsFunction = obj.get_named_property("updateImeConfig")?;
        let arg = env.create_int64(code)?.into_unknown();
        let _ = func.call(None, &[arg])?;
        Ok(())
    });
}

/// Drain all queued IME events. Called once per frame from the window update
/// loop (alongside the accessibility-action drain).
pub(crate) fn drain_ime_events() -> Vec<ImeEvent> {
    let mut q = IME_QUEUE.lock().unwrap();
    std::mem::take(&mut *q)
}

fn push_event(event: ImeEvent) {
    IME_QUEUE.lock().unwrap().push(event);
}

// --- ArkTS → Rust callbacks (exported into the N-API module) -----------
//
// The ArkTS input-method extension calls these when the user types. They are
// ordinary `#[napi]` exports, so `cargo-ohos-app` surfaces them as C symbols
// in `libfloem.so` that the ArkTS side imports.

/// Final text committed (composition finished, or a single key press / paste).
#[napi]
pub fn on_ime_commit(text: String) {
    push_event(ImeEvent::Commit(text));
}

/// Composition (preedit / marked) text updated. `cursor_start` / `cursor_end`
/// are byte offsets within `text`; negative values mean "no cursor".
#[napi]
pub fn on_ime_preedit(text: String, cursor_start: i64, cursor_end: i64) {
    let cursor = if cursor_start >= 0 && cursor_end >= 0 {
        Some((cursor_start as usize, cursor_end as usize))
    } else {
        None
    };
    push_event(ImeEvent::Preedit { text, cursor });
}

/// Composition finished without a following commit (the IME cancelled the
/// marked text). Mapped to `ImeEvent::Disabled`, matching the established
/// "composition end" semantics in `embedder/window_ext.rs`.
#[napi]
pub fn on_ime_composition_end() {
    push_event(ImeEvent::Disabled);
}

/// Surrounding text deleted around the cursor (`before` / `after` are byte
/// counts; negative values are clamped to 0).
#[napi]
pub fn on_ime_delete(before: i64, after: i64) {
    push_event(ImeEvent::DeleteSurrounding {
        before_bytes: before.max(0) as usize,
        after_bytes: after.max(0) as usize,
    });
}
