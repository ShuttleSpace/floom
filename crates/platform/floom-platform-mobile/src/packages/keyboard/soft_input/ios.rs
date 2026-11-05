// SPDX-License-Identifier: MIT
//! iOS soft-keyboard state.
//!
//! TODO(floom): read the real height/visibility from the
//! `keyboardWillShow` / `keyboardWillHide` notifications (or
//! `UIResponder.inputAccessoryView`) in the `floom-platform-ios` backend.

use super::SoftKeyboardState;

pub fn soft_keyboard_state() -> SoftKeyboardState {
    SoftKeyboardState::default()
}
