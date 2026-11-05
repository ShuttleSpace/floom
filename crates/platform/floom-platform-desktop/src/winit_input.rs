// SPDX-License-Identifier: MIT
//! Pure winit → Floom ABI input translations.
//!
//! Everything here is a total function over winit's value types, so it is unit
//! testable without creating a window or an event loop — which matters, because
//! that is the part of a backend most likely to be subtly wrong and least
//! likely to be exercised in CI.
//!
//! Key identity uses the **logical** key (what the active layout produces), not
//! the physical scancode: [`KeyCode`] is defined as a layout-resolved key so
//! that `to_key()` yields the character the user actually typed. The physical
//! code is used only as a fallback, for keys the layout does not name.

use floom_platform::{KeyCode, Modifiers, PointerButton};
use winit::event::{ButtonSource, ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode as WinitKeyCode;
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey, PhysicalKey};

/// Pixels per "line" of a legacy line-based wheel delta.
///
/// Trackpads report pixel deltas directly; a classic wheel with notches reports
/// `LineDelta`, which carries no unit. 20 px per notch is the conventional
/// conversion used by the browsers and matches what a scroll view expects.
const PIXELS_PER_SCROLL_LINE: f32 = 20.0;

/// Modifier chord as recorded on the most recent event.
pub fn map_modifiers(state: ModifiersState) -> Modifiers {
    let mut out = Modifiers::empty();
    if state.shift_key() {
        out = out.union(Modifiers::SHIFT);
    }
    if state.control_key() {
        out = out.union(Modifiers::CONTROL);
    }
    if state.alt_key() {
        out = out.union(Modifiers::ALT);
    }
    if state.meta_key() {
        out = out.union(Modifiers::META);
    }
    out
}

/// Pointer button, including the back/forward pair.
pub fn map_mouse_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Auxiliary,
        MouseButton::Back => PointerButton::Other(3),
        MouseButton::Forward => PointerButton::Other(4),
        MouseButton::Other(code) => PointerButton::Other(code.min(u8::MAX as u16) as u8),
    }
}

/// What produced a pointer *button* event.
///
/// winit distinguishes a mouse button from a finger contact and a tablet stylus
/// barrel button. The ABI only models the button itself, so a touch contact is
/// reported as a primary button press — which is precisely how a touch becomes
/// a click in a desktop UI.
pub fn map_button_source(source: ButtonSource) -> PointerButton {
    match source {
        ButtonSource::Mouse(button) => map_mouse_button(button),
        ButtonSource::Touch { .. } => PointerButton::Primary,
        ButtonSource::TabletTool { .. } => PointerButton::Primary,
        // A backend-reported button id we have no name for: keep it
        // distinguishable rather than pretending it means something.
        ButtonSource::Unknown(code) => PointerButton::Other(code.min(u8::MAX as u16) as u8),
    }
}

pub fn is_pressed(state: ElementState) -> bool {
    matches!(state, ElementState::Pressed)
}

/// Logical key → canonical code. Prefers the layout-resolved key so text input
/// follows the keyboard layout, and falls back to the physical code for keys
/// the layout leaves unnamed.
pub fn map_key(logical: &WinitKey, physical: PhysicalKey) -> KeyCode {
    match logical {
        WinitKey::Named(named) => {
            let code = map_named(*named);
            if code != KeyCode::NONE {
                code
            } else {
                map_physical(physical)
            }
        }
        WinitKey::Character(text) => {
            // A layout may produce a multi-character string (dead-key
            // composition); the first scalar is the key identity, the rest
            // arrives through the IME/text path.
            let code = text
                .chars()
                .next()
                .map(KeyCode::from_char)
                .unwrap_or(KeyCode::NONE);
            if code != KeyCode::NONE {
                code
            } else {
                map_physical(physical)
            }
        }
        // `Dead` and `Unidentified` carry no usable character.
        _ => map_physical(physical),
    }
}

