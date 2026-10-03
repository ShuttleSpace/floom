//! Android NDK key-code → Floom key mapping.
//!
//! This is the pure, platform-neutral half of Android keyboard support: it
//! translates an Android `AKeyEvent`'s integer key code and meta-state bitmask
//! into Floom's [`Key`]/[`Modifiers`]/[`KeyEvent`] types. It contains no JNI —
//! the Android platform backend (the real `floom-platform-android` window)
//! reads the raw `AKeyEvent` fields and calls these functions.
//!
//! The constant values match `<android/input.h>` / `android.view.KeyEvent`.
//! Only the subset Floom cares about is listed (the same scope gpui-mobile
//! covers).
//!
//! # Caveats vs the upstream table
//!
//! * Floom's [`Modifiers`] is a 4-bit set (`SHIFT`/`CONTROL`/`ALT`/`META`) and
//!   has **no** `FUNCTION` flag, so `AMETA_FUNCTION_ON` is dropped. Caps Lock
//!   is also not modelled — `AMETA_CAPS_LOCK_ON` does not surface.
//! * Floom's [`NamedKey`] does **not** include function keys `F1`–`F12`, so
//!   those key codes currently map to `None`. TODO(floom): extend `NamedKey`
//!   with `Function(u8)` and wire them here.
//! * Floom normalises letter keys to lower case and carries case in
//!   `Modifiers::SHIFT` (see `floom-types`). This table follows that rule:
//!   `AKEYCODE_A` → `Key::Char('a')`, and a shifted press adds `SHIFT`.
//!
//! # Ported from
//!
//! Adapted from [`gpui-mobile`](https://github.com/itsbalamurali/gpui-mobile)
//! (`src/android/keyboard.rs`), used under its Apache-2.0 option. The GPUI
//! `Keystroke`/`Modifiers` types were replaced with the Floom 2.0 equivalents
//! and the GPUI-trait `AndroidKeyboardLayout` was dropped (Floom has its own
//! keyboard-layout abstraction).

use floom_types::{Key, KeyEvent, KeyPhase, Modifiers, NamedKey};

// ── Android meta-state flags (from <android/input.h>) ────────────────────────

/// No meta keys pressed.
pub const AMETA_NONE: i32 = 0;
/// Shift key is pressed.
pub const AMETA_SHIFT_ON: i32 = 0x01;
/// Alt key is pressed.
pub const AMETA_ALT_ON: i32 = 0x02;
/// Ctrl key is pressed.
pub const AMETA_CTRL_ON: i32 = 0x1000;
/// Meta (Windows/Command) key is pressed.
pub const AMETA_META_ON: i32 = 0x10000;
/// Caps Lock is on (not modelled by Floom's `Modifiers`).
pub const AMETA_CAPS_LOCK_ON: i32 = 0x100000;
/// Function key is pressed (no Floom equivalent).
pub const AMETA_FUNCTION_ON: i32 = 0x08;

// ── Android key codes (from <android/keycodes.h>) ────────────────────────────
// Only the subset Floom cares about is listed here.

