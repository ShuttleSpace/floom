//! Canonical key identities shared by platform backends and the runtime.
//!
//! A backend sees OS-specific key enumerations (`winit::keyboard::KeyCode`,
//! Android keycodes, DOM `code` strings, …). It must not leak those into the
//! ABI, and it must not invent its own numbering either — otherwise the layer
//! that turns a [`KeyCode`] into a text/command [`Key`] would have to know
//! which backend produced it.
//!
//! So `KeyCode` is a small canonical space defined here, in the OS-neutral
//! contract crate, together with the mapping to [`floem_types::Key`]. Backends
//! do one `match` into this space; the app layer does one `to_key()`.
//!
//! The numbering is deliberately readable rather than arbitrary:
//!
//! | range          | meaning                                        |
//! |----------------|------------------------------------------------|
//! | `0x00`         | [`KeyCode::NONE`] — unknown / unmapped          |
//! | `0x01..=0x1F`  | named keys (arrows, editing, modifiers, …)      |
//! | `0x20..=0x7E`  | printable ASCII — space, punctuation, digits, letters |
//! | `0xE000..`     | function keys `F1`..=`F24`                      |
//!
//! Using ASCII for the printable range means `KeyCode::from_char('a')` and a
//! letter pressed with Shift produce the same code; case is carried by
//! [`Modifiers`](floem_types::Modifiers), exactly as [`Key::Char`] expects.

use floem_types::{Key, NamedKey};

/// Canonical key identifier (see the module docs for the numbering scheme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyCode(pub u32);

impl KeyCode {
    /// Unknown or unmapped key.
    pub const NONE: Self = Self(0x00);

    // --- named keys ---------------------------------------------------------
    pub const ENTER: Self = Self(0x01);
    pub const ESCAPE: Self = Self(0x02);
    pub const TAB: Self = Self(0x03);
    pub const BACKSPACE: Self = Self(0x04);
    pub const DELETE: Self = Self(0x05);
    pub const ARROW_UP: Self = Self(0x06);
    pub const ARROW_DOWN: Self = Self(0x07);
    pub const ARROW_LEFT: Self = Self(0x08);
    pub const ARROW_RIGHT: Self = Self(0x09);
    pub const HOME: Self = Self(0x0A);
    pub const END: Self = Self(0x0B);
    pub const PAGE_UP: Self = Self(0x0C);
    pub const PAGE_DOWN: Self = Self(0x0D);
    pub const SHIFT: Self = Self(0x0E);
    pub const CONTROL: Self = Self(0x0F);
    pub const ALT: Self = Self(0x10);
    pub const META: Self = Self(0x11);
    pub const CAPS_LOCK: Self = Self(0x12);
    pub const INSERT: Self = Self(0x13);
    pub const CONTEXT_MENU: Self = Self(0x14);
    pub const PRINT_SCREEN: Self = Self(0x15);
    pub const SCROLL_LOCK: Self = Self(0x16);
    pub const PAUSE: Self = Self(0x17);
    pub const NUM_LOCK: Self = Self(0x18);
    pub const SPACE: Self = Self(0x20);

    /// First function key (`F1`). `F(n)` is `F1 + (n - 1)`.
    pub const F1: Self = Self(0xE000);
    /// Last function key we model.
    pub const F24: Self = Self(0xE017);

    /// `F1`..=`F24`; `n` outside `1..=24` yields [`KeyCode::NONE`].
    pub const fn function(n: u8) -> Self {
        if n >= 1 && n <= 24 {
            Self(0xE000 + (n as u32 - 1))
        } else {
            Self::NONE
        }
    }

    /// Printable ASCII (`0x20..=0x7E`) as its ASCII value; anything else is
    /// [`KeyCode::NONE`]. Letters are upper-cased so [`from_char`](Self::from_char)
    /// is case-insensitive and agrees with [`letter`](Self::letter); case is
    /// carried by the modifier set, not the code.
    pub const fn from_char(c: char) -> Self {
        let c = c as u32;
        if c >= 0x61 && c <= 0x7A {
            // a-z → A-Z
            Self(c - 0x20)
        } else if c >= 0x20 && c <= 0x7E {
            Self(c)
        } else {
            Self::NONE
        }
    }

    /// Letter constant by position: `letter(0)` is `A`, `letter(25)` is `Z`.
    /// Matches [`from_char`](Self::from_char) with an uppercase letter.
    pub const fn letter(index: u8) -> Self {
        if index < 26 {
            Self(0x41 + index as u32)
        } else {
            Self::NONE
        }
    }

    /// Digit constant: `digit(0)` is `0`, `digit(9)` is `9`.
    pub const fn digit(value: u8) -> Self {
        if value < 10 {
            Self(0x30 + value as u32)
        } else {
            Self::NONE
        }
    }

    /// Index of a function key, if this is one.
    pub const fn function_index(self) -> Option<u8> {
        if self.0 >= Self::F1.0 && self.0 <= Self::F24.0 {
            Some((self.0 - Self::F1.0) as u8 + 1)
        } else {
            None
        }
    }

