//! `Surface` end to end, through a real host.
//!
//! The widget-level tests prove that a `Surface`'s padding *leaves* as a
//! `LayoutRequest::padding`. That is only half the claim, and the half that has
//! already gone wrong once: `Container` had a `padding` field that left as
//! nothing at all, and `md-card` rendered with its text against the edge while
//! its test passed. These tests close the loop — the host lays out, and the
//! content lands where the padding says it should.
//!
//! The content is located through the painted scene rather than by walking the
//! tree, because `WidgetHost` has no public children accessor and adding one for
//! a test would be the wrong reason.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_scene::SceneNode;
use floom_core::{Align, Axis};
use floom_types::{Color, Insets, Rect, Size};
use floom_widgets::{Stack, Surface, Text};

/// The box a padded surface laid out, and the rect its text landed in.
///
/// Two shapes, because the two claims need two boxes:
///
/// * `laid_out` mounts the surface as the **root**, so the host fills it to the
///   viewport. That is the right shape for "is the content inset inside the
///   box" — the box's position is fixed and only the content moves.
/// * `shrink_wrapped` mounts it **inside a non-stretching stack**, so the box
///   hugs its content. That is the only shape in which "the box grows by the
///   padding" is observable at all, because a root is force-filled and cannot
///   grow (see `WidgetHost::run_layout`).
///
/// The box is identified by the `Rect` it paints, so the test needs no children
/// accessor on `WidgetHost` — and what it measures is what a user would see.
fn laid_out(pad: f64) -> (Rect, Rect) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let root = host.mount_root(
        LayoutStyle::default(),
        Surface::new()
            .background(Color::WHITE)
            .padding_horiz(pad)
            .padding_vert(pad)
            .push(|| Box::new(Text::new("hello"))),
    );
    host.frame();
    (host.bounds(root), text_rect(&host))
}

/// The rect the surface's text landed in.
fn text_rect(host: &WidgetHost) -> Rect {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { .. } => node.bounds_hint(),
            _ => None,
        })
        .expect("the surface painted its content")
}

/// A padded surface that hugs its content, so its own size is the thing under
/// test. Cross-axis `Start` is what stops the stack stretching it back out to
/// the viewport.
fn shrink_wrapped(pad: f64) -> (Rect, Rect) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(move || {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .padding_horiz(pad)
                        .padding_vert(pad)
                        .push(|| Box::new(Text::new("hello"))),
                )
            }),
    );
    host.frame();
    painted_rects(&host)
}

/// The shrink-wrapped surface's own box, identified by the fill it paints.
fn painted_rects(host: &WidgetHost) -> (Rect, Rect) {
    let box_rect = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("the surface painted its fill, so its box is observable");
    (box_rect, text_rect(host))
}

/// The claim. A 16px inset on the box puts the content 16px in from the box's
/// own edge, in both axes — which is what `Container::padding` never managed,
/// because a leaf has nothing to inset.
#[test]
fn a_padded_surface_insets_its_content_inside_the_box() {
    let (box_rect, content) = laid_out(16.0);

    assert!(
        (content.x0 - box_rect.x0 - 16.0).abs() < 0.5,
        "content starts 16px from the box's left edge: box={box_rect:?} content={content:?}"
    );
    assert!(
        (content.y0 - box_rect.y0 - 16.0).abs() < 0.5,
        "and 16px from its top edge: box={box_rect:?} content={content:?}"
    );
}

/// The control for the negative case, and the reason this file exists rather than
/// trusting the widget test: the difference between the two hosts has to be
/// *exactly* the inset. If the padding were dropped, both would produce identical
/// geometry and this would catch it.
#[test]
fn the_inset_is_exactly_the_padding() {
    let (_, padded_text) = laid_out(16.0);
    let (_, bare_text) = laid_out(0.0);

    let shift = padded_text.x0 - bare_text.x0;

    assert!(
        (shift - 16.0).abs() < 0.5,
        "padding moves the content by the inset and nothing else, got {shift}"
    );
}

/// Padding is inside the box, so the box grows by it. This is the observable
/// difference from a margin, and the reason `Surface` carries real padding
/// rather than a margin per child: with two children a margin would also land
/// *between* them.
#[test]
fn padding_grows_the_box_rather_than_moving_it() {
    let (padded_box, padded_text) = shrink_wrapped(16.0);
    let (bare_box, bare_text) = shrink_wrapped(0.0);

    assert!(
        (padded_box.width() - bare_box.width() - 32.0).abs() < 0.5,
        "the box is 32px wider — both sides: {} vs {}",
        padded_box.width(),
        bare_box.width()
    );
    assert!(
        (padded_box.height() - bare_box.height() - 32.0).abs() < 0.5,
        "and 32px taller: {} vs {}",
        padded_box.height(),
        bare_box.height()
    );
    assert!(
        (padded_text.x0 - bare_text.x0 - 16.0).abs() < 0.5,
        "while the content moved in by the inset"
    );
}

/// An unpadded surface must not leave a zero-valued style on the node, and the
/// content must end up flush with the edge rather than at some default inset.
#[test]
fn an_unpadded_surface_leaves_the_content_flush() {
    let (box_rect, content) = laid_out(0.0);
    assert!(
        (content.x0 - box_rect.x0).abs() < 0.5,
        "flush with the edge: box={box_rect:?} content={content:?}"
    );
}

/// A settling host must not re-lay-out every frame because of it. A surface that
/// dirtied its node on each sync would make every card in a tree re-run layout
/// forever, which is the failure mode `FrameStats::relaid` exists to catch.
#[test]
fn a_settled_surface_does_not_relayout_every_frame() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    host.mount_root(
        LayoutStyle::default(),
        Surface::new()
            .padding(Insets::uniform(16.0))
            .push(|| Box::new(Text::new("hello"))),
    );
    host.frame();

    let second = host.frame();
    assert!(
        !second.relaid,
        "a frame with nothing changed must not re-run layout: {second:?}"
    );
}

/// Changing the padding after mount has to reach the layout, not just the pixels.
///
/// This is the case a caller hits when a theme signal fires, and it is the one a
/// `layout_request` read only at mount time would fail. The root is force-filled
/// to the viewport, so the *box* cannot change size here — what has to change is
/// where the content sits inside it, which is the same claim the first test makes
/// for a box built with the padding already on it.
#[test]
fn changing_the_padding_after_mount_moves_the_content() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let root = host.mount_root(
        LayoutStyle::default(),
        Surface::new()
            .background(Color::WHITE)
            .padding(Insets::ZERO)
            .push(|| Box::new(Text::new("hello"))),
    );
    host.frame();
    let before = text_rect(&host).x0;
    assert!(
        (before - host.bounds(root).x0).abs() < 0.5,
        "flush to begin with: content starts at {before}"
    );

    // Push a new inset into the mounted widget, exactly as a kit following a
    // theme change would.
    host.widget_mut::<Surface>(root)
        .expect("the root is a Surface")
        .set_padding(Insets::uniform(16.0));
    let stats = host.frame();

    let after = text_rect(&host).x0;
    assert!(
        (after - before - 16.0).abs() < 0.5,
        "the content moved in by the new inset: {before} -> {after}"
    );
    assert!(
        stats.relaid,
        "and it took a layout pass to do it — a padding that only repainted \
         would leave the content where it was ({stats:?})"
    );
}