pub const AKEYCODE_UNKNOWN: i32 = 0;
pub const AKEYCODE_SOFT_LEFT: i32 = 1;
pub const AKEYCODE_SOFT_RIGHT: i32 = 2;
pub const AKEYCODE_HOME: i32 = 3;
pub const AKEYCODE_BACK: i32 = 4;
pub const AKEYCODE_0: i32 = 7;
pub const AKEYCODE_1: i32 = 8;
pub const AKEYCODE_2: i32 = 9;
pub const AKEYCODE_3: i32 = 10;
pub const AKEYCODE_4: i32 = 11;
pub const AKEYCODE_5: i32 = 12;
pub const AKEYCODE_6: i32 = 13;
pub const AKEYCODE_7: i32 = 14;
pub const AKEYCODE_8: i32 = 15;
pub const AKEYCODE_9: i32 = 16;
pub const AKEYCODE_STAR: i32 = 17;
pub const AKEYCODE_POUND: i32 = 18;
pub const AKEYCODE_DPAD_UP: i32 = 19;
pub const AKEYCODE_DPAD_DOWN: i32 = 20;
pub const AKEYCODE_DPAD_LEFT: i32 = 21;
pub const AKEYCODE_DPAD_RIGHT: i32 = 22;
pub const AKEYCODE_DPAD_CENTER: i32 = 23;
pub const AKEYCODE_A: i32 = 29;
pub const AKEYCODE_B: i32 = 30;
pub const AKEYCODE_C: i32 = 31;
pub const AKEYCODE_D: i32 = 32;
pub const AKEYCODE_E: i32 = 33;
pub const AKEYCODE_F: i32 = 34;
pub const AKEYCODE_G: i32 = 35;
pub const AKEYCODE_H: i32 = 36;
pub const AKEYCODE_I: i32 = 37;
pub const AKEYCODE_J: i32 = 38;
pub const AKEYCODE_K: i32 = 39;
pub const AKEYCODE_L: i32 = 40;
pub const AKEYCODE_M: i32 = 41;
pub const AKEYCODE_N: i32 = 42;
pub const AKEYCODE_O: i32 = 43;
pub const AKEYCODE_P: i32 = 44;
pub const AKEYCODE_Q: i32 = 45;
pub const AKEYCODE_R: i32 = 46;
pub const AKEYCODE_S: i32 = 47;
pub const AKEYCODE_T: i32 = 48;
pub const AKEYCODE_U: i32 = 49;
pub const AKEYCODE_V: i32 = 50;
pub const AKEYCODE_W: i32 = 51;
pub const AKEYCODE_X: i32 = 52;
pub const AKEYCODE_Y: i32 = 53;
pub const AKEYCODE_Z: i32 = 54;
pub const AKEYCODE_COMMA: i32 = 55;
pub const AKEYCODE_PERIOD: i32 = 56;
pub const AKEYCODE_ALT_LEFT: i32 = 57;
pub const AKEYCODE_ALT_RIGHT: i32 = 58;
pub const AKEYCODE_SHIFT_LEFT: i32 = 59;
pub const AKEYCODE_SHIFT_RIGHT: i32 = 60;
pub const AKEYCODE_TAB: i32 = 61;
pub const AKEYCODE_SPACE: i32 = 62;
pub const AKEYCODE_ENTER: i32 = 66;
pub const AKEYCODE_DEL: i32 = 67; // Backspace
pub const AKEYCODE_GRAVE: i32 = 68;
pub const AKEYCODE_MINUS: i32 = 69;
pub const AKEYCODE_EQUALS: i32 = 70;
pub const AKEYCODE_LEFT_BRACKET: i32 = 71;
pub const AKEYCODE_RIGHT_BRACKET: i32 = 72;
pub const AKEYCODE_BACKSLASH: i32 = 73;
pub const AKEYCODE_SEMICOLON: i32 = 74;
pub const AKEYCODE_APOSTROPHE: i32 = 75;
pub const AKEYCODE_SLASH: i32 = 76;
pub const AKEYCODE_AT: i32 = 77;
pub const AKEYCODE_ESCAPE: i32 = 111;
pub const AKEYCODE_FORWARD_DEL: i32 = 112; // Delete
pub const AKEYCODE_CTRL_LEFT: i32 = 113;
pub const AKEYCODE_CTRL_RIGHT: i32 = 114;
pub const AKEYCODE_CAPS_LOCK: i32 = 115;
pub const AKEYCODE_SCROLL_LOCK: i32 = 116;
pub const AKEYCODE_META_LEFT: i32 = 117;
pub const AKEYCODE_META_RIGHT: i32 = 118;
pub const AKEYCODE_FUNCTION: i32 = 119;
pub const AKEYCODE_SYSRQ: i32 = 120;
pub const AKEYCODE_BREAK: i32 = 121;
pub const AKEYCODE_MOVE_HOME: i32 = 122;
pub const AKEYCODE_MOVE_END: i32 = 123;
pub const AKEYCODE_INSERT: i32 = 124;
pub const AKEYCODE_PAGE_UP: i32 = 92;
pub const AKEYCODE_PAGE_DOWN: i32 = 93;
pub const AKEYCODE_F1: i32 = 131;
pub const AKEYCODE_F2: i32 = 132;
pub const AKEYCODE_F3: i32 = 133;
pub const AKEYCODE_F4: i32 = 134;
pub const AKEYCODE_F5: i32 = 135;
pub const AKEYCODE_F6: i32 = 136;
pub const AKEYCODE_F7: i32 = 137;
pub const AKEYCODE_F8: i32 = 138;
pub const AKEYCODE_F9: i32 = 139;
pub const AKEYCODE_F10: i32 = 140;
pub const AKEYCODE_F11: i32 = 141;
pub const AKEYCODE_F12: i32 = 142;
pub const AKEYCODE_NUM_LOCK: i32 = 143;
pub const AKEYCODE_NUMPAD_0: i32 = 144;
pub const AKEYCODE_NUMPAD_1: i32 = 145;
pub const AKEYCODE_NUMPAD_2: i32 = 146;
pub const AKEYCODE_NUMPAD_3: i32 = 147;
pub const AKEYCODE_NUMPAD_4: i32 = 148;
pub const AKEYCODE_NUMPAD_5: i32 = 149;
pub const AKEYCODE_NUMPAD_6: i32 = 150;
pub const AKEYCODE_NUMPAD_7: i32 = 151;
pub const AKEYCODE_NUMPAD_8: i32 = 152;
pub const AKEYCODE_NUMPAD_9: i32 = 153;
pub const AKEYCODE_NUMPAD_DIVIDE: i32 = 154;
pub const AKEYCODE_NUMPAD_MULTIPLY: i32 = 155;
pub const AKEYCODE_NUMPAD_SUBTRACT: i32 = 156;
pub const AKEYCODE_NUMPAD_ADD: i32 = 157;
pub const AKEYCODE_NUMPAD_DOT: i32 = 158;
pub const AKEYCODE_NUMPAD_ENTER: i32 = 160;
pub const AKEYCODE_VOLUME_UP: i32 = 24;
pub const AKEYCODE_VOLUME_DOWN: i32 = 25;
pub const AKEYCODE_VOLUME_MUTE: i32 = 164;
pub const AKEYCODE_MENU: i32 = 82;

