//! iOS platform adapter.

use floem_platform::{HostPlatform, PlatformCapabilities};
use floem_types::PlatformId;

pub use floem_platform::{Capability, Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

pub fn ios_platform() -> HostPlatform {
    HostPlatform::new()
        .with_id(PlatformId::new(11, 1))
        .with_capabilities(PlatformCapabilities::unsupported_a11y_stub())
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_platform::{Ime, Platform, PlatformEvent, WindowConfig};
    use floem_types::{Point, Size};

    #[test]
    fn focus_and_ime_bridge_hooks() {
        let mut p = ios_platform();
        assert!(!p.capabilities().supports(Capability::ACCESSIBILITY));
        assert!(p.capabilities().supports(Capability::IME));

        let win = p.create_window(WindowConfig::default());
        p.set_enabled(win, true);
        p.set_cursor_area(win, Point::ZERO, Size::new(100.0, 24.0));
        p.push_event(PlatformEvent::WindowFocused {
            window: win,
            focused: true,
        });

        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(events.iter().any(|e| matches!(
            e,
            PlatformEvent::WindowFocused { focused: true, .. }
        )));
    }
}
