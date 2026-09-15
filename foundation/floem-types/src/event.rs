//! Platform-neutral input event payloads.
//!
//! These are pure value types shared by every layer that touches input:
//! platform backends mint them, the runtime routes them, and widgets consume
//! them. Keeping them here (instead of in `floem-runtime`) is what lets
//! `floem-core`'s [`EventContext`] expose an event without depending on the
//! runtime — the dependency arrow must point types → core → runtime, never
//! the other way.
//!
//! [`EventContext`]: crate::EventContext

use crate::Point;

/// Keyboard modifier state held during an event.
///
/// Hand-rolled bit set rather than `bitflags`: `floem-types` is required to
/// stay dependency-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const SHIFT: Self = Self(1 << 0);
    pub const CONTROL: Self = Self(1 << 1);
    pub const ALT: Self = Self(1 << 2);
    pub const META: Self = Self(1 << 3);

    pub const NONE: Self = Self(0);

    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

/// Which device produced a pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerKind {
    #[default]
    Mouse,
    Touch,
    Pen,
}

/// Physical button identifier, independent of device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// Left button / single finger.
    Primary,
    /// Right button / secondary action.
    Secondary,
    Middle,
}

/// Lifecycle position of a pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerPhase {
    Enter,
    Down,
    Move,
    Up,
    Leave,
    /// Gesture aborted by the system (e.g. palm rejection).
    Cancel,
}

impl PointerPhase {
    /// Phases that should move hover state.
    pub fn is_hover(self) -> bool {
        matches!(self, PointerPhase::Move | PointerPhase::Enter)
    }

    pub fn ends_interaction(self) -> bool {
        matches!(
            self,
            PointerPhase::Up | PointerPhase::Leave | PointerPhase::Cancel
        )
    }
}

/// A single pointer interaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    pub phase: PointerPhase,
    pub kind: PointerKind,
    /// Set for `Down` and `Up`; `None` for hover/move.
    pub button: Option<PointerButton>,
    /// Position in the target element's coordinate space.
    pub position: Point,
    pub modifiers: Modifiers,
    /// `1` = single click, `2` = double click, …
    pub click_count: u16,
}

impl PointerEvent {
    /// A primary-button press at `position`.
    pub fn primary_down(position: Point) -> Self {
        Self {
            phase: PointerPhase::Down,
            kind: PointerKind::Mouse,
            button: Some(PointerButton::Primary),
            position,
            modifiers: Modifiers::NONE,
            click_count: 1,
        }
    }

    /// A primary-button release at `position`.
    pub fn primary_up(position: Point) -> Self {
        Self {
            phase: PointerPhase::Up,
            ..Self::primary_down(position)
        }
    }

    pub fn hover(position: Point) -> Self {
        Self {
            phase: PointerPhase::Move,
            kind: PointerKind::Mouse,
            button: None,
            position,
            modifiers: Modifiers::NONE,
            click_count: 0,
        }
    }

    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// True for a complete primary click (press or release).
    pub fn is_primary_click(&self) -> bool {
        self.button == Some(PointerButton::Primary)
            && matches!(self.phase, PointerPhase::Down | PointerPhase::Up)
    }
}

/// Wheel / trackpad scroll.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollEvent {
    pub delta_x: f64,
    pub delta_y: f64,
    pub position: Point,
    pub modifiers: Modifiers,
}

/// Keys that have no character representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedKey {
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Shift,
    Control,
    Alt,
    Meta,
}

/// A key identity: either a printable character or a named key.
///
/// `Char` is normalized to lower case for letters; case lives in
/// [`Modifiers::SHIFT`] so keymaps do not need per-case entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Named(NamedKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyPhase {
    Down,
    Up,
    /// Auto-repeat while held.
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub phase: KeyPhase,
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn down(key: Key) -> Self {
        Self {
            phase: KeyPhase::Down,
            key,
            modifiers: Modifiers::NONE,
        }
    }

    pub fn up(key: Key) -> Self {
        Self {
            phase: KeyPhase::Up,
            key,
            modifiers: Modifiers::NONE,
        }
    }

    pub fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Whether this event should commit text into a field.
    pub fn is_text_input(&self) -> bool {
        matches!(self.phase, KeyPhase::Down | KeyPhase::Repeat)
            && matches!(self.key, Key::Char(_))
            && !self.modifiers.contains(Modifiers::CONTROL)
            && !self.modifiers.contains(Modifiers::META)
    }
}

/// A platform-neutral IME (input-method) update delivered to a focused text
/// widget.
///
/// `floem-ime` owns the state machine; this is the wire payload the platform
/// and widgets agree on. It lives in `floem-types` (rather than referencing
/// `floem-ime` directly) precisely so `floem-types` can stay dependency-free —
/// the widget layer maps an `ImeUpdate` into a `floem_ime::ImeEvent` and feeds
/// it to its own [`floem_ime::ImeState`].
///
/// [`floem_ime::ImeState`]: https://docs.rs/floem-ime
#[derive(Debug, Clone, PartialEq)]
pub enum ImeUpdate {
    /// The IME composition panel opened (`true`) or closed (`false`).
    OpenChanged(bool),
    /// The in-progress composition text changed. `caret` and `selection` are
    /// byte offsets, clamped to UTF-8 character boundaries by the consumer.
    Preedit {
        text: String,
        caret: usize,
        selection: Option<(usize, usize)>,
    },
}

/// Top-level input event payload delivered to widgets.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Pointer(PointerEvent),
    Key(KeyEvent),
    Scroll(ScrollEvent),
    /// An input-method composition update for the focused text widget.
    Ime(ImeUpdate),
}

impl From<PointerEvent> for Event {
    fn from(value: PointerEvent) -> Self {
        Event::Pointer(value)
    }
}

impl From<KeyEvent> for Event {
    fn from(value: KeyEvent) -> Self {
        Event::Key(value)
    }
}

impl From<ImeUpdate> for Event {
    fn from(value: ImeUpdate) -> Self {
        Event::Ime(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_are_a_bitset() {
        let none = Modifiers::empty();
        assert!(none.is_empty());

        let cmd_shift = Modifiers::META.union(Modifiers::SHIFT);
        assert!(cmd_shift.contains(Modifiers::META));
        assert!(cmd_shift.contains(Modifiers::SHIFT));
        assert!(!cmd_shift.contains(Modifiers::ALT));
        assert_eq!(cmd_shift.bits(), 0b1001);
    }

    #[test]
    fn primary_click_detection() {
        let at = Point::new(1.0, 2.0);
        assert!(PointerEvent::primary_down(at).is_primary_click());
        assert!(PointerEvent::primary_up(at).is_primary_click());
        assert!(!PointerEvent::hover(at).is_primary_click());
    }

    #[test]
    fn keyup_is_not_text_input() {
        assert!(KeyEvent::down(Key::Char('a')).is_text_input());
        assert!(!KeyEvent::up(Key::Char('a')).is_text_input());
        assert!(!KeyEvent::down(Key::Named(NamedKey::Enter)).is_text_input());
        assert!(
            !KeyEvent::down(Key::Char('a'))
                .with_modifiers(Modifiers::CONTROL)
                .is_text_input()
        );
    }

    #[test]
    fn pointer_phase_classification() {
        assert!(PointerPhase::Enter.is_hover());
        assert!(PointerPhase::Move.is_hover());
        assert!(!PointerPhase::Down.is_hover());
        assert!(PointerPhase::Cancel.ends_interaction());
        assert!(!PointerPhase::Down.ends_interaction());
    }
}