// ── Android key actions (used by the backend when reading AKeyEvent) ──────────

/// `AKEY_EVENT_ACTION_DOWN`
pub const AKEY_EVENT_ACTION_DOWN: i32 = 0;
/// `AKEY_EVENT_ACTION_UP`
pub const AKEY_EVENT_ACTION_UP: i32 = 1;
/// `AKEY_EVENT_ACTION_MULTIPLE` — used for character repeat.
pub const AKEY_EVENT_ACTION_MULTIPLE: i32 = 2;

// ── Conversion helpers ───────────────────────────────────────────────────────

/// Convert an Android `meta_state` bitmask to Floom [`Modifiers`].
///
/// `AMETA_FUNCTION_ON` and `AMETA_CAPS_LOCK_ON` have no Floom equivalent and
/// are intentionally dropped.
pub fn android_meta_to_modifiers(meta_state: i32) -> Modifiers {
    let mut m = Modifiers::empty();
    if meta_state & AMETA_CTRL_ON != 0 {
        m = m.union(Modifiers::CONTROL);
    }
    if meta_state & AMETA_ALT_ON != 0 {
        m = m.union(Modifiers::ALT);
    }
    if meta_state & AMETA_SHIFT_ON != 0 {
        m = m.union(Modifiers::SHIFT);
    }
    if meta_state & AMETA_META_ON != 0 {
        m = m.union(Modifiers::META);
    }
    m
}

/// Check whether Caps Lock is active from a raw meta-state bitmask.
///
/// Floom's [`Modifiers`] cannot represent this; the backend may consult it
/// separately if it needs the state.
pub fn android_meta_caps_lock(meta_state: i32) -> bool {
    meta_state & AMETA_CAPS_LOCK_ON != 0
}

