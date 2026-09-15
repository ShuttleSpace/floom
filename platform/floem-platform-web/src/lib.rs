//! Web / WASM platform adapter.

pub mod accessibility;

use floem_platform::{HostPlatform, PlatformCapabilities};
use floem_types::PlatformId;

pub use floem_platform::{Capability, Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

/// Detect WebGPU availability.
///
/// Native/CI builds report `false` (software fallback). WASM builds can flip this
/// when a real `navigator.gpu` probe is wired.
pub fn detect_webgpu() -> bool {
    cfg!(all(target_arch = "wasm32", feature = "webgpu-probe"))
}

pub fn web_platform() -> HostPlatform {
    HostPlatform::new()
        .with_id(PlatformId::new(20, 1))
        .with_capabilities(PlatformCapabilities::web_default(detect_webgpu()))
}

/// Hook for DOM / semantic bridge registration (stub).
#[derive(Debug, Default, Clone)]
pub struct DomBridge {
    pub root_selector: Option<String>,
}

impl DomBridge {
    pub fn attach(&mut self, selector: &str) {
        self.root_selector = Some(selector.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_platform::{Platform, PlatformEvent, WindowConfig};
    use floem_types::Size;

    #[test]
    fn webgpu_fallback_and_frame_clock() {
        assert!(!detect_webgpu());
        let mut p = web_platform();
        assert!(!p.capabilities().supports(Capability::WEBGPU));
        assert!(p.capabilities().supports(Capability::SOFTWARE_RENDER));
        assert!(p.capabilities().supports(Capability::NATIVE_VIEW));

        let win = p.create_window(WindowConfig {
            title: "web".into(),
            size: Size::new(1280.0, 720.0),
            resizable: true,
        });
        p.request_frame(win);
        let t0 = p.now_secs();
        assert!(t0 >= 0.0);

        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, PlatformEvent::RedrawRequested { .. }))
        );
    }

    #[test]
    fn dom_bridge_hook() {
        let mut bridge = DomBridge::default();
        bridge.attach("#floem-root");
        assert_eq!(bridge.root_selector.as_deref(), Some("#floem-root"));
    }
}
