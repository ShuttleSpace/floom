//! The ported Cupertino system bar -- the thin status strip at the top of the
//! application shell.
//!
//! The shape is the engine's vocabulary, so what is tested here is the
//! translation and the pixels: the tokens reach the style, the bar is painted
//! in v1's colour, the dark variant swaps the fill, and the content keeps the
//! colour it was given instead of inheriting the bar's.
//!
//! ## The tripwires
//!
//! `the_bar_is_painted_v1_s_colour` and `the_dark_variant_swaps_the_fill`
//! would fail if this kit stopped being this kit.

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Rect, Widget, WidgetAny};
use floom_types::{Color, Insets};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_signal::SignalGet;

use floom_widget_cupertino_system_bar::{CupertinoSystemBar, CupertinoSystemBarStyle};

const BAR: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 22.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoSystemBar {
    CupertinoSystemBar::new().with_theme(theme())
}

fn tokens() -> floom_core_theme::component_tokens::SystemBarTokens {
    match theme().component(floom_core_theme::component_tokens::ComponentId::SystemBar) {
        floom_core_theme::component_tokens::ComponentTokens::SystemBar(t) => t,
        _ => unreachable!("the system bar component must resolve to SystemBarTokens"),
    }
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
fn ops_of(widget: &dyn WidgetAny) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BAR);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The fill a painted box drew.
fn fill_of(ops: &[PaintOp]) -> Option<Color> {
    ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    })
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoSystemBarStyle = kit().resolved_style();
    assert_eq!(style.height, tokens().height, "the token's height");
    assert_eq!(style.pad, tokens().pad, "the token's padding");
    assert_eq!(
        style.surface,
        theme().colors().surface_variant,
        "the bar's fill"
    );
    assert_eq!(
        style.content,
        theme().colors().on_surface_variant,
        "the colour of the content"
    );
    assert!(!style.dark, "v1's default is the light bar");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

/// The props win over the tokens, and the tokens win by default.
#[test]
fn the_props_win_over_the_tokens() {
    assert_eq!(
        kit().resolved_style().height,
        tokens().height,
        "the props defer"
    );
    assert_eq!(kit().height(40.0).resolved_style().height, 40.0);
    assert_eq!(kit().pad(2.0).resolved_style().pad, 2.0);
    assert_eq!(
        kit().height(40.0).request().height.and_then(|h| h.px()),
        Some(40.0),
        "and the request follows the props, not the token"
    );
}

/// The props report the bar.
#[test]
fn the_props_report_the_bar() {
    let kit = kit().height(30.0).dark(true).pad(4.0);
    assert_eq!(kit.props().height, Some(30.0));
    assert!(kit.props().dark);
    assert_eq!(kit.props().pad, Some(4.0));
}

// --- the pixels -----------------------------------------------------------

/// The bar is painted in v1's colour.
#[test]
fn the_bar_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let bar = kit().view();
    assert_eq!(
        fill_of(&ops_of(&bar)),
        Some(paint(resolved.surface)),
        "the bar's fill"
    );
}

/// The dark variant swaps the fill *and* the ink: v1 branched on `dark` in
/// both places.
#[test]
fn the_dark_variant_swaps_the_fill() {
    let dark = kit().dark(true).resolved_style();
    assert_eq!(dark.surface, theme().colors().scrim, "the dark fill");
    assert_eq!(dark.content, theme().colors().on_surface, "the dark ink");
    let bar = kit().dark(true).view();
    assert_eq!(
        fill_of(&ops_of(&bar)),
        Some(paint(theme().colors().scrim)),
        "and the bar is painted with it"
    );
}

/// v1's `.color(fg)` cascaded to the caller's label; 2.0's `Text` carries its
/// own colour, so the content keeps what it was given.
#[test]
fn the_content_keeps_the_colour_it_was_given() {
    let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("12:00").color(Color::rgb(1, 2, 3)))
    });
    let bar = kit().child(child).view();
    assert_eq!(bar.child_count(), 1, "the caller's content is inside");

    let painted = bar.children().remove(0).build();
    let ink = ops_of(&*painted).iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(ink, Some(Color::rgb(1, 2, 3)), "the colour it was given");
    assert_ne!(
        ink,
        Some(paint(kit().resolved_style().content)),
        "and not the bar's content ink: 2.0 has no colour to cascade"
    );
}

// --- the structure --------------------------------------------------------

/// The bar declares its own height and the full width of its parent.
#[test]
fn the_bar_declares_its_own_height_and_full_width() {
    let request = kit().request();
    assert_eq!(
        request.height.and_then(|h| h.px()),
        Some(tokens().height),
        "the token's height"
    );
    assert!(
        request.width.map(|w| w.is_pct()).unwrap_or(false),
        "and the full width of its parent, v1's `width_pct(100.0)`: {request:?}"
    );
    let declared = kit().view().layout_request().expect("a request");
    assert_eq!(declared.height.and_then(|h| h.px()), Some(tokens().height));
}

/// The padding is inside the box, on every side -- as v1 wrote it, whatever
/// the prop's own doc says.
#[test]
fn the_padding_is_inside_the_box() {
    assert_eq!(
        kit().padding(),
        Insets::uniform(f64::from(tokens().pad)),
        "on every side, as v1's `.padding(..)` wrote it"
    );
    let declared = kit().view().layout_request().expect("a request");
    assert_eq!(
        declared.padding,
        Some(Insets::uniform(f64::from(tokens().pad))),
        "and it arrives as the surface's own padding"
    );
}

/// v1's `items_center()`: the *child* is centred, so the alignment lives on
/// the box's request rather than on its fill.
#[test]
fn the_content_is_centred() {
    use floom_widgets::Align;
    let request = kit().request();
    assert_eq!(
        request.cross_align,
        Some(Align::Center),
        "v1's `items_center()` on a row is the cross axis"
    );
}

// --- the layout registry --------------------------------------------------

/// v1 published the bar's height to a process-wide signal, and `cupertino-main`
/// reads it to inset the content region. Dropping it would overlap the content
/// with the bar, so the port keeps it.
#[test]
fn building_the_bar_publishes_its_height() {
    let _ = layout_signal().get().system_bar;
    kit().height(30.0).view();
    assert_eq!(
        layout_signal().get().system_bar,
        30.0,
        "`cupertino-main` reads this to inset the content region"
    );
}

/// The token's own height when the props defer.
#[test]
fn the_published_height_defaults_to_the_token() {
    kit().view();
    assert_eq!(
        layout_signal().get().system_bar,
        tokens().height,
        "the cupertino token for the default theme"
    );
}

// --- the theme ------------------------------------------------------------

/// v1's Cupertino system bar took a *snapshot* of its own design system, so a
/// change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        CupertinoSystemBar::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino system bar took a snapshot, not the app-wide signal"
    );
}
