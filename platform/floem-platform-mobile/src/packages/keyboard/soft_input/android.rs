//! Android soft-keyboard state.
//!
//! TODO(floem): read the real height/visibility from `WindowInsets` (the
//! `ime()` / `visibleBounds` of the soft input) via the JNI bridge in the
//! actual `floem-platform-android` window backend.

use super::SoftKeyboardState;

pub fn soft_keyboard_state() -> SoftKeyboardState {
    SoftKeyboardState::default()
}
