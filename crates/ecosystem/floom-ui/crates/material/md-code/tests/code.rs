//! Material code: the code chip's shape, and the text inside it.
//!
//! ## The claim
//!
//! v1 drew a `Container` holding a `Label`; 2.0 draws a `Surface` holding a
//! `Text`. The assertions are about the painted surface -- its fill, its
//! padding, its radius -- and about the painted text.

use floom_core::Widget;
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, CodeTokens};
use floom_core_theme::global_theme::{DesignSystem, set_design_system};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_code::{
    MaterialCode, MaterialCodeStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 120.0,
    y1: 32.0,
};

/// The Material theme, with Material selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Material's own values has to
/// select Material, and `set_design_system` writes a thread-local, so the
/// selection is this test's alone.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn tokens() -> floom_core_theme::component_tokens::CodeTokens {
    let theme = theme();
    match theme.component(ComponentId::Code) {
        ComponentTokens::Code(t) => t,
        _ => unreachable!(),
    }
}

/// The style resolves the theme's colours and the token's numbers.
#[test]
fn the_style_resolves_the_theme_and_the_tokens() {
    let theme = theme();
    let style: MaterialCodeStyle = MaterialCode::new().with_theme(theme.clone()).resolved_style();
    assert_eq!(style.design_system, "material");
    assert_eq!(style.background, theme.colors().surface_variant);
    assert_eq!(style.text_color, theme.colors().on_surface);
    assert_eq!(style.padding_horizontal, tokens().padding_horizontal);
    assert_eq!(style.padding_vertical, tokens().padding_vertical);
    assert_eq!(style.border_radius, tokens().border_radius);
    assert_eq!(style.font_size, tokens().font_size);
}

/// The `mono` prop is carried and reported, and not honoured: 2.0's text layer
/// has no font family at all, so there is nothing to ask for.
#[test]
fn the_mono_flag_is_carried_and_reported() {
    let kit = MaterialCode::new().with_theme(theme()).mono(true);
    assert!(kit.props().mono, "the prop is what the caller asked for");
    assert!(kit.resolved_style().mono, "and the style reports it");
    assert_eq!(kit.props().mono, true);
}

/// The surface carries the fill, the padding and the radius.
#[test]
fn the_surface_carries_the_fill_and_the_padding() {
    let b = MaterialCode::new().with_theme(theme()).content("x");
    let style = b.resolved_style();
    let mut surface = b.view();
    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();

    let request = surface.layout_request().expect("a surface asks for its own space");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets {
            left: f64::from(style.padding_horizontal),
            right: f64::from(style.padding_horizontal),
            top: f64::from(style.padding_vertical),
            bottom: f64::from(style.padding_vertical),
        }),
        "the token's padding"
    );
    assert_eq!(
        surface.decoration().corner_radius(),
        style.border_radius,
        "and the token's radius"
    );
    assert_eq!(
        surface.decoration().background_color(),
        Some(to_engine(style.background)),
        "and the theme's fill"
    );
}

/// The text is the theme's ink at the token's size.
#[test]
fn the_text_is_the_themes_ink_at_the_tokens_size() {
    let b = MaterialCode::new().with_theme(theme()).content("x");
    let style = b.resolved_style();
    let mut surface = b.view();
    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();
    // The text is the surface's *child*, and a widget paints only itself.
    let mut text = surface.children()[0].build();
    let mut create = AllocatingCreateContext::new();
    let text_id = text.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    text.paint(&mut recorder, text_id);
    recorder.finish();

    let painted = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, size, text, .. } => Some((*color, *size, text.clone())),
        _ => None,
    });
    let (color, size, content) = painted.expect("the text is painted");
    assert_eq!(content, "x");
    assert_eq!(size, style.font_size);
    assert_eq!(color, to_engine(style.text_color));
}

/// Design-system colour -> engine colour.
fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a: c.a }
}
