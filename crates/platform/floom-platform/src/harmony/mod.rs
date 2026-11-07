// SPDX-License-Identifier: MIT
//! HarmonyOS platform adapter — **independent** of Android (no shared alias).

pub mod accessibility;

use crate::{Capability, HostPlatform, PlatformCapabilities};
use floom_types::PlatformId;

pub use crate::{Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

pub fn harmony_platform() -> HostPlatform {
    // Distinct capability profile from Android: marks NATIVE_VIEW for ArkUI embedding hooks.
    let mut caps = PlatformCapabilities::unsupported_a11y_stub();
    caps.caps |= Capability::NATIVE_VIEW;
    caps.caps |= Capability::IME;
    caps.caps |= Capability::CLIPBOARD;
    HostPlatform::new()
        .with_id(PlatformId::new(12, 1))
        .with_capabilities(caps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Capability, Platform, WindowConfig};
    use floom_types::PlatformId;

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
