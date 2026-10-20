//! The ported Material app bar -- an optional leading slot, a title, and
//! optional trailing actions.
//!
//! The shape is the engine's vocabulary, so what is tested here is the
//! translation and the pixels: the tokens reach the style, the bar is painted
//! the colour v1 painted, and the title is drawn in v1's ink at v1's size.
//!
//! ## The tripwires
//!
//! `the_bar_is_painted_in_v1_s_colour` and `the_callers_height_wins_over_the
//! token` would fail if this kit stopped being this kit.

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_app_bar::{MaterialAppBar, MaterialAppBarStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 100.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialAppBar {
    MaterialAppBar::new().with_theme(theme())
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

/// A caller's own slot, for the leading and the actions.
fn slot(text: &'static str) -> Child {
    Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
        Box::new(floom_widgets::Text::new(text))
    })
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialAppBarStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::AppBar) {
        floom_core_theme::component_tokens::ComponentTokens::AppBar(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(
        style.padding_horizontal, tokens.padding_horizontal,
        "the token's padding"
    );
    assert_eq!(
        style.title_font_size, tokens.title_font_size,
        "the token's title size"
    );
    assert_eq!(style.bar_color, theme().colors().surface, "the bar's fill");
    assert_eq!(style.title_color, theme().colors().on_surface, "the title's ink");
    assert_eq!(
        style.action_color, theme().colors().on_surface_variant,
        "the actions' ink"
    );
}

/// The caller's height wins over the token's, as it did in v1.
#[test]
fn the_callers_height_wins_over_the_token() {
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::AppBar) {
        floom_core_theme::component_tokens::ComponentTokens::AppBar(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(kit().resolved_style().height, tokens.height, "the token's by default");
    assert_eq!(
        kit().height(48.0).resolved_style().height,
        48.0,
        "and the caller's when they gave one"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

// --- the pixels -----------------------------------------------------------

/// The bar is painted in v1's colour, and the title in v1's ink.
#[test]
fn the_bar_is_painted_in_v1_s_colour() {
    let resolved = kit().resolved_style();
    let bar = kit().title("Title").view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = bar.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    bar.paint(&mut recorder, id);
    recorder.finish();

    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.bar_color)), "the bar's fill");

    // The title is inside the bar.
    let row = bar.children().remove(0).build();
    let title = row.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = title.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    title.paint(&mut recorder, id);
    recorder.finish();
    let (text, size, ink) = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, color, .. } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the title is painted");
    assert_eq!(text, "Title");
    assert_eq!(size, resolved.title_font_size, "v1's title size");
    assert_eq!(ink, paint(resolved.title_color), "and v1's title ink");
}

/// An elevation draws the shadow v1 drew.
#[test]
fn an_elevation_draws_the_shadow() {
    let flat = kit().title("Title").view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = flat.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    flat.paint(&mut recorder, id);
    recorder.finish();
    assert!(
        !recorder
            .ops()
            .iter()
            .any(|op| matches!(op, PaintOp::Shadow { .. })),
        "no shadow without an elevation"
    );

    let raised = kit().title("Title").elevation(2).view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = raised.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    raised.paint(&mut recorder, id);
    recorder.finish();
    let shadow = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Shadow { blur, spread, .. } => Some((*blur, *spread)),
            _ => None,
        });
    assert_eq!(
        shadow,
        Some((2.0 + 2.0 * 2.0, 1.0)),
        "v1's shadow: the blur grows with the elevation, the spread is one"
    );
}

// --- the structure --------------------------------------------------------

/// The caller's leading slot and actions are mounted.
#[test]
fn the_callers_slots_are_mounted() {
    let bar = kit()
        .title("Title")
        .leading(slot("L"))
        .actions(vec![slot("A")])
        .view();
    let row = bar.children().remove(0).build();
    // The leading, the title, the spacer, the actions.
    assert_eq!(row.children().len(), 4, "the leading, the title, the room and the actions");
    let leading = row.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = leading.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    leading.paint(&mut recorder, id);
    recorder.finish();
    assert!(
        recorder
            .ops()
            .iter()
            .any(|op| matches!(op, PaintOp::Text { text, .. } if text == "L")),
        "the leading slot is mounted"
    );
}

/// The props report the title and the elevation.
#[test]
fn the_props_report_the_title() {
    let kit = kit().title("Title").elevation(3);
    assert_eq!(kit.props().title, "Title");
    assert_eq!(kit.props().elevation, 3);
}

/// The theme: v1's Material app bar read the *app-wide* theme signal, so a
/// change of theme re-themes it with everything else.
#[test]
fn the_theme_is_the_app_wide_one() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_eq!(
        MaterialAppBar::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material app bar read the app-wide signal, not a snapshot"
    );
}
