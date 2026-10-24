//! The ported Cupertino infinite scroller -- a viewport that asks for more when
//! the user reaches the end of the content.
//!
//! The shape is the engine's vocabulary plus a kit-local scroller, so what is
//! tested here is the translation and the pixels: the tokens reach the style,
//! the viewport is painted in v1's colours, the loading footer shows v1's
//! label, and the scroller asks for the next page at the end of the content.
//!
//! ## The tripwires
//!
//! `the_viewport_is_painted_v1_s_colour` and
//! `the_scroller_asks_for_more_at_the_end` would fail if this kit stopped
//! being this kit.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, Event, Length, Point, Rect, ScrollEvent, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::with_alpha;

use floom_widget_cupertino_infinite_scroller::{
    shared_height, CupertinoInfiniteScroller, CupertinoInfiniteScrollerStyle,
};

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
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoInfiniteScroller {
    CupertinoInfiniteScroller::new().with_theme(theme())
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
fn text_of(widget: &dyn Widget) -> Option<(String, f32, Color)> {
    ops_of(widget).into_iter().find_map(|op| match op {
        PaintOp::Text {
            text, size, color, ..
        } => Some((text, size, color)),
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

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoInfiniteScrollerStyle = kit().resolved_style();
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::InfiniteScroller)
    {
        floom_core_theme::component_tokens::ComponentTokens::InfiniteScroller(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        style.loading_indicator_size, tokens.loading_indicator_size,
        "the token's loading height"
    );
    assert_eq!(
        style.footer_padding, tokens.footer_padding,
        "the token's footer padding"
    );
    assert_eq!(
        style.surface,
        theme().colors().surface,
        "the viewport's fill"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the loading label's ink"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the viewport's outline"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

// --- the pixels -----------------------------------------------------------

/// The viewport is painted in v1's colours: the fill, and the outline.
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

/// The loading footer shows v1's label, in v1's ink, at v1's size.
#[test]
fn the_loading_footer_shows_v1_s_label() {
    let resolved = kit().resolved_style();
    let surface = kit().loading(true).view();

    let scroller = surface.children().remove(0).build();
    let content = scroller.children().remove(0).build();
    let footer = content.children().remove(0).build();
    let label = footer.children().remove(0).build();

    let (text, size, ink) = text_of(&*label).expect("the loading label is painted");
    assert_eq!(text, "Loading…");
    assert_eq!(size, 13.0, "v1's label size");
    assert_eq!(ink, paint(resolved.on_surface_variant), "v1's label ink");
}

/// The idle footer is an empty box the height of the footer's padding, and it
/// paints nothing.
#[test]
fn the_idle_footer_is_an_empty_box() {
    let resolved = kit().resolved_style();
    let surface = kit().view();

    let scroller = surface.children().remove(0).build();
    let content = scroller.children().remove(0).build();
    let footer = content.children().remove(0).build();

    assert!(
        ops_of(&*footer).is_empty(),
        "the idle footer paints nothing"
    );
    let request = footer
        .layout_request()
        .expect("the footer asks for its own space");
    assert_eq!(
        request.height,
        Some(Length::Px(resolved.footer_padding)),
        "v1's idle footer was the footer's padding tall"
    );
}

// --- the structure --------------------------------------------------------

/// The scroller asks for the next page when the user reaches the end.
#[test]
fn the_scroller_asks_for_more_at_the_end() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let kit = kit()
        .viewport(360.0)
        .content_height(1000.0)
        .on_load_more(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });

    let surface = kit.view();
    let mut scroller = surface.children().remove(0).build();
    let id = ElementId::new(0, 1);
    // The content is 1000 tall in a 360 viewport, so the end is 640 pixels down.
    let mut cx = TestEventContext::new(id, BOX, wheel(640.0));
    scroller.event(&mut cx, id);

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the scroller asked for the next page at the end of the content"
    );
}

/// A page that is already being fetched is not asked for again: that is the
/// re-fire guard, and it is the loading flag.
#[test]
fn the_loading_flag_guards_the_re_fire() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let kit = kit()
        .viewport(360.0)
        .content_height(1000.0)
        .loading(true)
        .on_load_more(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });

    let surface = kit.view();
    let mut scroller = surface.children().remove(0).build();
    let id = ElementId::new(0, 1);
    let mut cx = TestEventContext::new(id, BOX, wheel(640.0));
    scroller.event(&mut cx, id);

    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "a page that is already on its way is not asked for again"
    );
}

/// A content height the host grows after mount is taken up: the scroller
/// re-declares its content, because a box's size is its own layout request
/// and only a reconcile re-reads it.
#[test]
fn a_grown_content_height_is_taken_up() {
    let height = shared_height(0.0);
    let kit = kit().content_height_cell(height.clone());
    let surface = kit.view();
    let mut scroller = surface.children().remove(0).build();

    if let Ok(mut guard) = height.lock() {
        *guard = 1000.0;
    }

    let mut update = RecordingUpdateContext::new();
    let id = ElementId::new(0, 1);
    scroller.update(&mut update, id);

    assert!(
        update.reconciled(),
        "the scroller re-declares its content when the content's height moves"
    );
}

/// The props report the threshold, and the callback slot stays empty: the
/// props' callback is a `Box<dyn Fn()>`, which cannot cross a 2.0 slot.
#[test]
fn the_props_report_the_threshold() {
    let kit = kit().threshold(120.0);
    assert_eq!(kit.props().threshold, 120.0);
    assert!(
        kit.props().on_load_more.is_none(),
        "the props' callback slot stays empty; the kit holds its own handle"
    );
}

/// The loading flag is the host's to write.
#[test]
fn the_loading_flag_is_the_hosts_to_write() {
    let kit = kit();
    let flag = kit.loading_flag();
    assert!(!floom_widgets::read_flag(&flag), "the scroller starts idle");
    floom_widgets::write_flag(&flag, true);
    assert!(
        floom_widgets::read_flag(&flag),
        "and the host can start a fetch"
    );
}

// --- the theme source ---

/// The theme: v1's Cupertino infinite scroller took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not re-theme it.
#[test]
fn the_theme_is_a_snapshot() {
    assert_ne!(
        CupertinoInfiniteScroller::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino infinite scroller took a snapshot, not the app-wide signal"
    );
}
