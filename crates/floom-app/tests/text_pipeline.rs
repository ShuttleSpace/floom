//! Text pipeline tests: WidgetHost shapes via floom-text, interns glyph-run
//! resources, and scene text nodes carry the real laid-out extent.
//!
//! These are the resource-side counterpart to the GPU presenter contract: a
//! presenter resolving `SceneNode::Text { glyphs }` through
//! `WidgetHost::resources()` gets everything it needs to rasterise — font
//! blob, font size, glyph ids and pixel-space positions.

use floom_app::WidgetHost;
use floom_layout::{LayoutStyle, Length};
use floom_scene::{ResourceKind, SceneNode};
use floom_types::ResourceId;
use floom_widgets::Text;

fn fixed(width: f64, height: f64) -> LayoutStyle {
    LayoutStyle {
        width: Some(Length::Px(width as f32)),
        height: Some(Length::Px(height as f32)),
        ..LayoutStyle::default()
    }
}

fn host_with_text(content: &str, size: f32) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.mount_root(LayoutStyle::default(), Text::new(content).size(size));
    host.frame();
    host
}

/// Finds the first text scene node and its glyph resource id.
fn first_text_node(host: &WidgetHost) -> (floom_types::Size, ResourceId) {
    for (_id, node) in host.scene().iter() {
        if let SceneNode::Text { size, glyphs, .. } = node {
            return (*size, *glyphs);
        }
    }
    panic!("no text node in the scene");
}

fn glyph_runs(host: &WidgetHost, id: ResourceId) -> Vec<floom_text::GlyphRun> {
    match host.resources().get(id).map(|r| &r.kind) {
        Some(ResourceKind::GlyphRun { runs, .. }) => runs.clone(),
        other => panic!("expected a glyph-run resource, got {other:?}"),
    }
}

#[test]
fn text_frame_produces_a_shaped_glyph_resource() {
    let host = host_with_text("Hello", 24.0);
    let (node_size, glyphs) = first_text_node(&host);
    let runs = glyph_runs(&host, glyphs);

    assert!(!runs.is_empty(), "shaping yields at least one run");
    for run in &runs {
        assert!(!run.glyphs.is_empty(), "each run has glyphs");
        assert!(!run.font.data.as_ref().is_empty(), "run carries font data");
        assert_eq!(run.font_size, 24.0, "run keeps the requested font size");
        assert!(run.origin.x.is_finite() && run.origin.y.is_finite());
    }
    // The node's size is the real laid-out extent (line height ≥ font size),
    // not the old per-character advance estimate.
    assert!(node_size.width > 0.0, "laid-out width is positive");
    assert!(
        node_size.height >= f64::from(24.0),
        "laid-out height covers the font size, got {}",
        node_size.height
    );
}

#[test]
fn identical_text_reuses_the_glyph_resource() {
    let mut host = host_with_text("Count: 0", 14.0);
    let (_size_a, glyphs_a) = first_text_node(&host);
    let units_a = host.resources().used_units();

    host.frame();
    let (_size_b, glyphs_b) = first_text_node(&host);
    let units_b = host.resources().used_units();

    assert_eq!(glyphs_a, glyphs_b, "same (text, size) interns once");
    assert_eq!(units_a, units_b, "no resource growth across frames");
}

#[test]
fn different_font_sizes_get_different_resources() {
    // Two texts, same string, different sizes, mounted with fixed bounds so
    // both actually paint (the shell sizes imperative children explicitly).
    let mut host = WidgetHost::new();
    host.set_size(floom_core::Size::new(240.0, 160.0));
    let root = host.mount_root(LayoutStyle::default(), floom_widgets::Container::new());
    host.mount(root, fixed(200.0, 24.0), Text::new("same text").size(14.0));
    host.mount(root, fixed(200.0, 40.0), Text::new("same text").size(28.0));
    host.frame();

    let mut ids = Vec::new();
    for (_id, node) in host.scene().iter() {
        if let SceneNode::Text { glyphs, .. } = node {
            ids.push(*glyphs);
        }
    }
    assert_eq!(ids.len(), 2, "two text nodes painted");
    assert_ne!(ids[0], ids[1], "size is part of the intern key");
    for id in &ids {
        assert!(!glyph_runs(&host, *id).is_empty());
    }
}

// --- intrinsic sizing -------------------------------------------------------

/// A composite child with no explicit extent used to collapse to zero bounds
/// (the layout engine cannot measure text on its own), so `Text::paint`
/// early-returned and nothing reached the scene. With `Text::layout_request`
/// reporting the measured extent, the rows shrink-wrap their content.
#[test]
fn stack_child_text_sizes_itself_without_explicit_extent() {
    let stack = floom_widgets::Stack::new(floom_core::Axis::Vertical)
        .push(|| Box::new(Text::new("auto sized row")) as Box<dyn floom_core::WidgetAny>)
        .push(|| Box::new(Text::new("second row").size(20.0)) as Box<dyn floom_core::WidgetAny>);
    let mut host = WidgetHost::new();
    host.mount_root(LayoutStyle::default(), stack);
    host.frame();

    let mut text_nodes = 0;
    for (_id, node) in host.scene().iter() {
        if let SceneNode::Text { size, .. } = node {
            text_nodes += 1;
            assert!(
                size.width > 0.0 && size.height > 0.0,
                "painted text must have a real extent, got {size:?}"
            );
        }
    }
    assert_eq!(
        text_nodes, 2,
        "both rows paint text now that they have bounds"
    );
}

/// The intrinsic size is a *default*: explicit sizing in the mount style is
/// never overridden by the measurement.
#[test]
fn explicit_sizing_still_beats_intrinsic() {
    let mut host = WidgetHost::new();
    host.set_size(floom_core::Size::new(240.0, 160.0));
    let root = host.mount_root(LayoutStyle::default(), floom_widgets::Container::new());
    let label = host.mount(
        root,
        fixed(200.0, 24.0),
        Text::new("this text is definitely wider than 200px at 14pt"),
    );
    host.frame();

    let bounds = host.bounds(label);
    assert_eq!(bounds.width(), 200.0, "explicit width wins");
    assert_eq!(bounds.height(), 24.0, "explicit height wins");
}
