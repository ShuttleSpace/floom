#![cfg(feature = "virtual_scroller")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, Event, Point, Rect, ScrollEvent, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Text;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::with_alpha;

use floom_material::widgets::virtual_scroller::{
    MaterialVirtualScroller, MaterialVirtualScrollerStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

/// A row's height, so the window's arithmetic is easy to read.
const ROW: f32 = 44.0;

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialVirtualScroller {
    MaterialVirtualScroller::new().with_theme(theme())
}

/// A scroller over `count` rows, each labelled with its own index.
fn kit_with_rows(count: usize) -> MaterialVirtualScroller {
    let mut kit = kit()
        .item_height(ROW)
        .row_gap(0.0)
        .overscan(4)
        .height(360.0);
    for index in 0..count {
        kit = kit.item(move || Box::new(Text::new(index.to_string())));
    }
    kit
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

/// The text a widget paints, if it paints one.
fn text_of(widget: &dyn Widget) -> Option<String> {
    ops_of(widget).into_iter().find_map(|op| match op {
        PaintOp::Text { text, .. } => Some(text),
        _ => None,
    })
}

/// A wheel of `delta` pixels.
fn wheel(delta: f64) -> Event {
    Event::Scroll(ScrollEvent {
        delta_x: 0.0,
        delta_y: delta,
        position: Point::new(10.0, 10.0),
        modifiers: Default::default(),
    })
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialVirtualScrollerStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::VirtualScroller) {
            floom_core_theme::component_tokens::ComponentTokens::VirtualScroller(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(
        style.item_height,
        kit().props().item_height,
        "the prop's row height wins"
    );
    assert_eq!(
        kit().item_height(0.0).resolved_style().item_height,
        tokens.item_height,
        "and the token's is the fallback"
    );
    assert_eq!(style.row_gap, tokens.row_gap, "the token's row gap");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(
        style.surface,
        theme().colors().surface,
        "the viewport's fill"
    );
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the row's ink"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the viewport's outline"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

/// The style reports the numbers the paint uses: the caller's row height and
/// row gap override the tokens.
#[test]
fn the_style_reports_the_numbers_the_paint_uses() {
    let style = kit()
        .item_height(30.0)
        .row_gap(6.0)
        .height(200.0)
        .resolved_style();
    assert_eq!(style.item_height, 30.0, "the caller's row height");
    assert_eq!(style.row_gap, 6.0, "the caller's row gap");
    assert_eq!(style.viewport_height, 200.0, "the caller's viewport height");
}

#[test]
fn the_viewport_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let surface = kit().view();
    let ops = ops_of(&surface);

    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.surface)), "the viewport's fill");

    let border = ops.iter().find_map(|op| match op {
        PaintOp::Path { stroke, .. } => stroke.map(|(width, color)| (width, color)),
        _ => None,
    });
    let (width, color) = border.expect("the viewport has an outline");
    assert_eq!(width, 1.0, "v1's border was one pixel wide");
    assert_eq!(
        color,
        paint(with_alpha(resolved.outline, 80)),
        "v1's outline was the outline colour at 80"
    );
}

#[test]
fn only_the_windows_rows_are_mounted() {
    let surface = kit_with_rows(100).view();
    let scroller = surface.children().remove(0).build();
    let content = scroller.children().remove(0).build();
    assert_eq!(
        content.children().len(),
        3,
        "the spacer, the rows, the spacer"
    );
    let list = content.children().remove(1).build();

    assert_eq!(
        list.children().len(),
        18,
        "a hundred rows are there, and eighteen are mounted"
    );
}

/// Scrolling mounts a different window: ten rows down, less the overscan, so
/// the first mounted row is the sixth.
#[test]
fn scrolling_mounts_a_different_window() {
    let surface = kit_with_rows(100).view();
    let mut scroller = surface.children().remove(0).build();
    let id = ElementId::new(0, 1);

    let mut cx = TestEventContext::new(id, BOX, wheel(f64::from(ROW) * 10.0));
    scroller.event(&mut cx, id);

    let mut update = RecordingUpdateContext::new();
    scroller.update(&mut update, id);
    assert!(
        update.reconciled(),
        "the scroller re-declares its rows when the window moves"
    );

    let content = scroller.children().remove(0).build();
    let list = content.children().remove(1).build();
    let row = list.children().remove(0).build();
    let label = row.children().remove(0).build();
    assert_eq!(
        text_of(&*label).as_deref(),
        Some("6"),
        "ten rows down, less the overscan of four"
    );
}

/// The scroller asks for more when the user reaches the end of the list.
#[test]
fn the_scroller_asks_for_more_at_the_end() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let kit = kit_with_rows(100).on_reach_end(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });

    let surface = kit.view();
    let mut scroller = surface.children().remove(0).build();
    let id = ElementId::new(0, 1);
    let mut cx = TestEventContext::new(id, BOX, wheel(4040.0));
    scroller.event(&mut cx, id);

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the scroller asked for the next page at the end of the list"
    );
}

/// The props report the numbers, and the callback slot stays empty: the
/// props' callback is a `Box<dyn Fn()>`, which cannot cross a 2.0 slot.
#[test]
fn the_props_report_the_numbers() {
    let kit = kit().item_height(30.0).row_gap(6.0).overscan(2);
    assert_eq!(kit.props().item_height, 30.0);
    assert_eq!(kit.props().row_gap, 6.0);
    assert_eq!(kit.props().overscan, 2);
    assert!(
        kit.props().on_reach_end.is_none(),
        "the props' callback slot stays empty; the kit holds its own handle"
    );
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialVirtualScroller::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material virtual scroller read the app-wide signal, not a snapshot"
    );
}
