//! The ported Fluent hover -- a box that fills while the pointer is over it.
//!
//! The box is the kit's own, because the engine has no "fills while the pointer
//! is over it" widget (see the kit's module documentation), so what is tested
//! here is the translation and the pixels: the tokens reach the style, the fill
//! is the colour v1 painted, and the callback reports the hover.
//!
//! ## The tripwires
//!
//! `the_fill_is_painted_in_v1_s_colour` and
//! `the_callback_reports_the_hover` would fail if this kit stopped being this
//! kit.

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget};
use floom_types::{Color, PointerPhase};
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_hover::{FluentHover, FluentHoverStyle};
use floom_ui_composites::HoverBox;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 60.0,
};

const AT: Point = Point { x: 10.0, y: 10.0 };

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentHover {
    FluentHover::new().with_theme(theme())
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

/// The pointer arriving on the box, and the pointer leaving it again.
fn pointer_over(hover: &mut HoverBox, arriving: bool) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = hover.create(&mut create);
    let mut pointer = PointerEvent::primary_down(AT);
    pointer.phase = if arriving {
        PointerPhase::Enter
    } else {
        PointerPhase::Leave
    };
    let mut cx = TestEventContext::new(id, BOX, pointer.into());
    hover.event(&mut cx, id);
}

/// The fill the box painted, if it painted one.
fn fill_painted(hover: &HoverBox) -> Option<Color> {
    ops_of(hover).into_iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(color),
        _ => None,
    })
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentHoverStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Hover) {
        floom_core_theme::component_tokens::ComponentTokens::Hover(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.highlight_alpha, tokens.highlight_alpha, "the token's alpha");
    assert_eq!(style.transition_ms, tokens.transition_ms, "the token's transition");
    assert_eq!(
        style.highlight_color,
        floom_core_theme::tokens::color_ext::with_alpha(
            theme().colors().on_surface,
            tokens.highlight_alpha as u8
        ),
        "v1's `with_alpha(on_surface, highlight_alpha as u8)`"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

// --- the pixels -----------------------------------------------------------

/// The box paints no fill until the pointer is over it, and then paints
/// v1's fill.
#[test]
fn the_fill_is_painted_in_v1_s_colour() {
    let resolved = kit().resolved_style();
    let mut hover = kit().child(|| Box::new(floom_widgets::Text::new("hello"))).view();

    assert_eq!(
        fill_painted(&hover),
        None,
        "no fill before the pointer arrives"
    );

    pointer_over(&mut hover, true);
    assert_eq!(
        fill_painted(&hover),
        Some(paint(resolved.highlight_color)),
        "v1's fill, under the child"
    );

    pointer_over(&mut hover, false);
    assert_eq!(
        fill_painted(&hover),
        None,
        "and no fill once the pointer has gone"
    );
}

/// The fill covers the whole box, the way v1's container background did.
#[test]
fn the_fill_covers_the_whole_box() {
    let mut hover = kit().child(|| Box::new(floom_widgets::Text::new("hello"))).view();
    pointer_over(&mut hover, true);
    let bounds = ops_of(&hover)
        .into_iter()
        .find_map(|op| match op {
            PaintOp::Rect { bounds, .. } => Some(bounds),
            _ => None,
        })
        .expect("the fill is painted");
    assert_eq!(bounds, BOX);
}

// --- the structure --------------------------------------------------------

/// The child is mounted inside the box.
#[test]
fn the_child_is_mounted_inside_the_box() {
    let hover = kit().child(|| Box::new(floom_widgets::Text::new("hello"))).view();
    assert_eq!(hover.children().len(), 1, "the child");
    let child = hover.children().remove(0).build();
    let ops = ops_of(&*child);
    assert!(
        ops.iter().any(|op| matches!(op, PaintOp::Text { text, .. } if text == "hello")),
        "and it is the caller's own content"
    );
}

/// The callback reports the hover: true on the way in, false on the way out.
#[test]
fn the_callback_reports_the_hover() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let mut hover = kit()
        .child(|| Box::new(floom_widgets::Text::new("hello")))
        .on_hover_change(move |hovered| s.lock().unwrap().push(hovered))
        .view();

    pointer_over(&mut hover, true);
    assert_eq!(seen.lock().unwrap().as_slice(), &[true]);

    pointer_over(&mut hover, false);
    assert_eq!(seen.lock().unwrap().as_slice(), &[true, false]);
}

/// The hover is shared state: a caller that holds the flag drives the fill.
#[test]
fn the_hover_is_shared_state() {
    let kit = kit().child(|| Box::new(floom_widgets::Text::new("hello")));
    let flag = kit.hovered_flag();
    let hover = kit.view();
    assert!(!hover.is_hovered(), "nothing is hovered to begin with");

    floom_widgets::write_flag(&flag, true);
    assert!(hover.is_hovered(), "the caller wrote the flag");
    assert!(
        fill_painted(&hover).is_some(),
        "and the fill follows it, the way v1's signal-driven style did"
    );
}

/// The props report nothing but the callback slot, which the kit leaves empty:
/// 2.0's slots are `Send + Sync` and the props' `Box<dyn Fn(..)>` is not.
#[test]
fn the_props_report_no_callback() {
    let kit = kit().on_hover_change(|_| {});
    assert!(kit.props().on_hover_change.is_none());
}

/// The theme: v1's Fluent hover took a *snapshot* of its own design system's
/// theme, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentHover::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent hover took a snapshot, not the app-wide signal"
    );
}
