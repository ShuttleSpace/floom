//! The ported Cupertino window -- a box of pages, with arrows and dots.
//!

use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Point, Rect, Widget, WidgetAny};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_window::{CupertinoWindow, CupertinoWindowStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoWindow {
    CupertinoWindow::new().with_theme(theme())
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

/// The stroke in a set of paint ops, if there is one.
fn stroke_of(ops: &[PaintOp]) -> Option<(f64, Color)> {
    ops.iter().find_map(|op| match op {
        PaintOp::Path { stroke, .. } => stroke.map(|(width, color)| (width, color)),
        _ => None,
    })
}

/// The text run in a set of paint ops: its text, its size and its ink.
fn text_of(ops: &[PaintOp]) -> (String, f32, Color) {
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("something was painted")
}

/// A page: a slot that builds a text.
fn page(index: usize, text: &'static str) -> Child {
    Child::new(ChildKey::index(index), LayoutRequest::new(), move || {
        Box::new(floom_widgets::Text::new(text))
    })
}

/// Clicks a widget: a press and a release over the same spot.
fn click(widget: &mut dyn WidgetAny) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let point = Point::new(10.0, 10.0);
    let mut cx = TestEventContext::pointer(id, BOX, PointerEvent::primary_down(point));
    widget.event(&mut cx, id);
    let mut cx = TestEventContext::pointer(id, BOX, PointerEvent::primary_up(point));
    widget.event(&mut cx, id);
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoWindowStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Window) {
        floom_core_theme::component_tokens::ComponentTokens::Window(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.dot_size, tokens.dot_size, "the token's dot size");
    assert_eq!(
        style.arrow_size, tokens.arrow_size,
        "the token's arrow size"
    );
    assert_eq!(style.surface, theme().colors().surface, "the box's fill");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the arrows' ink"
    );
    assert_eq!(
        style.primary,
        theme().colors().primary,
        "the active dot's fill"
    );
    assert_eq!(
        style.dot_inactive_color,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface, 102),
        "v1's inactive dot was `on_surface` at 0.4 of 255"
    );
    assert_eq!(
        style.arrow_font_size,
        tokens.arrow_size * 0.6,
        "v1's arrow glyph size"
    );
    assert_eq!(style.arrow_width, 40.0, "v1's arrow box width");
    assert_eq!(style.dot_gap, 6.0, "v1's gap between the dots");
    assert_eq!(style.border_width, 1.0, "v1's outline thickness");
    assert_eq!(
        style.border_color,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().outline, 80),
        "v1's outline colour"
    );
    assert_eq!(style.dots_inset_pct, 2.0, "v1's dots inset");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

// --- the pixels -----------------------------------------------------------

/// The box is painted the surface fill, with v1's outline.
#[test]
fn the_box_is_painted_the_surface_and_the_outline() {
    let resolved = kit().resolved_style();
    let viewer = kit().items(vec![page(0, "One")]).view();
    let ops = ops_of(&viewer);
    assert_eq!(
        fill_of(&ops),
        Some(paint(resolved.surface)),
        "the box's fill"
    );
    let (width, colour) = stroke_of(&ops).expect("v1 drew an outline around the box");
    assert_eq!(
        width,
        f64::from(resolved.border_width),
        "v1's outline thickness"
    );
    assert_eq!(colour, paint(resolved.border_color), "v1's outline colour");
}

// --- the structure --------------------------------------------------------

/// The page on show is the one that was selected.
#[test]
fn the_page_on_show_is_the_one_that_was_selected() {
    let viewer = kit()
        .items(vec![page(0, "One"), page(1, "Two")])
        .selected(1)
        .view();
    let mut children = viewer.children();
    // The page, then the two arrows: v1 put the arrows over the pages.
    assert_eq!(children.len(), 3, "the page and the two arrows");
    let shown = children.remove(0).build();
    let (text, _, _) = text_of(&ops_of(&*shown));
    assert_eq!(text, "Two", "the page that was selected");
}

/// The arrows are there when there is more than one page, and not otherwise.
#[test]
fn the_arrows_are_there_when_there_is_more_than_one_page() {
    let one = kit().items(vec![page(0, "One")]).view();
    assert_eq!(one.children().len(), 1, "one page, no arrows");

    let two = kit().items(vec![page(0, "One"), page(1, "Two")]).view();
    assert_eq!(two.children().len(), 3, "the page and the two arrows");

    let none = kit()
        .items(vec![page(0, "One"), page(1, "Two")])
        .show_arrows(false)
        .view();
    assert_eq!(none.children().len(), 1, "the page alone");
}

/// The dots are there when they are asked for, and one per page.
#[test]
fn the_dots_are_there_when_they_are_asked_for() {
    let viewer = kit()
        .items(vec![page(0, "One"), page(1, "Two")])
        .show_dots(true)
        .view();
    let mut children = viewer.children();
    // The page, the two arrows and the dots.
    assert_eq!(children.len(), 4, "the page, the arrows and the dots");
    let row = children.remove(3).build();
    assert_eq!(row.children().len(), 2, "one dot per page");
}

/// Clicking the next arrow moves the page on and calls the callback.
#[test]
fn clicking_the_next_arrow_moves_the_page_on() {
    let seen = Arc::new(Mutex::new(Vec::<usize>::new()));
    let recorder = seen.clone();
    let viewer = kit()
        .items(vec![page(0, "One"), page(1, "Two")])
        .on_change(move |index| recorder.lock().unwrap().push(index))
        .view();

    let mut children = viewer.children();
    let mut next = children.remove(2).build();
    click(&mut *next);

    assert_eq!(
        *seen.lock().unwrap(),
        vec![1],
        "the callback was called with the page's index"
    );
    assert_eq!(viewer.index(), 1, "the page moved on");

    // And the page on show is the second one.
    let mut children = viewer.children();
    let shown = children.remove(0).build();
    let (text, _, _) = text_of(&ops_of(&*shown));
    assert_eq!(text, "Two", "the second page is on show");
}

/// Clicking the previous arrow at the first page wraps around, because v1's
/// window was continuous by default.
#[test]
fn clicking_the_previous_arrow_wraps_around() {
    let viewer = kit().items(vec![page(0, "One"), page(1, "Two")]).view();
    let mut children = viewer.children();
    let mut previous = children.remove(1).build();
    click(&mut *previous);
    assert_eq!(viewer.index(), 1, "v1's continuous window wrapped around");
}

/// The props report the flags.
#[test]
fn the_props_report_the_flags() {
    let kit = kit()
        .height(120.0)
        .show_arrows(false)
        .show_dots(true)
        .continuous(false);
    assert_eq!(kit.props().height, 120.0);
    assert!(!kit.props().show_arrows);
    assert!(kit.props().show_dots);
    assert!(!kit.props().continuous);
}

// --- the theme source -----------------------------------------------------

/// The theme: v1's Cupertino window took a snapshot of its own design system's
/// theme rather than reading the app-wide signal, so an app-wide theme
/// change does not re-theme it.
#[test]
fn the_theme_is_a_snapshot_not_the_app_wide_signal() {
    assert_ne!(
        CupertinoWindow::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino window took a snapshot, not the app-wide signal"
    );
}