/// Named key → canonical code (`KeyCode::NONE` when we do not model it).
pub fn map_named(named: NamedKey) -> KeyCode {
    match named {
        NamedKey::Enter => KeyCode::ENTER,
        NamedKey::Escape => KeyCode::ESCAPE,
        NamedKey::Tab => KeyCode::TAB,
        NamedKey::Backspace => KeyCode::BACKSPACE,
        NamedKey::Delete => KeyCode::DELETE,
        NamedKey::ArrowUp => KeyCode::ARROW_UP,
        NamedKey::ArrowDown => KeyCode::ARROW_DOWN,
        NamedKey::ArrowLeft => KeyCode::ARROW_LEFT,
        NamedKey::ArrowRight => KeyCode::ARROW_RIGHT,
        NamedKey::Home => KeyCode::HOME,
        NamedKey::End => KeyCode::END,
        NamedKey::PageUp => KeyCode::PAGE_UP,
        NamedKey::PageDown => KeyCode::PAGE_DOWN,
        NamedKey::Shift => KeyCode::SHIFT,
        NamedKey::Control => KeyCode::CONTROL,
        NamedKey::Alt | NamedKey::AltGraph => KeyCode::ALT,
        NamedKey::Meta => KeyCode::META,
        NamedKey::CapsLock => KeyCode::CAPS_LOCK,
        NamedKey::Insert => KeyCode::INSERT,
        NamedKey::ContextMenu => KeyCode::CONTEXT_MENU,
        NamedKey::PrintScreen => KeyCode::PRINT_SCREEN,
        NamedKey::ScrollLock => KeyCode::SCROLL_LOCK,
        NamedKey::Pause => KeyCode::PAUSE,
        NamedKey::NumLock => KeyCode::NUM_LOCK,
        NamedKey::F1 => KeyCode::function(1),
        NamedKey::F2 => KeyCode::function(2),
        NamedKey::F3 => KeyCode::function(3),
        NamedKey::F4 => KeyCode::function(4),
        NamedKey::F5 => KeyCode::function(5),
        NamedKey::F6 => KeyCode::function(6),
        NamedKey::F7 => KeyCode::function(7),
        NamedKey::F8 => KeyCode::function(8),
        NamedKey::F9 => KeyCode::function(9),
        NamedKey::F10 => KeyCode::function(10),
        NamedKey::F11 => KeyCode::function(11),
        NamedKey::F12 => KeyCode::function(12),
        NamedKey::F13 => KeyCode::function(13),
        NamedKey::F14 => KeyCode::function(14),
        NamedKey::F15 => KeyCode::function(15),
        NamedKey::F16 => KeyCode::function(16),
        NamedKey::F17 => KeyCode::function(17),
        NamedKey::F18 => KeyCode::function(18),
        NamedKey::F19 => KeyCode::function(19),
        NamedKey::F20 => KeyCode::function(20),
        NamedKey::F21 => KeyCode::function(21),
        NamedKey::F22 => KeyCode::function(22),
        NamedKey::F23 => KeyCode::function(23),
        NamedKey::F24 => KeyCode::function(24),
        _ => KeyCode::NONE,
    }
}

