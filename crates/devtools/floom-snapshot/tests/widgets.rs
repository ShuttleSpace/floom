// SPDX-License-Identifier: MIT
//! Phase 31 / P1 acceptance — the snapshot engine over **real mounted widgets**.
//!
//! Task 24.2 built the engine (`SceneSnapshot` / `SemanticsSnapshot` /
//! `PixelSnapshot` + a golden-file workflow) and proved it on hand-built scenes.
//! The P1 line "snapshot/interaction tests cover core widgets" stayed PARTIAL
//! because nothing pointed it at `floom-widgets`. This suite is that wiring:
//! every fixture is a widget mounted in a real `WidgetHost`, laid out and
//! painted by the same code path a running app uses.
//!
//! Which layer carries which golden is deliberate. A scene node records the
//! shaped advance width (`width: 78.5791…` for "Hello, Floom"), and that number
//! normally comes from whatever font the host's shaper picked — so every golden
//! here that touches text registers **Ahem** (`fixtures/ahem.ttf`, public
//! domain: one solid box per glyph, one em per advance) and measures in that.
//! The goldens therefore record Floom's layout decisions, not the host's font
//! stack, and are the same on macOS, Linux and Windows.
//!
//! * **Semantics goldens**: role, label and state are decided by Floom, never by
//!   the font.
//! * **Pixel goldens stay glyph-free**: Ahem pins the advances, but the box each
//!   glyph paints would still be compared at a size no real font reproduces.
//!   Text reaches the pixel layer in the *within-run* comparisons below.
//!
//! Regenerate with `UPDATE_SNAPSHOTS=1 cargo test -p floom-snapshot --test widgets`.

use floom_app::WidgetHost;
use floom_core::Axis;
use floom_layout::LayoutStyle;
use floom_snapshot::{
    PixelProbe, PixelSnapshot, ReferencePixels, SceneSnapshot, SemanticsSnapshot, snapshots_dir,
};
use floom_text::{FontFamily, register_font};
use floom_types::{Color, Size};
use floom_widgets::{Button, Container, Stack, Switch, Text, WidgetAny};

/// Mount `widget` as the root of a host of `size`, run one frame, and hand the
/// host back. One frame is enough: layout, paint and semantics are all filled in
/// by `WidgetHost::frame`.
fn mounted(size: Size, widget: impl WidgetAny + 'static) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(size);
    host.mount_root(LayoutStyle::default(), widget);
    host.frame();
    host
}

fn pixel(host: &WidgetHost, size: Size) -> PixelSnapshot {
    // The registry matters: without it `PixelSnapshot` skips Text nodes
    // entirely. See the test that pins that.
    PixelSnapshot::of(host.scene(), Some(host.resources()), size, 1.0)
}

const BOX: Size = Size::new(160.0, 40.0);

/// Ahem, so a golden over text records layout rather than the host's font.
const AHEM: &[u8] = include_bytes!("fixtures/ahem.ttf");

/// Mounts like [`mounted`], with the default family pointed at [`AHEM`] first.
///
/// Layout asks the thread's shaper and painting asks the host's, so both are
/// registered: the point is a golden whose numbers come from a bundled font
/// instead of whichever face the machine happens to call sans-serif.
fn mounted_in_test_font(size: Size, widget: impl WidgetAny + 'static) -> WidgetHost {
    assert!(
        register_font(AHEM.to_vec(), FontFamily::SansSerif) > 0,
        "Ahem must register, or the golden is host-dependent"
    );
    let mut host = WidgetHost::new();
    assert!(
        host.register_font(AHEM.to_vec(), FontFamily::SansSerif) > 0,
        "the host must register Ahem too, or paint and layout disagree"
    );
    host.set_size(size);
    host.mount_root(LayoutStyle::default(), widget);
    host.frame();
    host
}

#[test]
fn text_scene_matches_golden() {
    let host = mounted_in_test_font(BOX, Text::new("Hello, Floom"));
    let snap = SceneSnapshot::of(host.scene());
    assert!(
        !snap.text.trim().is_empty(),
        "a mounted Text must paint something into the scene"
    );
    snap.assert(snapshots_dir!(), "widget_text_scene");
}

/// The checkbox that used to carry this golden moved to
/// `floom-ui-composites`, in the satellite workspace the root one excludes, so
/// the scene this file records is now the switch -- the same claim (two
/// states, a different scene) about the two-state control that is still a
/// primitive.
#[test]
fn switch_scene_differs_between_states_and_the_checked_one_is_golden() {
    let off = mounted_in_test_font(BOX, Switch::new("Remember me").on(false));
    let on = mounted_in_test_font(BOX, Switch::new("Remember me").on(true));
    let off_snap = SceneSnapshot::of(off.scene());
    let on_snap = SceneSnapshot::of(on.scene());
    // The control: if these matched, the "on" flag would not be reaching
    // the scene at all and the golden below would be recording nothing.
    assert_ne!(
        off_snap.hash, on_snap.hash,
        "checking the box must change what it paints"
    );
    on_snap.assert(snapshots_dir!(), "widget_switch_checked_scene");
}

