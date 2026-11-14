// SPDX-License-Identifier: MIT
//! HarmonyOS platform adapter — **independent** of Android (no shared alias).

pub mod accessibility;

use crate::{HostPlatform, PlatformCapabilities};
use floom_types::PlatformId;

pub use crate::{Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

pub fn harmony_platform() -> HostPlatform {
    // The stand-in uses the same canonical OS profile as the real
    // `floom-platform-harmony` backend: HarmonyOS is a full OS, not an embedded
    // board. IME/clipboard/accessibility wait on the ArkTS bridge, so they are
    // not claimed here either (every capability bit is a promise).
    HostPlatform::new()
        .with_id(PlatformId::new(12, 1))
        .with_capabilities(PlatformCapabilities::harmony_default())
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