/// Convert an Android key code to a Floom [`Key`].
///
/// Returns `None` for key codes that should be ignored (pure modifier keys,
/// volume buttons, function keys `F1`–`F12` — Floom has no `NamedKey` for
/// them yet — and unknown codes).
pub fn android_keycode_to_key(key_code: i32) -> Option<Key> {
    let key = match key_code {
        // Letters (normalized to lower case; case lives in Modifiers::SHIFT).
        AKEYCODE_A => Key::Char('a'),
        AKEYCODE_B => Key::Char('b'),
        AKEYCODE_C => Key::Char('c'),
        AKEYCODE_D => Key::Char('d'),
        AKEYCODE_E => Key::Char('e'),
        AKEYCODE_F => Key::Char('f'),
        AKEYCODE_G => Key::Char('g'),
        AKEYCODE_H => Key::Char('h'),
        AKEYCODE_I => Key::Char('i'),
        AKEYCODE_J => Key::Char('j'),
        AKEYCODE_K => Key::Char('k'),
        AKEYCODE_L => Key::Char('l'),
        AKEYCODE_M => Key::Char('m'),
        AKEYCODE_N => Key::Char('n'),
        AKEYCODE_O => Key::Char('o'),
        AKEYCODE_P => Key::Char('p'),
        AKEYCODE_Q => Key::Char('q'),
        AKEYCODE_R => Key::Char('r'),
        AKEYCODE_S => Key::Char('s'),
        AKEYCODE_T => Key::Char('t'),
        AKEYCODE_U => Key::Char('u'),
        AKEYCODE_V => Key::Char('v'),
        AKEYCODE_W => Key::Char('w'),
        AKEYCODE_X => Key::Char('x'),
        AKEYCODE_Y => Key::Char('y'),
        AKEYCODE_Z => Key::Char('z'),

        // Number row
        AKEYCODE_0 => Key::Char('0'),
        AKEYCODE_1 => Key::Char('1'),
        AKEYCODE_2 => Key::Char('2'),
        AKEYCODE_3 => Key::Char('3'),
        AKEYCODE_4 => Key::Char('4'),
        AKEYCODE_5 => Key::Char('5'),
        AKEYCODE_6 => Key::Char('6'),
        AKEYCODE_7 => Key::Char('7'),
        AKEYCODE_8 => Key::Char('8'),
        AKEYCODE_9 => Key::Char('9'),

        // Punctuation / symbols
        AKEYCODE_COMMA => Key::Char(','),
        AKEYCODE_PERIOD => Key::Char('.'),
        AKEYCODE_SPACE => Key::Char(' '),
        AKEYCODE_GRAVE => Key::Char('`'),
        AKEYCODE_MINUS => Key::Char('-'),
        AKEYCODE_EQUALS => Key::Char('='),
        AKEYCODE_LEFT_BRACKET => Key::Char('['),
        AKEYCODE_RIGHT_BRACKET => Key::Char(']'),
        AKEYCODE_BACKSLASH => Key::Char('\\'),
        AKEYCODE_SEMICOLON => Key::Char(';'),
        AKEYCODE_APOSTROPHE => Key::Char('\''),
        AKEYCODE_SLASH => Key::Char('/'),
        AKEYCODE_AT => Key::Char('@'),
        AKEYCODE_STAR => Key::Char('*'),
        AKEYCODE_POUND => Key::Char('#'),

        // Navigation
        AKEYCODE_ENTER => Key::Named(NamedKey::Enter),
        AKEYCODE_ESCAPE => Key::Named(NamedKey::Escape),
        AKEYCODE_DEL => Key::Named(NamedKey::Backspace),
        AKEYCODE_FORWARD_DEL => Key::Named(NamedKey::Delete),
        AKEYCODE_TAB => Key::Named(NamedKey::Tab),
        AKEYCODE_DPAD_UP => Key::Named(NamedKey::ArrowUp),
        AKEYCODE_DPAD_DOWN => Key::Named(NamedKey::ArrowDown),
        AKEYCODE_DPAD_LEFT => Key::Named(NamedKey::ArrowLeft),
        AKEYCODE_DPAD_RIGHT => Key::Named(NamedKey::ArrowRight),
        AKEYCODE_DPAD_CENTER => Key::Named(NamedKey::Enter),
        AKEYCODE_MOVE_HOME => Key::Named(NamedKey::Home),
        AKEYCODE_MOVE_END => Key::Named(NamedKey::End),
        // No Floom `Insert` named key yet → ignored.
        AKEYCODE_PAGE_UP => Key::Named(NamedKey::PageUp),
        AKEYCODE_PAGE_DOWN => Key::Named(NamedKey::PageDown),

        // Numpad (digits reuse the same chars; operators become their symbol)
        AKEYCODE_NUMPAD_0 => Key::Char('0'),
        AKEYCODE_NUMPAD_1 => Key::Char('1'),
        AKEYCODE_NUMPAD_2 => Key::Char('2'),
        AKEYCODE_NUMPAD_3 => Key::Char('3'),
        AKEYCODE_NUMPAD_4 => Key::Char('4'),
        AKEYCODE_NUMPAD_5 => Key::Char('5'),
        AKEYCODE_NUMPAD_6 => Key::Char('6'),
        AKEYCODE_NUMPAD_7 => Key::Char('7'),
        AKEYCODE_NUMPAD_8 => Key::Char('8'),
        AKEYCODE_NUMPAD_9 => Key::Char('9'),
        AKEYCODE_NUMPAD_DIVIDE => Key::Char('/'),
        AKEYCODE_NUMPAD_MULTIPLY => Key::Char('*'),
        AKEYCODE_NUMPAD_SUBTRACT => Key::Char('-'),
        AKEYCODE_NUMPAD_ADD => Key::Char('+'),
        AKEYCODE_NUMPAD_DOT => Key::Char('.'),
        AKEYCODE_NUMPAD_ENTER => Key::Named(NamedKey::Enter),

        // Menu / Back. No Floom `Menu` named key yet → ignored.
        AKEYCODE_MENU => return None,
        AKEYCODE_BACK => Key::Named(NamedKey::Escape), // Android back → escape

        // Function keys F1–F12: Floom has no NamedKey for them yet.
        AKEYCODE_F1..=AKEYCODE_F12 => return None,

        // Pure modifier keys, volume, system keys, and anything unknown → ignore.
        _ if is_ignored_modifier(key_code) => return None,
        _ => return None,
    };

    Some(key)
}