#[test]
fn a_stack_of_three_widgets_is_golden_as_a_composite() {
    let host = mounted_in_test_font(
        Size::new(200.0, 120.0),
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push(|| Box::new(Text::new("title")))
            .push(|| Box::new(Button::new("Save")))
            .push(|| Box::new(Switch::new("Remember me").on(true))),
    );
    let snap = SceneSnapshot::of(host.scene());
    // Three children plus the stack's own box: the composite is doing layout,
    // not painting one blob.
    assert!(
        snap.text.lines().count() >= 3,
        "a vertical stack of three widgets must produce more than one scene node:\n{}",
        snap.text
    );
    snap.assert(snapshots_dir!(), "widget_stack_composite_scene");
}

// Semantics goldens — portable, and the layer that catches a11y regressions
// that no amount of pixel comparison would.

#[test]
fn button_semantics_matches_golden() {
    // The snapshot carries the node's bounds since 2026-11-21, so the label has
    // to be measured the same way on every host.
    let host = mounted_in_test_font(BOX, Button::new("Save"));
    let snap = SemanticsSnapshot::of(host.semantics());
    assert!(
        snap.text.contains("Save"),
        "the button's label must be its accessible name:\n{}",
        snap.text
    );
    snap.assert(snapshots_dir!(), "widget_button_semantics");
}

#[test]
fn a_disabled_widget_reports_a_different_semantics_than_an_enabled_one() {
    let enabled = mounted(BOX, Switch::new("Remember me"));
    let disabled = mounted(BOX, Switch::new("Remember me").disabled(true));
    let a = SemanticsSnapshot::of(enabled.semantics());
    let b = SemanticsSnapshot::of(disabled.semantics());
    assert_ne!(
        a.hash, b.hash,
        "`disabled` must surface in the semantics tree, or a screen reader cannot see it:\n--- enabled ---\n{}\n--- disabled ---\n{}",
        a.text, b.text
    );
}

#[test]
fn container_pixel_matches_golden() {
    // No text anywhere in this fixture, so the hash depends only on geometry
    // and colour — it is reproducible on any machine.
    let size = Size::new(64.0, 32.0);
    let host = mounted(size, Container::new().background(Color::rgb(30, 90, 200)));
    let snap = pixel(&host, size);
    assert_eq!((snap.width, snap.height), (64, 32));
    snap.assert(snapshots_dir!(), "widget_container_pixel");
}

#[test]
fn text_reaches_the_pixel_layer_only_through_the_resource_registry() {
    // This is the bug the Phase 27 gallery found: `PixelSnapshot::of(scene,
    // None, ..)` cannot resolve glyph runs, so it silently *skips* Text nodes —
    // and two different strings then hash identically, which reads as "no
    // visual change" forever. With the registry they must differ.
    let a = mounted(BOX, Text::new("alpha"));
    let b = mounted(BOX, Text::new("beta"));

    let with_a = PixelSnapshot::of(a.scene(), Some(a.resources()), BOX, 1.0);
    let with_b = PixelSnapshot::of(b.scene(), Some(b.resources()), BOX, 1.0);
    assert_ne!(
        with_a.hash, with_b.hash,
        "different strings must render to different pixels once glyphs resolve"
    );

    let without_a = PixelSnapshot::of(a.scene(), None, BOX, 1.0);
    let without_b = PixelSnapshot::of(b.scene(), None, BOX, 1.0);
    assert_eq!(
        without_a.hash, without_b.hash,
        "without the registry the text node is skipped, which is exactly the false \
         negative this assertion exists to detect"
    );
}

#[test]
fn a_widget_state_change_reaches_the_pixel_layer() {
    let size = Size::new(64.0, 32.0);
    let off = pixel(
        &mounted(size, Container::new().background(Color::rgb(30, 90, 200))),
        size,
    );
    let on = pixel(
        &mounted(size, Container::new().background(Color::rgb(200, 90, 30))),
        size,
    );
    assert_ne!(off.hash, on.hash, "colour must reach the pixels");
    // And the pixels really are the colour asked for, not merely "different".
    assert_eq!(off.rgba_pixel(32, 16), Some([30, 90, 200, 255]));
    assert_eq!(on.rgba_pixel(32, 16), Some([200, 90, 30, 255]));
}

#[test]
fn every_widget_golden_is_accounted_for() {
    // `UPDATE_SNAPSHOTS=1` writes whatever it is asked for and never removes
    // anything, so a renamed test leaves an orphan file behind that no golden
    // points at. Keep the list explicit and in one place.
    //
    // While regenerating, the other tests in this file may not have written
    // their file yet (they run in parallel), so this check is the one thing
    // that cannot be verified in the same run that creates them.
    if std::env::var("UPDATE_SNAPSHOTS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return;
    }
    let dir = std::path::Path::new(snapshots_dir!()).join("snapshots");
    let owned: Vec<String> = std::fs::read_dir(&dir)
        .expect("snapshots dir must exist")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("widget_"))
        .collect();
    let expected = [
        "widget_text_scene.snap",
        // The two-state control's golden follows the control: the fixture
        // uses a `Switch`, so the golden is the switch's scene.
        "widget_switch_checked_scene.snap",
        "widget_stack_composite_scene.snap",
        "widget_button_semantics.snap",
        "widget_container_pixel.snap",
    ];
    for want in expected {
        assert!(
            owned.iter().any(|n| n == want),
            "missing golden `{want}` — run with UPDATE_SNAPSHOTS=1"
        );
    }
    assert_eq!(
        owned.len(),
        expected.len(),
        "orphaned widget goldens: {owned:?}"
    );
}
