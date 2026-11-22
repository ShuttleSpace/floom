// SPDX-License-Identifier: MIT
//! Android platform adapter (independent crate; not an alias of Harmony).
//!
//! Accessibility has its adapter (`accessibility`): the tree arrives, in reading
//! order and with frames, and an activation index can be routed back. What is
//! still missing is the JVM half — an `AccessibilityNodeProvider` to publish to
//! — so the capability bits stay the stub until that exists.

pub mod accessibility;
// The JNI bridge is Android-only: the `jni` crate is a target-gated dependency.
#[cfg(target_os = "android")]
pub mod jni;
pub mod logging;
pub mod packages;

use floom_platform::{HostPlatform, PlatformCapabilities};
use floom_types::PlatformId;

pub use floom_platform::{Capability, Clipboard, Ime, Platform, PlatformEvent, WindowConfig};

/// Build an Android-capability host (headless until NDK surface glue exists).
///
/// On a real Android target this is superseded by `android` (the winit
/// backend): its `android::run` is the real entry point and
/// `android::AndroidPlatform` the real [`Platform`] implementation. This
/// headless constructor stays available on every target so host-side unit
/// tests and tooling can still exercise the capability surface.
pub fn android_platform() -> HostPlatform {
    HostPlatform::new()
        .with_id(PlatformId::new(10, 1))
        .with_capabilities(PlatformCapabilities::unsupported_a11y_stub())
}

/// The `AndroidApp` handle `android_main` receives, re-exported so an app does
/// not have to pin the winit fork's revision just to name the parameter type.
#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

/// Real winit-backed Android backend (Phase 17 / 19.4). Only compiled for the
/// `android` target; on other targets [`android_platform`] provides a headless
/// stand-in. See the module for the `android_main` entry-point seam.
#[cfg(target_os = "android")]
pub use android::{AndroidPlatform, AndroidRunner, run, set_android_app};

#[cfg(target_os = "android")]
mod android;

#[cfg(test)]
mod tests {
    use super::*;
    use floom_platform::{Platform, PlatformEvent, WindowConfig};
    use floom_types::Size;

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
