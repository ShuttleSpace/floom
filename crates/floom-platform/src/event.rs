//! Platform input / lifecycle events (no OS handles).
//!
//! Every geometry field is in **logical** pixels; a backend divides out its
//! device pixel ratio before pushing an event, so consumers never have to know
//! which OS produced it. Modifier state rides along on each input event instead
//! of having to be tracked separately, because a backend may batch events and a
//! consumer that only sees the tail of a batch must still see the right chord.

use floom_types::{Modifiers, Point, Size, SurfaceId, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
    Other(u8),
}

pub use crate::keymap::KeyCode;

#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    WindowCreated {
        window: WindowId,
        surface: SurfaceId,
        size: Size,
    },
    WindowResized {
        window: WindowId,
        size: Size,
    },
    WindowClosed {
        window: WindowId,
    },
    WindowFocused {
        window: WindowId,
        focused: bool,
    },
    PointerMoved {
        window: WindowId,
        position: Point,
        modifiers: Modifiers,
    },
    PointerButton {
        window: WindowId,
        position: Point,
        button: PointerButton,
        pressed: bool,
        modifiers: Modifiers,
    },
    /// Wheel / trackpad scroll. `position` is the pointer location so a
    /// consumer can route the scroll to the element under it.
    Scroll {
        window: WindowId,
        position: Point,
        delta_x: f64,
        delta_y: f64,
        modifiers: Modifiers,
    },
    Key {
        window: WindowId,
        code: KeyCode,
        pressed: bool,
        /// `true` when the OS is auto-repeating a held key.
        repeat: bool,
        modifiers: Modifiers,
    },
    /// The modifier chord changed without a key event (e.g. Shift was released
    /// while the pointer was outside the window).
    ModifiersChanged {
        window: WindowId,
        modifiers: Modifiers,
    },
    ImePreedit {
        window: WindowId,
        text: String,
    },
    ImeCommit {
        window: WindowId,
        text: String,
    },
    RedrawRequested {
        window: WindowId,
    },
    Suspended,
    Resumed,
}
