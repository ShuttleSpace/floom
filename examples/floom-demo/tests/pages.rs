//! The gallery's headless gate: every page, and every sample on it, mounts and
//! paints.
//!
//! The gallery's whole claim is that it shows every one of the 98 components
//! the `floom-ui` satellite provides. This test is what holds that claim to
//! account: it mounts every page, and every sample on every page, in a
//! headless host, and insists that each one mounts and paints.

use floom::floom_types::Size;
use floom_testkit::TestApp;

use floom_demo::registry;

const WINDOW: Size = Size::new(1280.0, 860.0);

/// Every page of the gallery mounts and paints.
#[test]
fn every_page_mounts_and_paints() {
    let pages = registry::pages();
    assert_eq!(pages.len(), 98, "the gallery holds 98 components");
    for page in pages {
        let mut app = TestApp::unpinned(WINDOW, page.build_shell());
        assert!(app.frame().painted > 0, "{} painted nothing", page.name);
    }
}

/// Every sample of every page mounts and paints on its own.
///
/// Mounting the page would hide a sample that fails to mount behind the
/// shell's chrome, so each sample is mounted by itself. This is the gate that
/// catches a component that can be built but not mounted -- the gap the
/// gallery found in six composites while it was being written.
#[test]
fn every_sample_mounts_and_paints() {
    for page in registry::pages() {
        for sample in &page.samples {
            let mut app = TestApp::new_boxed(WINDOW, (sample.build)());
            assert!(
                app.frame().painted > 0,
                "{}'s {} sample painted nothing",
                page.name,
                sample.title
            );
        }
    }
}
