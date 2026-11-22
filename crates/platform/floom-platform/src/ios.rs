// SPDX-License-Identifier: MIT
//! iOS platform adapter.

pub mod accessibility;

use crate::{HostPlatform, PlatformCapabilities};
use floom_types::PlatformId;

pub use crate::{Capability, Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

pub fn ios_platform() -> HostPlatform {
    // The stand-in uses the same canonical OS profile as the real
    // `floom-platform-ios` backend: iOS is a full OS, not an embedded board.
    // A stand-in still has no soft keyboard or OS a11y tree, so `ime`/touch
    // claims below stay honest — see the test.
    HostPlatform::new()
        .with_id(PlatformId::new(11, 1))
        .with_capabilities(PlatformCapabilities::ios_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ime, Platform, PlatformEvent, WindowConfig};
    use floom_types::{Point, Size};

    #[test]
    fn focus_and_ime_bridge_hooks() {
        let mut p = ios_platform();
        assert!(!p.capabilities().supports(Capability::ACCESSIBILITY));
        // The iOS backend is a stand-in until the UIKit platform lands, and a
        // stand-in has no soft keyboard: its IME is an in-memory map, and
        // claiming the capability would say text typed on a phone reaches the
        // framework when nothing does that yet.
        assert!(!p.capabilities().supports(Capability::IME));

        let win = p.create_window(WindowConfig::default());
        p.set_enabled(win, true);
        p.set_cursor_area(win, Point::ZERO, Size::new(100.0, 24.0));
        p.push_event(PlatformEvent::WindowFocused {
            window: win,
            focused: true,
        });

        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, PlatformEvent::WindowFocused { focused: true, .. }))
        );
    }
}
