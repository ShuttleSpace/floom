//! The capability flag on a real [`WinitPlatform`], without a window.
//!
//! Gated on the `winit` feature because that is what brings the type in; run it
//! with `cargo test -p floom-platform-desktop --features winit`.
//!
//! The half of the flag that can be checked here is the dangerous half.
//! Advertising `NATIVE_VIEW` makes every `NativeSurface` mount *try* to
//! composite, so a session that has not built a bridge advertising it turns a
//! feature that is merely absent into a thing that fails; the other direction
//! — a bridge with no flag — makes widgets refuse to try at all, which is
//! silent but not wrong. The flag being *set* is
//! `native_view_provider`'s doing and needs a window whose OS handle exists, so
//! it is device case D1 rather than a test here.
#![cfg(feature = "winit")]

use floom_platform::{Capability, Platform};
use floom_platform_desktop::WinitPlatform;

#[test]
fn a_session_with_no_bridge_does_not_advertise_native_views() {
    let platform = WinitPlatform::new();

    assert!(
        !platform.has_native_views(),
        "a platform nobody asked for a provider from has none"
    );
    assert!(
        !platform.capabilities().supports(Capability::NATIVE_VIEW),
        "and must not promise one: every participant mount would fail instead of \
         the feature simply being absent"
    );
    assert_eq!(
        platform.capabilities(),
        floom_platform_desktop::desktop_capabilities(false),
        "the no-bridge answer is the shared function's, not a second derivation"
    );
}
