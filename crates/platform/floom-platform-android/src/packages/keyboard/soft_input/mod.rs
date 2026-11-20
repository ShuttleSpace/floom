// SPDX-License-Identifier: MIT
//! Android soft-keyboard control and state.
//!
//! The pure types (`SoftKeyboardState`, `ImeInput`, `KeyboardHeightFilter`,
//! `split_composition`, …) live in `floom_platform::mobile`; this module owns
//! the Android implementation and re-exports the shared types so callers do
//! not need to name both crates.

#[cfg(target_os = "android")]
mod android;

pub use floom_platform::mobile::{
    ImeInput, KeyboardHeightFilter, SoftKeyboardState, newly_committed, split_composition,
};
use floom_types::Insets;

/// Live soft-keyboard state on Android; default (hidden) on the host.
pub fn soft_keyboard_state() -> SoftKeyboardState {
    #[cfg(target_os = "android")]
    {
        android::soft_keyboard_state()
    }
    #[cfg(not(target_os = "android"))]
    {
        SoftKeyboardState::default()
    }
}

/// Bottom insets covering the soft keyboard, or [`Insets::ZERO`] when hidden.
pub fn keyboard_insets() -> Insets {
    soft_keyboard_state().insets()
}

/// Whether the soft keyboard is currently visible.
pub fn is_keyboard_visible() -> bool {
    soft_keyboard_state().visible
}

/// Read whatever the IME has delivered since the last call.
pub fn take_ime_input() -> Vec<ImeInput> {
    #[cfg(target_os = "android")]
    {
        android::take_ime_input()
    }
    #[cfg(not(target_os = "android"))]
    {
        Vec::new()
    }
}

/// Ask the OS to show or hide the soft keyboard. No-op on the host.
pub fn set_soft_keyboard_visible(visible: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::set_visible(visible)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = visible;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_keyboard_state_is_hidden() {
        assert!(!is_keyboard_visible());
        assert_eq!(keyboard_insets(), Insets::ZERO);
        assert!(take_ime_input().is_empty());
    }
}
