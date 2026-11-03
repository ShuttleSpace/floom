//! The ported Cupertino main content region -- the area the application's

use floom_core::{ElementId, Length, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::layout::{register_layout, LayoutSlot};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_main::{CupertinoMain, CupertinoMainStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 600.0,
};

/// The Cupertino theme, with Cupertino selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// reads the *current* one -- and `set_design_system` writes a thread-local,
/// so the selection is this test's alone.
fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoMain {
    CupertinoMain::new().with_theme(theme())
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

/// The middle row: the left spacer, the content and the right spacer.
fn middle_of(kit: CupertinoMain) -> Box<dyn WidgetAny> {
    let region = kit.view();
    region.children().remove(1).build()
}

/// The content box: the middle row's second child.
fn content_of(kit: CupertinoMain) -> Box<dyn WidgetAny> {
    let middle = middle_of(kit);
    middle.children().remove(1).build()
}

// --- the translation ------------------------------------------------------

/// The theme's fill and the design system reach the style.
#[test]
fn the_theme_and_the_design_system_reach_the_style() {
    let style: CupertinoMainStyle = kit().resolved_style();
    assert_eq!(style.design_system, "cupertino");
    assert_eq!(
        style.background,
        theme().colors().background,
        "the content's fill"
    );
}

/// The insets are the layout signal's own numbers.
#[test]
fn the_insets_are_the_layout_signals_own_numbers() {
    register_layout(LayoutSlot::SystemBar, 24.0);
    register_layout(LayoutSlot::AppBar, 64.0);
    register_layout(LayoutSlot::Footer, 48.0);
    register_layout(LayoutSlot::NavDrawerLeft, 256.0);
    register_layout(LayoutSlot::NavDrawerRight, 320.0);

    let style = kit().resolved_style();
    assert_eq!(style.top, 88.0, "the system bar plus the app bar");
    assert_eq!(style.bottom, 48.0, "the footer");
    assert_eq!(style.left, 256.0, "the open left drawer");
    assert_eq!(style.right, 320.0, "the open right drawer");
}

// --- the pixels -----------------------------------------------------------

/// The content is painted in the theme's background colour.
#[test]
fn the_content_is_painted_in_the_background() {
    let content = content_of(kit());
    let ops = ops_of(&*content);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(theme().colors().background)),
        "the content's own fill"
    );
}

/// The spacers are sized from the registered insets.
#[test]
fn the_spacers_are_sized_from_the_registered_insets() {
    register_layout(LayoutSlot::AppBar, 64.0);
    register_layout(LayoutSlot::NavDrawerLeft, 256.0);

    let region = kit().view();
    let top = region.children().remove(0).build();
    let request = top
        .layout_request()
        .expect("the top spacer asks for its own space");
    assert_eq!(
        request.height,
        Some(Length::Px(64.0)),
        "the top spacer is as tall as the registered app bar"
    );
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "and as wide as the region"
    );

    let middle = region.children().remove(1).build();
    let left = middle.children().remove(0).build();
    let request = left
        .layout_request()
        .expect("the left spacer asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Px(256.0)),
        "the left spacer is as wide as the open drawer"
    );
    assert_eq!(
        request.height,
        Some(Length::Pct(100.0)),
        "and as tall as the region"
    );
}

// --- the structure --------------------------------------------------------

/// The region is a column of the top spacer, the middle row and the bottom
/// spacer.
#[test]
fn the_region_is_a_column_of_three() {
    let region = kit().view();
    assert_eq!(
        region.children().len(),
        3,
        "the top, the middle and the bottom"
    );
    let request = region
        .layout_request()
        .expect("the region asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "v1's region was the full width"
    );
    assert_eq!(
        request.height,
        Some(Length::Pct(100.0)),
        "and the full height"
    );
}

/// The middle row is a row of the left spacer, the content and the right
/// spacer.
#[test]
fn the_middle_row_is_a_row_of_three() {
    let middle = middle_of(kit());
    assert_eq!(
        middle.children().len(),
        3,
        "the left, the content and the right"
    );
}

/// The content takes the room the spacers leave.
#[test]
fn the_content_takes_the_room_the_spacers_leave() {
    let middle = middle_of(kit());
    let slot = middle.children().remove(1);
    assert_eq!(
        slot.layout.flex_grow,
        Some(1.0),
        "v1's content grew into the row"
    );
}

/// With no child, the content box has none: v1 mounted an `Empty` there, and
/// `Empty` had no size, so this is a structural difference and not a pixel
/// one.
#[test]
fn the_content_box_is_empty_when_there_is_no_child() {
    let content = content_of(kit());
    assert_eq!(
        content.children().len(),
        0,
        "there is nothing to put in the content"
    );
}

// --- the theme source -----------------------------------------------------

/// The theme is a snapshot: v1's Cupertino main took a `MultiTheme` for Cupertino,
/// not the app-wide signal.
#[test]
fn the_theme_is_a_snapshot() {
    assert_ne!(
        CupertinoMain::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino main took a snapshot, not the app-wide signal"
    );
}