    /// Translate into a text/command key the widget layer understands.
    ///
    /// Letters are normalised to lower case (case lives in the modifier set),
    /// matching [`Key::Char`]'s documented invariant. Function keys, lock keys
    /// and other keys with no `Key` representation return `None` — they still
    /// have a stable `KeyCode`, so nothing is lost at the platform boundary;
    /// [`NamedKey`] simply does not model them yet.
    pub fn to_key(self) -> Option<Key> {
        let named = match self {
            Self::ENTER => Some(NamedKey::Enter),
            Self::ESCAPE => Some(NamedKey::Escape),
            Self::TAB => Some(NamedKey::Tab),
            Self::BACKSPACE => Some(NamedKey::Backspace),
            Self::DELETE => Some(NamedKey::Delete),
            Self::ARROW_UP => Some(NamedKey::ArrowUp),
            Self::ARROW_DOWN => Some(NamedKey::ArrowDown),
            Self::ARROW_LEFT => Some(NamedKey::ArrowLeft),
            Self::ARROW_RIGHT => Some(NamedKey::ArrowRight),
            Self::HOME => Some(NamedKey::Home),
            Self::END => Some(NamedKey::End),
            Self::PAGE_UP => Some(NamedKey::PageUp),
            Self::PAGE_DOWN => Some(NamedKey::PageDown),
            Self::SHIFT => Some(NamedKey::Shift),
            Self::CONTROL => Some(NamedKey::Control),
            Self::ALT => Some(NamedKey::Alt),
            Self::META => Some(NamedKey::Meta),
            _ => None,
        };
        if let Some(named) = named {
            return Some(Key::Named(named));
        }

        match self.0 {
            0x20 => Some(Key::Char(' ')),
            0x21..=0x7E => {
                let c = char::from_u32(self.0)?;
                Some(Key::Char(c.to_ascii_lowercase()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys_round_trip() {
        for (code, named) in [
            (KeyCode::ENTER, NamedKey::Enter),
            (KeyCode::ESCAPE, NamedKey::Escape),
            (KeyCode::TAB, NamedKey::Tab),
            (KeyCode::BACKSPACE, NamedKey::Backspace),
            (KeyCode::DELETE, NamedKey::Delete),
            (KeyCode::ARROW_UP, NamedKey::ArrowUp),
            (KeyCode::ARROW_DOWN, NamedKey::ArrowDown),
            (KeyCode::ARROW_LEFT, NamedKey::ArrowLeft),
            (KeyCode::ARROW_RIGHT, NamedKey::ArrowRight),
            (KeyCode::HOME, NamedKey::Home),
            (KeyCode::END, NamedKey::End),
            (KeyCode::PAGE_UP, NamedKey::PageUp),
            (KeyCode::PAGE_DOWN, NamedKey::PageDown),
            (KeyCode::SHIFT, NamedKey::Shift),
            (KeyCode::CONTROL, NamedKey::Control),
            (KeyCode::ALT, NamedKey::Alt),
            (KeyCode::META, NamedKey::Meta),
        ] {
            assert_eq!(code.to_key(), Some(Key::Named(named)), "{code:?}");
        }
    }

    #[test]
    fn letters_normalise_case() {
        assert_eq!(KeyCode::letter(0), KeyCode::from_char('A'));
        assert_eq!(KeyCode::letter(0), KeyCode::from_char('a'));
        assert_eq!(KeyCode::letter(25), KeyCode::from_char('Z'));
        assert_eq!(KeyCode::letter(0).to_key(), Some(Key::Char('a')));
        assert_eq!(KeyCode::letter(25).to_key(), Some(Key::Char('z')));
    }

    #[test]
    fn digits_punctuation_and_space() {
        assert_eq!(KeyCode::digit(0), KeyCode::from_char('0'));
        assert_eq!(KeyCode::digit(9).to_key(), Some(Key::Char('9')));
        assert_eq!(KeyCode::SPACE.to_key(), Some(Key::Char(' ')));
        assert_eq!(KeyCode::from_char('/').to_key(), Some(Key::Char('/')));
    }

    #[test]
    fn digits_are_not_letters() {
        assert_ne!(KeyCode::digit(0), KeyCode::letter(0));
        assert_eq!(KeyCode::digit(0).0, 0x30);
        assert_eq!(KeyCode::letter(0).0, 0x41);
    }

    #[test]
    fn function_keys_are_stable_but_not_text() {
        assert_eq!(KeyCode::F1.function_index(), Some(1));
        assert_eq!(KeyCode::F24.function_index(), Some(24));
        assert_eq!(KeyCode::function(5).function_index(), Some(5));
        assert_eq!(KeyCode::function(0), KeyCode::NONE);
        assert_eq!(KeyCode::function(25), KeyCode::NONE);
        // Modelled at the platform boundary, but `NamedKey` has no variant.
        assert_eq!(KeyCode::F1.to_key(), None);
    }

    #[test]
    fn out_of_range_inputs_are_none() {
        assert_eq!(KeyCode::letter(26), KeyCode::NONE);
        assert_eq!(KeyCode::digit(10), KeyCode::NONE);
        assert_eq!(KeyCode::from_char('中'), KeyCode::NONE);
        assert_eq!(KeyCode::NONE.to_key(), None);
        assert_eq!(KeyCode::CAPS_LOCK.to_key(), None);
    }
}
