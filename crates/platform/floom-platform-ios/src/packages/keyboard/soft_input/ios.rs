// SPDX-License-Identifier: MIT
//! iOS soft keyboard and IME state, supplied by the host through a C ABI seam.
//!
//! UIKit delivers `keyboardWillShow/Hide` notifications and `UITextInput`
//! text to the host's view controller, not to Rust. The host forwards them
//! through the `floom_ios_*` functions in the crate root; Rust holds the last
//! reported state and drains IME text on demand. Until the host does, the
//! state is hidden and the input queue empty.

use std::ffi::CStr;
use std::ffi::c_char;
use std::sync::Mutex;

use super::{ImeInput, SoftKeyboardState};

static KEYBOARD: Mutex<SoftKeyboardState> = Mutex::new(SoftKeyboardState {
    visible: false,
    height: 0.0,
});
static IME_QUEUE: Mutex<Vec<ImeInput>> = Mutex::new(Vec::new());

pub fn soft_keyboard_state() -> SoftKeyboardState {
    KEYBOARD.lock().map(|g| *g).unwrap_or_default()
}

/// Host entry point: publish keyboard visibility and height (logical points).
pub fn set_keyboard_state(visible: bool, height: f64) {
    if let Ok(mut guard) = KEYBOARD.lock() {
        *guard = SoftKeyboardState {
            visible,
            height: if visible { height.max(0.0) } else { 0.0 },
        };
    }
}

pub fn take_ime_input() -> Vec<ImeInput> {
    IME_QUEUE
        .lock()
        .map(|mut q| q.drain(..).collect())
        .unwrap_or_default()
}

/// Host entry point: enqueue text the IME committed or is composing.
/// `text` must be a NUL-terminated UTF-8 string valid for the call.
pub fn push_ime_input(committed: bool, text: *const c_char) {
    if text.is_null() {
        return;
    }
    let s = unsafe { CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned();
    let item = if committed {
        ImeInput::Commit(s)
    } else {
        ImeInput::Preedit(s)
    };
    if let Ok(mut q) = IME_QUEUE.lock() {
        q.push(item);
    }
}

pub fn set_soft_keyboard_visible(_visible: bool) -> Result<(), String> {
    Err("iOS soft keyboard control requires the host's first-responder path (not wired yet)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrip_and_normalisation() {
        set_keyboard_state(true, 300.0);
        assert_eq!(soft_keyboard_state().height, 300.0);
        set_keyboard_state(false, 300.0);
        assert_eq!(soft_keyboard_state().height, 0.0);
    }

    #[test]
    fn ime_queue_drains() {
        let c = std::ffi::CString::new("你好").unwrap();
        push_ime_input(true, c.as_ptr());
        assert_eq!(take_ime_input(), vec![ImeInput::Commit("你好".into())]);
        assert!(take_ime_input().is_empty());
    }
}
