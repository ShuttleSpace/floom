//! iOS soft-keyboard state.
//!
//! TODO(floem): read the real height/visibility from the
//! `keyboardWillShow` / `keyboardWillHide` notifications (or
//! `UIResponder.inputAccessoryView`) in the `floem-platform-ios` backend.

use super::SoftKeyboardState;

pub fn soft_keyboard_state() -> SoftKeyboardState {
    SoftKeyboardState::default()
}
