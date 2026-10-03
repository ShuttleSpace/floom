//! Neutral input, lifecycle, and surface events sent from a backend into Floom.
//!
//! These do not depend on `winit`. Backends translate their native events into
//! [`EmbedderEvent`] values; Floom's runtime converts them into its existing
//! event dispatch, focus, style, and reactive update systems.

use std::path::PathBuf;

use peniko::kurbo::{Point, Size};
use ui_events::{keyboard::KeyboardEvent, pointer::PointerEvent};

use super::{EmbedderWindowId, ImeEvent, SurfaceEvent};
use winit::dpi::PhysicalInsets;

/// Neutral system color scheme.
///
/// Kept separate from `winit::window::Theme` so the protocol stays backend-free;
/// the winit backend maps its theme into this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    /// Light appearance.
    Light,
    /// Dark appearance.
    Dark,
    /// Unknown / not reported by the platform.
    Unknown,
}

/// Window lifecycle events.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowLifecycleEvent {
    /// Window created with initial logical size and scale factor.
    Created {
        /// Initial logical size.
        logical_size: Size,
        /// Initial scale factor.
        scale: f64,
    },
    /// User requested the window close.
    CloseRequested,
    /// Window destroyed.
    Destroyed,
    /// Window moved to a new position.
    Moved(Point),
    /// Window resized to a new logical size.
    Resized(Size),
    /// Scale factor (DPI) changed.
    ScaleChanged(f64),
    /// System theme changed.
    ThemeChanged(Theme),
}

/// Platform window focus changes reported by the backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEvent {
    /// Window gained (`true`) or lost (`false`) platform focus.
    WindowFocused(bool),
}

/// Raw platform gesture events (optional, best-effort).
#[derive(Clone, Debug, PartialEq)]
pub enum GestureEvent {
    /// Two-finger pinch.
    Pinch {
        /// Scale delta (1.0 = no change).
        delta: f64,
        /// Gesture centroid.
        position: Point,
    },
    /// Pan / drag with two fingers.
    Pan {
        /// Translation delta in logical pixels.
        delta: (f64, f64),
        /// Gesture centroid.
        position: Point,
    },
    /// Rotation gesture.
    Rotate {
        /// Rotation delta in radians.
        delta_radians: f64,
        /// Gesture centroid.
        position: Point,
    },
    /// Double tap.
    DoubleTap {
        /// Tap position.
        position: Point,
    },
}

/// Drag-and-drop events.
#[derive(Clone, Debug, PartialEq)]
pub enum DragDropEvent {
    /// Drag entered the window with the given paths.
    Entered {
        /// Dragged file paths.
        paths: Vec<PathBuf>,
        /// Pointer position.
        position: Point,
    },
    /// Drag moved within the window.
    Moved {
        /// Pointer position.
        position: Point,
    },
    /// Drop occurred.
    Dropped {
        /// Dropped file paths.
        paths: Vec<PathBuf>,
        /// Drop position.
        position: Point,
    },
    /// Drag left the window.
    Left,
}

/// The single event type a backend feeds into Floom.
///
/// `PartialEq` is implemented manually because `ui_events::pointer::PointerEvent`
/// does not implement `PartialEq`; for the `Pointer` variant we compare only the
/// target window id (pointer payloads are matched structurally by backends, not
/// by value here).
#[derive(Clone, Debug)]
pub enum EmbedderEvent {
    /// Window lifecycle.
    Window {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Lifecycle event.
        event: WindowLifecycleEvent,
    },
    /// Surface lifecycle.
    Surface {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Surface event.
        event: SurfaceEvent,
    },
    /// Platform focus change.
    Focus {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Focus event.
        event: FocusEvent,
    },
    /// Keyboard input.
    Keyboard {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Neutral keyboard event.
        event: KeyboardEvent,
        /// Whether this is a synthetic key event.
        synthetic: bool,
    },
    /// Pointer input (mouse, touch, pen).
    Pointer {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Neutral pointer event.
        event: PointerEvent,
    },
    /// IME input.
    Ime {
        /// Target window.
        window_id: EmbedderWindowId,
        /// IME event.
        event: ImeEvent,
    },
    /// Raw gesture input.
    Gesture {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Gesture event.
        event: GestureEvent,
    },
    /// Drag and drop.
    DragDrop {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Drag/drop event.
        event: DragDropEvent,
    },
    /// The backend is ready for Floom to render a frame.
    RedrawRequested {
        /// Target window.
        window_id: EmbedderWindowId,
    },
    /// Safe-area insets changed (notch / gesture-bar / status bar). Backends
    /// push this whenever the insets change (e.g. on resize, rotate, or focus)
    /// so core can keep content clear of OS chrome.
    SafeArea {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Insets in physical (device) pixels.
        insets: PhysicalInsets<u32>,
    },
}

impl PartialEq for EmbedderEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                EmbedderEvent::Window {
                    window_id: a_id,
                    event: a_ev,
                },
                EmbedderEvent::Window {
                    window_id: b_id,
                    event: b_ev,
                },
            ) => a_id == b_id && a_ev == b_ev,
            (
                EmbedderEvent::Surface {
                    window_id: a_id,
                    event: a_ev,
                },
                EmbedderEvent::Surface {
                    window_id: b_id,
                    event: b_ev,
                },
            ) => a_id == b_id && a_ev == b_ev,
            (
                EmbedderEvent::Focus {
                    window_id: a_id,
                    event: a_ev,
                },
                EmbedderEvent::Focus {
                    window_id: b_id,
                    event: b_ev,
                },
            ) => a_id == b_id && a_ev == b_ev,
            (
                EmbedderEvent::Keyboard {
                    window_id: a_id,
                    event: a_ev,
                    synthetic: a_syn,
                },
                EmbedderEvent::Keyboard {
                    window_id: b_id,
                    event: b_ev,
                    synthetic: b_syn,
                },
            ) => a_id == b_id && a_ev == b_ev && a_syn == b_syn,
            (
                EmbedderEvent::Pointer {
                    window_id: a_id, ..
                },
                EmbedderEvent::Pointer {
                    window_id: b_id, ..
                },
            ) => a_id == b_id,
            (
                EmbedderEvent::Ime {
                    window_id: a_id,
                    event: a_ev,
                },
                EmbedderEvent::Ime {
                    window_id: b_id,
                    event: b_ev,
                },
            ) => a_id == b_id && a_ev == b_ev,
            (
                EmbedderEvent::Gesture {
                    window_id: a_id,
                    event: a_ev,
                },
                EmbedderEvent::Gesture {
                    window_id: b_id,
                    event: b_ev,
                },
            ) => a_id == b_id && a_ev == b_ev,
            (
                EmbedderEvent::DragDrop {
                    window_id: a_id,
                    event: a_ev,
                },
                EmbedderEvent::DragDrop {
                    window_id: b_id,
                    event: b_ev,
                },
            ) => a_id == b_id && a_ev == b_ev,
            (
                EmbedderEvent::RedrawRequested { window_id: a_id },
                EmbedderEvent::RedrawRequested { window_id: b_id },
            ) => a_id == b_id,
            (
                EmbedderEvent::SafeArea {
                    window_id: a_id,
                    insets: a_ins,
                },
                EmbedderEvent::SafeArea {
                    window_id: b_id,
                    insets: b_ins,
                },
            ) => a_id == b_id && a_ins == b_ins,
            _ => false,
        }
    }
}
