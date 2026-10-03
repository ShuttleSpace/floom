//! Soft keyboard visibility and height.
//!
//! On mobile, a software keyboard covers part of the screen. Apps must avoid
//! that region and react to height changes without jitter. The pure
//! [`KeyboardHeightFilter`] applies hysteresis to raw OS height reports;
//! [`keyboard_insets`] / [`is_keyboard_visible`] read the live state on
//! iOS/Android (host returns zero / false).
//!
//! The native arms live in `android.rs` / `ios.rs`; the real values come from
//! the platform window backend (`floom-platform-{android,ios}`), which owns
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

use floom_types::Insets;

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

/// Text the IME handed to the platform since the last read.
///
/// The distinction is the IME's, not ours: a *composition* is text the IME is
/// still working on (underline visible in the keyboard's candidate strip) and
/// may replace or drop, while *committed* text is finished and belongs to the
/// document. An editor that treats the first as the second will show characters
/// that then change or vanish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeInput {
    /// Finished text. Append it.
    Commit(String),
    /// The composition in progress. Replaces any previous preedit; empty clears it.
    Preedit(String),
}

/// Split a field's raw contents into `(committed, composing)`.
///
/// `start`/`end` are the IME's composing span in **UTF-16 code units** — that is
/// what Android's `Spannable.getComposingSpanStart/End` report, because they
/// index a Java `String`. Rust strings are UTF-8, so this converts rather than
/// slicing directly: the two agree only while everything is ASCII, and getting
/// that wrong splits a multi-byte character in half. (For a `qwerty` IME on a
/// `TextField` this is invisible; for a CJK IME, where every keystroke commits
/// multi-byte text, it is every keystroke.)
///
/// `start < 0` (Android's "no composition") or any out-of-range / non-boundary
/// span degrades to "everything is committed, nothing is composing" — a
/// composition we cannot locate must not cost the user their text.
pub fn split_composition(text: &str, start: i32, end: i32) -> (String, String) {
    if start < 0 || end < start {
        return (text.to_string(), String::new());
    }
    let (Some(start), Some(end)) = (
        utf16_offset_to_byte(text, start as usize),
        utf16_offset_to_byte(text, end as usize),
    ) else {
        return (text.to_string(), String::new());
    };

    let mut committed = String::with_capacity(text.len() - (end - start));
    committed.push_str(&text[..start]);
    committed.push_str(&text[end..]);
    (committed, text[start..end].to_string())
}

/// The part of `committed` that was not there before, or `None` when the text
/// was not extended but rewritten.
///
/// An editor can only *append* what the IME commits. A rewrite (deletion, a
/// replacement, the IME clearing its buffer) is not an append and has to be
/// handled by whoever owns the document — on Android deletions arrive as key
/// events, so the honest answer here is `None` rather than a guess.
pub fn newly_committed<'a>(previous: &str, committed: &'a str) -> Option<&'a str> {
    committed.strip_prefix(previous)
}

/// Byte offset of the `utf16`-th UTF-16 code unit, or `None` past the end.
fn utf16_offset_to_byte(text: &str, utf16: usize) -> Option<usize> {
    let mut units = 0usize;
    for (byte, ch) in text.char_indices() {
        if units == utf16 {
            return Some(byte);
        }
        units += ch.len_utf16();
        if units > utf16 {
            // The offset landed inside a surrogate pair: not a boundary.
            return None;
        }
    }
    (units == utf16).then_some(text.len())
}

/// Read whatever the IME has delivered since the last call.
///
/// Empty on the host (no IME) and on iOS (the anchor's text is not read back
/// yet) — the same shape as [`soft_keyboard_state`], so callers need no `cfg`.
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

    #[test]
    fn no_composition_means_everything_is_committed() {
        // -1 is Android's "no composing span".
        assert_eq!(
            split_composition("hello", -1, -1),
            ("hello".to_string(), String::new())
        );
    }

    #[test]
    fn a_trailing_composition_is_split_off() {
        let (committed, preedit) = split_composition("hello wor", 6, 9);
        assert_eq!(committed, "hello ");
        assert_eq!(preedit, "wor");
    }

    #[test]
    fn a_composition_in_the_middle_is_removed_from_the_committed_text() {
        let (committed, preedit) = split_composition("abXcd", 2, 3);
        assert_eq!(committed, "abcd");
        assert_eq!(preedit, "X");
    }

    /// The regression this exists for: a CJK IME commits multi-byte text, and
    /// Java's span is in UTF-16 units while a Rust `&str` is UTF-8 bytes. Doing
    /// the slice in bytes would cut "你好吗" in the middle of a character.
    #[test]
    fn utf16_offsets_are_converted_not_used_as_bytes() {
        // "你好" is 2 code units but 6 bytes. Composing the trailing "吗"
        // (1 unit, 3 bytes) starts at unit 2 — which is byte 6.
        let (committed, preedit) = split_composition("你好吗", 2, 3);
        assert_eq!(committed, "你好");
        assert_eq!(preedit, "吗");
    }

    #[test]
    fn an_offset_inside_a_surrogate_pair_degrades_to_all_committed() {
        // "🦀" is one char, two code units. Unit 1 is not a boundary.
        let (committed, preedit) = split_composition("a🦀", 1, 2);
        assert_eq!(committed, "a🦀");
        assert_eq!(preedit, "");
    }

    #[test]
    fn an_out_of_range_span_degrades_to_all_committed() {
        assert_eq!(
            split_composition("abc", 1, 99),
            ("abc".to_string(), String::new())
        );
        assert_eq!(
            split_composition("abc", 2, 1),
            ("abc".to_string(), String::new())
        );
    }

    #[test]
    fn only_an_appended_commit_counts_as_new() {
        assert_eq!(newly_committed("hello", "hello world"), Some(" world"));
        // Nothing new: no event to send.
        assert_eq!(newly_committed("hello", "hello"), Some(""));
        // Rewritten, not appended — the key path owns this.
        assert_eq!(newly_committed("hello", "hell"), None);
        assert_eq!(newly_committed("hello", "HELLO"), None);
    }

    #[test]
    fn host_ime_input_is_empty() {
        assert!(take_ime_input().is_empty());
    }
}
