//! Platform input / lifecycle events (no OS handles).

use floem_types::{Point, Size, SurfaceId, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCode(pub u32);

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
    },
    PointerButton {
        window: WindowId,
        position: Point,
        button: PointerButton,
        pressed: bool,
    },
    Key {
        window: WindowId,
        code: KeyCode,
        pressed: bool,
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