/// Physical code → canonical code. Only needed for keys with no logical name,
/// so it is deliberately small: printable keys come from the layout.
pub fn map_physical(physical: PhysicalKey) -> KeyCode {
    let PhysicalKey::Code(code) = physical else {
        return KeyCode::NONE;
    };
    match code {
        WinitKeyCode::Space => KeyCode::SPACE,
        WinitKeyCode::Enter | WinitKeyCode::NumpadEnter => KeyCode::ENTER,
        WinitKeyCode::Escape => KeyCode::ESCAPE,
        WinitKeyCode::Tab => KeyCode::TAB,
        WinitKeyCode::Backspace => KeyCode::BACKSPACE,
        WinitKeyCode::Delete => KeyCode::DELETE,
        WinitKeyCode::ArrowUp => KeyCode::ARROW_UP,
        WinitKeyCode::ArrowDown => KeyCode::ARROW_DOWN,
        WinitKeyCode::ArrowLeft => KeyCode::ARROW_LEFT,
        WinitKeyCode::ArrowRight => KeyCode::ARROW_RIGHT,
        WinitKeyCode::Home => KeyCode::HOME,
        WinitKeyCode::End => KeyCode::END,
        WinitKeyCode::PageUp => KeyCode::PAGE_UP,
        WinitKeyCode::PageDown => KeyCode::PAGE_DOWN,
        WinitKeyCode::ShiftLeft | WinitKeyCode::ShiftRight => KeyCode::SHIFT,
        WinitKeyCode::ControlLeft | WinitKeyCode::ControlRight => KeyCode::CONTROL,
        WinitKeyCode::AltLeft | WinitKeyCode::AltRight => KeyCode::ALT,
        WinitKeyCode::MetaLeft | WinitKeyCode::MetaRight => KeyCode::META,
        WinitKeyCode::CapsLock => KeyCode::CAPS_LOCK,
        WinitKeyCode::Insert => KeyCode::INSERT,
        WinitKeyCode::ContextMenu => KeyCode::CONTEXT_MENU,
        WinitKeyCode::PrintScreen => KeyCode::PRINT_SCREEN,
        WinitKeyCode::ScrollLock => KeyCode::SCROLL_LOCK,
        WinitKeyCode::Pause => KeyCode::PAUSE,
        WinitKeyCode::NumLock => KeyCode::NUM_LOCK,
        // `KeyA`..`KeyZ` and `Digit0`..`Digit9` are stable fallbacks for a
        // layout that produced no character.
        WinitKeyCode::KeyA => KeyCode::letter(0),
        WinitKeyCode::KeyB => KeyCode::letter(1),
        WinitKeyCode::KeyC => KeyCode::letter(2),
        WinitKeyCode::KeyD => KeyCode::letter(3),
        WinitKeyCode::KeyE => KeyCode::letter(4),
        WinitKeyCode::KeyF => KeyCode::letter(5),
        WinitKeyCode::KeyG => KeyCode::letter(6),
        WinitKeyCode::KeyH => KeyCode::letter(7),
        WinitKeyCode::KeyI => KeyCode::letter(8),
        WinitKeyCode::KeyJ => KeyCode::letter(9),
        WinitKeyCode::KeyK => KeyCode::letter(10),
        WinitKeyCode::KeyL => KeyCode::letter(11),
        WinitKeyCode::KeyM => KeyCode::letter(12),
        WinitKeyCode::KeyN => KeyCode::letter(13),
        WinitKeyCode::KeyO => KeyCode::letter(14),
        WinitKeyCode::KeyP => KeyCode::letter(15),
        WinitKeyCode::KeyQ => KeyCode::letter(16),
        WinitKeyCode::KeyR => KeyCode::letter(17),
        WinitKeyCode::KeyS => KeyCode::letter(18),
        WinitKeyCode::KeyT => KeyCode::letter(19),
        WinitKeyCode::KeyU => KeyCode::letter(20),
        WinitKeyCode::KeyV => KeyCode::letter(21),
        WinitKeyCode::KeyW => KeyCode::letter(22),
        WinitKeyCode::KeyX => KeyCode::letter(23),
        WinitKeyCode::KeyY => KeyCode::letter(24),
        WinitKeyCode::KeyZ => KeyCode::letter(25),
        WinitKeyCode::Digit0 => KeyCode::digit(0),
        WinitKeyCode::Digit1 => KeyCode::digit(1),
        WinitKeyCode::Digit2 => KeyCode::digit(2),
        WinitKeyCode::Digit3 => KeyCode::digit(3),
        WinitKeyCode::Digit4 => KeyCode::digit(4),
        WinitKeyCode::Digit5 => KeyCode::digit(5),
        WinitKeyCode::Digit6 => KeyCode::digit(6),
        WinitKeyCode::Digit7 => KeyCode::digit(7),
        WinitKeyCode::Digit8 => KeyCode::digit(8),
        WinitKeyCode::Digit9 => KeyCode::digit(9),
        WinitKeyCode::F1 => KeyCode::function(1),
        WinitKeyCode::F2 => KeyCode::function(2),
        WinitKeyCode::F3 => KeyCode::function(3),
        WinitKeyCode::F4 => KeyCode::function(4),
        WinitKeyCode::F5 => KeyCode::function(5),
        WinitKeyCode::F6 => KeyCode::function(6),
        WinitKeyCode::F7 => KeyCode::function(7),
        WinitKeyCode::F8 => KeyCode::function(8),
        WinitKeyCode::F9 => KeyCode::function(9),
        WinitKeyCode::F10 => KeyCode::function(10),
        WinitKeyCode::F11 => KeyCode::function(11),
        WinitKeyCode::F12 => KeyCode::function(12),
        _ => KeyCode::NONE,
    }
}

