// SPDX-License-Identifier: MIT
//! iOS soft-keyboard state (host-driven; current impl is a stub).

#[cfg(target_os = "ios")]
mod ios;

#[cfg(target_os = "ios")]
pub use ios::{push_ime_input, set_keyboard_state};

pub use floom_platform::mobile::{
    ImeInput, KeyboardHeightFilter, SoftKeyboardState, newly_committed, split_composition,
};
use floom_types::Insets;

pub fn soft_keyboard_state() -> SoftKeyboardState {
    #[cfg(target_os = "ios")]
    {
        ios::soft_keyboard_state()
    }
    #[cfg(not(target_os = "ios"))]
    {
        SoftKeyboardState::default()
    }
}

pub fn keyboard_insets() -> Insets {
    soft_keyboard_state().insets()
}

pub fn is_keyboard_visible() -> bool {
    soft_keyboard_state().visible
}

pub fn take_ime_input() -> Vec<ImeInput> {
    #[cfg(target_os = "ios")]
    {
        ios::take_ime_input()
    }
    #[cfg(not(target_os = "ios"))]
    {
        Vec::new()
    }
}

pub fn set_soft_keyboard_visible(visible: bool) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        ios::set_soft_keyboard_visible(visible)
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = visible;
        Err("soft keyboard control is only available on iOS and Android".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_defaults_hidden() {
        assert!(!is_keyboard_visible());
        assert_eq!(keyboard_insets(), Insets::ZERO);
    }
}
