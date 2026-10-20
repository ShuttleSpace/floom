//! The ported Material image -- a picture, or the alt text on the placeholder
//! fill.
//!
//! What is tested here is the translation and the pixels: the tokens reach the
//! style, the box is painted the placeholder fill, the placeholder is the alt
//! text in v1's ink, and a picture is drawn from the resource the caller
//! registered.
//!
//! ## The tripwires
//!
//! `the_placeholder_is_the_alt_text_on_the_placeholder_fill` and
//! `the_picture_is_drawn_from_the_resource` would fail if this kit stopped
//! being this kit.

use floom_core::{ElementId, Length, Rect, Widget};
use floom_types::{Color, ResourceId, Size};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_image::{MaterialImage, MaterialImageStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialImage {
    MaterialImage::new().with_theme(theme())
}

/// Theme colour -> engine colour.
fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// One widget's paint ops.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The fill in a set of paint ops, if there is one.
fn fill_of(ops: &[PaintOp]) -> Option<Color> {
    ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    })
}

/// The text run in a set of paint ops: its text, its size and its ink.
fn text_of(ops: &[PaintOp]) -> (String, f32, Color) {
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, color, .. } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the placeholder is painted")
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialImageStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Image) {
        floom_core_theme::component_tokens::ComponentTokens::Image(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.corner_radius, tokens.corner_radius, "the token's radius");
    assert_eq!(
        style.placeholder_color, tokens.placeholder_color,
        "the token's placeholder fill"
    );
    assert_eq!(
        style.alt_color,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface_variant, 180),
        "v1's placeholder ink"
    );
    assert_eq!(style.placeholder_font_size, 14.0, "v1's label size");
    assert_eq!(style.width, None, "no width unless the caller gave one");
    assert_eq!(style.height, None, "no height unless the caller gave one");
    assert!(!style.rounded, "not rounded unless the caller said so");
    assert!(!style.cover, "not covering unless the caller said so");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

// --- the pixels -----------------------------------------------------------

/// The box is painted the placeholder fill, and the placeholder is the alt
/// text in v1's ink.
#[test]
fn the_placeholder_is_the_alt_text_on_the_placeholder_fill() {
    let resolved = kit().resolved_style();
    let surface = kit().alt("A cat").view();

    let fill = fill_of(&ops_of(&surface));
    assert_eq!(
        fill,
        Some(paint(resolved.placeholder_color)),
        "the fill behind the picture"
    );

    let mut children = surface.children();
    let placeholder = children.remove(0).build();
    let (text, size, ink) = text_of(&ops_of(&*placeholder));
    assert_eq!(text, "A cat", "v1's placeholder was the alt text");
    assert_eq!(size, resolved.placeholder_font_size, "v1's label size");
    assert_eq!(ink, paint(resolved.alt_color), "v1's placeholder ink");
}

/// A picture is drawn from the resource the caller registered.
#[test]
fn the_picture_is_drawn_from_the_resource() {
    let resource = ResourceId::new(3, 1);
    let surface = kit()
        .alt("A cat")
        .resource(resource)
        .intrinsic_size(Size::new(64.0, 32.0))
        .view();

    let mut children = surface.children();
    let picture = children.remove(0).build();
    let ops = ops_of(&*picture);
    let drawn = ops.iter().find_map(|op| match op {
        PaintOp::Image { resource, .. } => Some(*resource),
        _ => None,
    });
    assert_eq!(drawn, Some(resource), "the picture is the registered resource");
    assert!(
        ops.iter().all(|op| !matches!(op, PaintOp::Text { .. })),
        "a picture is not the placeholder"
    );
}

/// The box is rounded only when the image is rounded.
#[test]
fn the_box_is_rounded_only_when_the_image_is_rounded() {
    let resolved = kit().resolved_style();

    let rounded = kit().rounded(true).view();
    let radius = ops_of(&rounded).iter().find_map(|op| match op {
        PaintOp::RoundedRect { radius, .. } => Some(*radius),
        _ => None,
    });
    assert_eq!(
        radius,
        Some(resolved.corner_radius),
        "v1 rounded the box when the image was rounded"
    );

    let square = kit().rounded(false).view();
    assert!(
        ops_of(&square)
            .iter()
            .all(|op| !matches!(op, PaintOp::RoundedRect { .. })),
        "v1 left the box square otherwise"
    );
}

/// The box is the size the caller asked for.
#[test]
fn the_box_is_the_size_the_caller_asked_for() {
    let resolved = kit().width(120.0).height(80.0).resolved_style();
    let surface = kit().width(120.0).height(80.0).view();
    let request = surface
        .layout_request()
        .expect("the box asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Px(resolved.width.expect("the caller gave a width"))),
        "the caller's width"
    );
    assert_eq!(
        request.height,
        Some(Length::Px(resolved.height.expect("the caller gave a height"))),
        "the caller's height"
    );
}

// --- the structure --------------------------------------------------------

/// The props report the source and the flags.
#[test]
fn the_props_report_the_source_and_the_flags() {
    let kit = kit()
        .src("logo.png")
        .alt("A logo")
        .width(10.0)
        .height(20.0)
        .rounded(true)
        .cover(true);
    assert_eq!(kit.props().src, "logo.png");
    assert_eq!(kit.props().alt, "A logo");
    assert_eq!(kit.props().width, Some(10.0));
    assert_eq!(kit.props().height, Some(20.0));
    assert!(kit.props().rounded);
    assert!(kit.props().cover);
}

// --- the theme source -----------------------------------------------------

/// The theme: v1's Material image read the *app-wide* theme signal, so a
/// change of theme re-themes it with everything else.
#[test]
fn the_theme_is_the_app_wide_one() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_eq!(
        MaterialImage::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material image read the app-wide signal, not a snapshot"
    );
}
