//! HarmonyOS platform adapter — **independent** of Android (no shared alias).

pub mod accessibility;

use floem_platform::{Capability, HostPlatform, PlatformCapabilities};
use floem_types::PlatformId;

pub use floem_platform::{Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

pub fn harmony_platform() -> HostPlatform {
    // Distinct capability profile from Android: marks NATIVE_VIEW for ArkUI embedding hooks.
    let mut caps = PlatformCapabilities::unsupported_a11y_stub();
    caps.caps |= Capability::NATIVE_VIEW;
    HostPlatform::new()
        .with_id(PlatformId::new(12, 1))
        .with_capabilities(caps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_platform::{Capability, Platform, WindowConfig};
    use floem_types::PlatformId;

    #[test]
    fn independent_from_android_profile() {
        let p = harmony_platform();
        assert_eq!(p.id(), PlatformId::new(12, 1));
        assert!(p.capabilities().supports(Capability::NATIVE_VIEW));
        assert!(!p.capabilities().supports(Capability::ACCESSIBILITY));
        let mut p = harmony_platform();
        let _ = p.create_window(WindowConfig::default());
    }
}