/// Whether the code is a pure modifier key (no character / no navigation).
fn is_ignored_modifier(key_code: i32) -> bool {
    matches!(
        key_code,
        AKEYCODE_SHIFT_LEFT
            | AKEYCODE_SHIFT_RIGHT
            | AKEYCODE_CTRL_LEFT
            | AKEYCODE_CTRL_RIGHT
            | AKEYCODE_ALT_LEFT
            | AKEYCODE_ALT_RIGHT
            | AKEYCODE_META_LEFT
            | AKEYCODE_META_RIGHT
            | AKEYCODE_FUNCTION
            | AKEYCODE_CAPS_LOCK
            | AKEYCODE_NUM_LOCK
            | AKEYCODE_SCROLL_LOCK
            | AKEYCODE_VOLUME_UP
            | AKEYCODE_VOLUME_DOWN
            | AKEYCODE_VOLUME_MUTE
            | AKEYCODE_HOME
            | AKEYCODE_SOFT_LEFT
            | AKEYCODE_SOFT_RIGHT
            | AKEYCODE_SYSRQ
            | AKEYCODE_BREAK
    )
}

/// Build a Floom [`KeyEvent`] (phase [`KeyPhase::Down`]) from an Android key
/// event. Returns `None` if the key code should be ignored.
///
/// `unicode_char` — the Unicode character produced by the key event (from
/// `AKeyEvent_getUnicodeChar()`). This is currently unused by the table because
/// Floom carries case in [`Modifiers::SHIFT`]; the backend may pass `0`.
pub fn android_key_to_key_event(
    key_code: i32,
    meta_state: i32,
    _unicode_char: u32,
) -> Option<KeyEvent> {
    let key = android_keycode_to_key(key_code)?;
    Some(KeyEvent {
        phase: KeyPhase::Down,
        key,
        modifiers: android_meta_to_modifiers(meta_state),
    })
}

