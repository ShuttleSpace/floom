//! Soft keyboard visibility and height.
//!
//! On mobile, a software keyboard covers part of the screen. Apps must avoid
//! that region and react to height changes without jitter. The pure
//! [`KeyboardHeightFilter`] applies hysteresis to raw OS height reports;
//! [`keyboard_insets`] / [`is_keyboard_visible`] read the live state on
//! iOS/Android (host returns zero / false).
//!
//! The native arms live in `android.rs` / `ios.rs`; the real values come from
//! the platform window backend (`floem-platform-{android,ios}`), which owns
//! the `WindowInsets` / `keyboardWillShow` context.
//!
//! # Ported from
//!
//! Adapted from `gpui-mobile` (the soft-keyboard dirty-flag / height-lag notes),
//! used under its Apache-2.0 option.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;

use floem_types::Insets;

/// Stable keyboard state reported to the app.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SoftKeyboardState {
    /// Whether the soft keyboard is currently shown.
    pub visible: bool,
    /// Visible keyboard height in logical pixels (0 when not visible).
    pub height: f64,
}

impl SoftKeyboardState {
    /// The screen region the keyboard covers, as bottom insets.
    pub fn insets(&self) -> Insets {
        if self.visible && self.height > 0.0 {
            Insets {
                top: 0.0,
                right: 0.0,
                bottom: self.height,
                left: 0.0,
            }
        } else {
            Insets::ZERO
        }
    }
}

/// Hysteresis filter that collapses small / transient keyboard-height reports
/// into a stable value, preventing layout jitter during the show/hide
/// animation.
///
/// Feed each raw `height` report from the OS through [`push`](Self::push); it
/// returns the updated stable height only when the change is meaningful — a
/// visibility transition, or a move beyond `threshold` logical px. Reports
/// within the threshold (animation micro-steps, sensor noise) are ignored.
pub struct KeyboardHeightFilter {
    threshold: f64,
    current: f64,
}

impl KeyboardHeightFilter {
    /// A filter with the given hysteresis threshold (logical px).
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            current: 0.0,
        }
    }

    /// A filter with the default 8px threshold.
    pub fn with_default_threshold() -> Self {
        Self::new(8.0)
    }

    /// Current stable height.
    pub fn current(&self) -> f64 {
        self.current
    }

    /// Feed a raw reported height. Returns `Some(new_height)` when the stable
    /// height should update, otherwise `None`.
    pub fn push(&mut self, reported: f64) -> Option<f64> {
        let reported = reported.max(0.0);
        let visibility_changed = (self.current == 0.0) != (reported == 0.0);
        let moved = (reported - self.current).abs() >= self.threshold;
        if visibility_changed || moved {
            self.current = reported;
            Some(self.current)
        } else {
            None
        }
    }
}

/// Live soft-keyboard state. Host returns [`Default`] (not visible).
pub fn soft_keyboard_state() -> SoftKeyboardState {
    #[cfg(target_os = "android")]
    {
        android::soft_keyboard_state()
    }
    #[cfg(target_os = "ios")]
    {
        ios::soft_keyboard_state()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        SoftKeyboardState::default()
    }
}

/// Bottom insets covering the soft keyboard, or [`Insets::ZERO`] when hidden /
/// on the host.
pub fn keyboard_insets() -> Insets {
    soft_keyboard_state().insets()
}

/// Whether the soft keyboard is currently visible.
pub fn is_keyboard_visible() -> bool {
    soft_keyboard_state().visible
}

/// Ask the OS to show or hide the soft keyboard.
///
/// This is the *control* half of the package: [`soft_keyboard_state`] reads
/// what the OS reports, this changes it. On Android the keyboard is driven
/// through a hidden anchor view — see [`android`] for why the obvious NDK route
/// cannot work for a `NativeActivity`.
///
/// `Err` means the request could not be issued at all (the platform bridge is
/// missing or a JNI call failed), not that the OS declined: Android's
/// `showSoftInput` returns `false` for "nothing to show", which is not an error
/// for a caller that just asked for the keyboard.
///
/// Host builds have no soft keyboard, so there is nothing to control and the
/// call succeeds vacuously — that keeps callers free of `#[cfg]`.
pub fn set_soft_keyboard_visible(visible: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::set_visible(visible)
    }
    #[cfg(target_os = "ios")]
    {
        let _ = visible;
        Err("soft keyboard control is not implemented on iOS yet".to_string())
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = visible;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_commits_visibility_transition() {
        let mut f = KeyboardHeightFilter::with_default_threshold();
        // 0 -> 300 (keyboard appears): visibility change ⇒ commit.
        assert_eq!(f.push(300.0), Some(300.0));
        // Small animation jitter within threshold ⇒ no commit.
        assert_eq!(f.push(305.0), None);
        assert_eq!(f.push(298.0), None);
        // Large move beyond threshold ⇒ commit.
        assert_eq!(f.push(340.0), Some(340.0));
    }

    #[test]
    fn filter_commits_hide() {
        let mut f = KeyboardHeightFilter::with_default_threshold();
        f.push(300.0);
        // 300 -> 0 (keyboard hides): visibility change ⇒ commit to 0.
        assert_eq!(f.push(0.0), Some(0.0));
    }

    #[test]
    fn filter_ignores_sub_threshold_within_visible() {
        let mut f = KeyboardHeightFilter::with_default_threshold();
        f.push(300.0);
        assert_eq!(f.push(303.0), None);
        assert_eq!(f.push(297.0), None);
        assert_eq!(f.current(), 300.0);
    }

    #[test]
    fn host_keyboard_state_is_hidden() {
        assert!(!is_keyboard_visible());
        assert_eq!(keyboard_insets(), Insets::ZERO);
    }

    #[test]
    fn visible_state_insets_bottom_only() {
        let s = SoftKeyboardState {
            visible: true,
            height: 250.0,
        };
        let i = s.insets();
        assert_eq!(i.bottom, 250.0);
        assert_eq!(i.top, 0.0);
    }
}
