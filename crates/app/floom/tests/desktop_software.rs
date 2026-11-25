// SPDX-License-Identifier: MIT
//! A build with the software desktop bundle and no GPU bundle runs its windows
//! through the software backend -- no adapter is consulted, and the door the
//! facade opens is the same one `floom::run` uses.

#![cfg(all(feature = "desktop", feature = "renderer-software"))]

use floom::platform;

#[test]
fn the_software_bundle_is_what_a_window_opens_through() {
    let backend = platform::current();
    assert!(
        backend
            .capabilities()
            .supports(platform::Capability::WINDOWING),
        "the software desktop is still a desktop window"
    );
    #[cfg(not(feature = "renderer-gpu"))]
    assert_eq!(
        backend.name(),
        "desktop (winit + software)",
        "without a GPU bundle there is nothing else to draw with"
    );
    #[cfg(feature = "renderer-gpu")]
    assert_eq!(
        backend.name(),
        "desktop (winit + vello)",
        "with both bundles `run` stays on the GPU and `run_desktop_software` is opt-in"
    );
}