/// Create a [`KeyEvent`] for a software-keyboard text insertion (a single
/// character delivered as a string rather than a key event).
///
/// Letters are normalized to lower case with the case moved into
/// [`Modifiers::SHIFT`], matching Floom's key convention.
pub fn character_to_key_event(c: char) -> KeyEvent {
    let mut modifiers = Modifiers::empty();
    let key = if c.is_ascii_alphabetic() {
        if c.is_ascii_uppercase() {
            modifiers = modifiers.union(Modifiers::SHIFT);
        }
        Key::Char(c.to_ascii_lowercase())
    } else {
        Key::Char(c)
    };
    KeyEvent {
        phase: KeyPhase::Down,
        key,
        modifiers,
    }
}

/// Create a backspace [`KeyEvent`] for a software-keyboard deletion.
pub fn backspace_key_event() -> KeyEvent {
    KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Backspace),
        modifiers: Modifiers::empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_to_modifiers_none() {
        let m = android_meta_to_modifiers(AMETA_NONE);
        assert!(m.is_empty());
    }

    #[test]
    fn meta_to_modifiers_ctrl_shift() {
        let m = android_meta_to_modifiers(AMETA_CTRL_ON | AMETA_SHIFT_ON);
        assert!(m.contains(Modifiers::CONTROL));
        assert!(m.contains(Modifiers::SHIFT));
        assert!(!m.contains(Modifiers::ALT));
        assert!(!m.contains(Modifiers::META));
    }

    #[test]
    fn meta_to_modifiers_all_except_function() {
        let m = android_meta_to_modifiers(
            AMETA_CTRL_ON | AMETA_ALT_ON | AMETA_SHIFT_ON | AMETA_META_ON | AMETA_FUNCTION_ON,
        );
        assert!(m.contains(Modifiers::CONTROL));
        assert!(m.contains(Modifiers::ALT));
        assert!(m.contains(Modifiers::SHIFT));
        assert!(m.contains(Modifiers::META));
        // FUNCTION has no Floom equivalent and must not leak into the bits.
        assert_eq!(m.bits(), (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3));
    }

    #[test]
    fn caps_lock_detection() {
        assert!(!android_meta_caps_lock(AMETA_NONE));
        assert!(android_meta_caps_lock(AMETA_CAPS_LOCK_ON));
        assert!(android_meta_caps_lock(AMETA_CAPS_LOCK_ON | AMETA_SHIFT_ON));
    }

    #[test]
    fn letter_keycodes() {
        assert_eq!(android_keycode_to_key(AKEYCODE_A), Some(Key::Char('a')));
        assert_eq!(android_keycode_to_key(AKEYCODE_Z), Some(Key::Char('z')));
    }

    #[test]
    fn number_keycodes() {
        assert_eq!(android_keycode_to_key(AKEYCODE_0), Some(Key::Char('0')));
        assert_eq!(android_keycode_to_key(AKEYCODE_9), Some(Key::Char('9')));
    }

    #[test]
    fn special_keycodes() {
        assert_eq!(
            android_keycode_to_key(AKEYCODE_ENTER),
            Some(Key::Named(NamedKey::Enter))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_DEL),
            Some(Key::Named(NamedKey::Backspace))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_ESCAPE),
            Some(Key::Named(NamedKey::Escape))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_TAB),
            Some(Key::Named(NamedKey::Tab))
        );
        assert_eq!(android_keycode_to_key(AKEYCODE_SPACE), Some(Key::Char(' ')));
    }

    #[test]
    fn arrow_keycodes() {
        assert_eq!(
            android_keycode_to_key(AKEYCODE_DPAD_UP),
            Some(Key::Named(NamedKey::ArrowUp))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_DPAD_DOWN),
            Some(Key::Named(NamedKey::ArrowDown))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_DPAD_LEFT),
            Some(Key::Named(NamedKey::ArrowLeft))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_DPAD_RIGHT),
            Some(Key::Named(NamedKey::ArrowRight))
        );
    }

    #[test]
    fn function_keys_unmapped() {
        // Floom's NamedKey lacks F1–F12; they must map to None.
        assert_eq!(android_keycode_to_key(AKEYCODE_F1), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_F12), None);
    }

    #[test]
    fn modifier_keys_are_ignored() {
        assert_eq!(android_keycode_to_key(AKEYCODE_SHIFT_LEFT), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_CTRL_LEFT), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_ALT_LEFT), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_META_LEFT), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_CAPS_LOCK), None);
    }

    #[test]
    fn volume_keys_are_ignored() {
        assert_eq!(android_keycode_to_key(AKEYCODE_VOLUME_UP), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_VOLUME_DOWN), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_VOLUME_MUTE), None);
    }

    #[test]
    fn unknown_keycode_is_none() {
        assert_eq!(android_keycode_to_key(99999), None);
        assert_eq!(android_keycode_to_key(AKEYCODE_UNKNOWN), None);
    }

    #[test]
    fn back_button_maps_to_escape() {
        assert_eq!(
            android_keycode_to_key(AKEYCODE_BACK),
            Some(Key::Named(NamedKey::Escape))
        );
    }

    #[test]
    fn key_event_from_key_code() {
        let ks = android_key_to_key_event(AKEYCODE_A, AMETA_NONE, 0).unwrap();
        assert_eq!(ks.key, Key::Char('a'));
        assert!(ks.modifiers.is_empty());
        assert_eq!(ks.phase, KeyPhase::Down);
    }

    #[test]
    fn key_event_with_shift() {
        let ks = android_key_to_key_event(AKEYCODE_A, AMETA_SHIFT_ON, 0).unwrap();
        assert_eq!(ks.key, Key::Char('a')); // lower-case, case in SHIFT
        assert!(ks.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn key_event_ctrl_c() {
        let ks = android_key_to_key_event(AKEYCODE_C, AMETA_CTRL_ON, 0).unwrap();
        assert_eq!(ks.key, Key::Char('c'));
        assert!(ks.modifiers.contains(Modifiers::CONTROL));
    }

    #[test]
    fn key_event_enter() {
        let ks = android_key_to_key_event(AKEYCODE_ENTER, AMETA_NONE, 0).unwrap();
        assert_eq!(ks.key, Key::Named(NamedKey::Enter));
    }

    #[test]
    fn key_event_modifier_only_returns_none() {
        assert!(android_key_to_key_event(AKEYCODE_SHIFT_LEFT, AMETA_SHIFT_ON, 0).is_none());
        assert!(android_key_to_key_event(AKEYCODE_CTRL_LEFT, AMETA_CTRL_ON, 0).is_none());
    }

    #[test]
    fn character_to_key_event_basic() {
        let ks = character_to_key_event('h');
        assert_eq!(ks.key, Key::Char('h'));
        assert!(ks.modifiers.is_empty());
    }

    #[test]
    fn character_to_key_event_uppercase_carries_shift() {
        let ks = character_to_key_event('H');
        assert_eq!(ks.key, Key::Char('h'));
        assert!(ks.modifiers.contains(Modifiers::SHIFT));
    }

    #[test]
    fn backspace_key_event_is_named() {
        let ks = backspace_key_event();
        assert_eq!(ks.key, Key::Named(NamedKey::Backspace));
    }

    #[test]
    fn numpad_keycodes() {
        assert_eq!(
            android_keycode_to_key(AKEYCODE_NUMPAD_0),
            Some(Key::Char('0'))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_NUMPAD_ENTER),
            Some(Key::Named(NamedKey::Enter))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_NUMPAD_ADD),
            Some(Key::Char('+'))
        );
    }

    #[test]
    fn punctuation_keycodes() {
        assert_eq!(android_keycode_to_key(AKEYCODE_COMMA), Some(Key::Char(',')));
        assert_eq!(
            android_keycode_to_key(AKEYCODE_SEMICOLON),
            Some(Key::Char(';'))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_APOSTROPHE),
            Some(Key::Char('\''))
        );
        assert_eq!(
            android_keycode_to_key(AKEYCODE_BACKSLASH),
            Some(Key::Char('\\'))
        );
    }
}