/// Wheel / trackpad delta as `(dx, dy)` in logical pixels.
pub fn map_scroll_delta(delta: MouseScrollDelta, scale: f64) -> (f64, f64) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (
            (x * PIXELS_PER_SCROLL_LINE) as f64,
            (y * PIXELS_PER_SCROLL_LINE) as f64,
        ),
        // Pixel deltas are physical; normalise so a consumer sees logical px.
        MouseScrollDelta::PixelDelta(pos) => (pos.x / scale, pos.y / scale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::SmolStr;

    #[test]
    fn logical_character_wins_over_physical_position() {
        // AZERTY: the physical `KeyQ` position produces the character 'a'.
        let logical = WinitKey::Character(SmolStr::new("a"));
        assert_eq!(
            map_key(&logical, PhysicalKey::Code(WinitKeyCode::KeyQ)),
            KeyCode::letter(0)
        );
    }

    #[test]
    fn named_keys_map_directly() {
        let enter = WinitKey::Named(NamedKey::Enter);
        assert_eq!(
            map_key(&enter, PhysicalKey::Code(WinitKeyCode::Enter)),
            KeyCode::ENTER
        );
        let f5 = WinitKey::Named(NamedKey::F5);
        assert_eq!(
            map_key(&f5, PhysicalKey::Code(WinitKeyCode::F5)),
            KeyCode::function(5)
        );
        assert_eq!(KeyCode::function(5).function_index(), Some(5));
    }

    #[test]
    fn unmodelled_named_key_falls_back_to_physical() {
        // `NamedKey::Help` is not in our canonical set, but its physical code is.
        let help = WinitKey::Named(NamedKey::Help);
        assert_eq!(map_named(NamedKey::Help), KeyCode::NONE);
        assert_eq!(
            map_key(&help, PhysicalKey::Code(WinitKeyCode::Help)),
            KeyCode::NONE
        );
        assert_eq!(
            map_key(&help, PhysicalKey::Code(WinitKeyCode::Space)),
            KeyCode::SPACE
        );
    }

    #[test]
    fn unidentified_keys_do_not_panic() {
        let unknown = WinitKey::Unidentified(winit::keyboard::NativeKey::Unidentified);
        assert_eq!(
            map_key(
                &unknown,
                PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified)
            ),
            KeyCode::NONE
        );
    }

    #[test]
    fn modifiers_become_a_chord() {
        assert_eq!(map_modifiers(ModifiersState::empty()), Modifiers::NONE);
        let chord = ModifiersState::SHIFT | ModifiersState::META;
        let mapped = map_modifiers(chord);
        assert!(mapped.contains(Modifiers::SHIFT));
        assert!(mapped.contains(Modifiers::META));
        assert!(!mapped.contains(Modifiers::CONTROL));
    }

    #[test]
    fn buttons_round_trip() {
        assert_eq!(map_mouse_button(MouseButton::Left), PointerButton::Primary);
        assert_eq!(
            map_mouse_button(MouseButton::Right),
            PointerButton::Secondary
        );
        assert_eq!(
            map_mouse_button(MouseButton::Middle),
            PointerButton::Auxiliary
        );
        assert_eq!(map_mouse_button(MouseButton::Back), PointerButton::Other(3));
    }

    #[test]
    fn touch_button_source_is_a_primary_press() {
        assert_eq!(
            map_button_source(ButtonSource::Mouse(MouseButton::Left)),
            PointerButton::Primary
        );
        assert_eq!(
            map_button_source(ButtonSource::Touch {
                finger_id: winit::event::FingerId::from_raw(1),
                force: None,
            }),
            PointerButton::Primary
        );
    }

    #[test]
    fn scroll_delta_is_logical() {
        let (dx, dy) = map_scroll_delta(MouseScrollDelta::LineDelta(1.0, -2.0), 1.0);
        assert_eq!((dx, dy), (20.0, -40.0));
        let (dx, dy) = map_scroll_delta(
            MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(100.0, 50.0)),
            2.0,
        );
        assert_eq!((dx, dy), (50.0, 25.0));
    }

    #[test]
    fn pressed_state() {
        assert!(is_pressed(ElementState::Pressed));
        assert!(!is_pressed(ElementState::Released));
    }
}
