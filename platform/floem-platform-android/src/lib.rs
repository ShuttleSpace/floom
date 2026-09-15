//! Android platform adapter (independent crate; not an alias of Harmony).
//!
//! Accessibility is capability-limited: the adapter retains the semantics tree
//! but no native bridge is wired, so actions cannot reach the OS.

pub mod accessibility;

use floem_platform::{HostPlatform, PlatformCapabilities};
use floem_types::PlatformId;

pub use floem_platform::{Capability, Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

/// Build an Android-capability host (headless until NDK surface glue exists).
pub fn android_platform() -> HostPlatform {
    HostPlatform::new()
        .with_id(PlatformId::new(10, 1))
        .with_capabilities(PlatformCapabilities::unsupported_a11y_stub())
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_platform::{Platform, PlatformEvent, WindowConfig};
    use floem_types::Size;

    #[test]
    fn lifecycle_and_capabilities() {
        let mut p = android_platform();
        assert!(p.capabilities().supports(Capability::TOUCH));
        assert!(!p.capabilities().supports(Capability::ACCESSIBILITY));

        let win = p.create_window(WindowConfig {
            title: "android".into(),
            size: Size::new(360.0, 640.0),
            resizable: false,
        });
        p.suspend();
        p.resume();

        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(
            events.iter().any(
                |e| matches!(e, PlatformEvent::WindowCreated { window, .. } if *window == win)
            )
        );
        assert!(events.iter().any(|e| matches!(e, PlatformEvent::Suspended)));
        assert!(events.iter().any(|e| matches!(e, PlatformEvent::Resumed)));
    }
}
