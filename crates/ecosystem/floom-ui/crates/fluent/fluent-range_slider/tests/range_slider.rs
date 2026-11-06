//! The ported Fluent range slider -- a track with two thumbs, and the band

use floom_core::{ElementId, Event, Point, Rect, Widget};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{dispatch, AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_ui_composites::RangeSlider;
use floom_widget_fluent_range_slider::{FluentRangeSlider, FluentRangeSliderStyle};

const TRACK: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentRangeSlider {
    FluentRangeSlider::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(TRACK);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

#[test]
fn the_tokens_reach_the_style() {
    let style: FluentRangeSliderStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::RangeSlider) {
            floom_core_theme::component_tokens::ComponentTokens::RangeSlider(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(
        style.track_height, tokens.track_height,
        "the track's thickness"
    );
    assert_eq!(style.thumb_size, tokens.thumb_size, "the thumb's size");
    assert_eq!(
        style.thumb_radius, tokens.thumb_radius,
        "the thumb's radius"
    );
}

#[test]
fn the_colours_reach_the_style() {
    let style = kit().resolved_style();
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style.track_color, colors.surface_variant, "the track");
    assert_eq!(style.fill_color, colors.primary, "the band");
    assert_eq!(style.thumb_color, colors.primary, "the thumbs");
}

#[test]
fn a_disabled_slider_dims_to_120_alpha() {
    let style = kit().disabled(true).resolved_style();
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style.track_color.r, colors.surface_variant.r);
    assert_eq!(style.track_color.a, 120, "the track dims");
    assert_eq!(style.fill_color.r, colors.primary.r);
    assert_eq!(style.fill_color.a, 120, "the band dims");
    assert_eq!(style.thumb_color.a, 120, "the thumbs dim");
}

#[test]
fn a_disabled_slider_is_not_the_enabled_one() {
    assert_ne!(
        kit().disabled(true).resolved_style(),
        kit().resolved_style(),
        "the disabled palette is a different palette"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_props_report_the_range() {
    let kit = kit().low(10.0).high(60.0).min(0.0).max(80.0).step(2.0);
    assert_eq!(kit.props().low, 10.0);
    assert_eq!(kit.props().high, 60.0);
    assert_eq!(kit.props().min, 0.0);
    assert_eq!(kit.props().max, 80.0);
    assert_eq!(kit.props().step, 2.0);
}

#[test]
fn the_track_the_band_and_the_two_thumbs_are_painted() {
    let resolved = kit().resolved_style();
    let widget = kit().low(25.0).high(75.0).view();
    let ops = ops_of(&widget);
    assert_eq!(ops.len(), 4, "the track, the band and the two thumbs");

    let (track_color, track_radius) = match &ops[0] {
        PaintOp::RoundedRect { color, radius, .. } => (*color, *radius),
        _ => panic!("the track is a rounded rect"),
    };
    assert_eq!(track_color, paint(resolved.track_color), "the track's fill");
    assert_eq!(
        track_radius,
        resolved.track_height / 2.0,
        "v1's track was a pill: half its thickness as the radius"
    );

    let (band_color, band) = match &ops[1] {
        PaintOp::Rect { color, bounds } => (*color, *bounds),
        _ => panic!("the band is a plain rect"),
    };
    assert_eq!(band_color, paint(resolved.fill_color), "the band's fill");
    assert_eq!(band.x0, 50.0, "the band starts at the low thumb");
    assert_eq!(band.x1, 150.0, "and ends at the high thumb");

    for (index, centre) in [(2usize, 50.0f64), (3usize, 150.0f64)] {
        let (thumb_color, thumb) = match &ops[index] {
            PaintOp::RoundedRect { color, bounds, .. } => (*color, *bounds),
            _ => panic!("a thumb is a rounded rect"),
        };
        assert_eq!(thumb_color, paint(resolved.thumb_color), "the thumb's fill");
        let half = f64::from(resolved.thumb_size) / 2.0;
        assert_eq!(thumb.x0, centre - half, "the thumb is centred on its value");
        assert_eq!(thumb.x1, centre + half, "the thumb is centred on its value");
        assert_eq!(thumb.x1 - thumb.x0, f64::from(resolved.thumb_size));
    }
}

#[test]
fn the_band_runs_between_the_two_thumbs() {
    let widget = kit().low(10.0).high(20.0).view();
    let ops = ops_of(&widget);
    let band = match &ops[1] {
        PaintOp::Rect { bounds, .. } => *bounds,
        _ => panic!("the band is a plain rect"),
    };
    assert!(
        (band.x0 - 20.0).abs() < 0.001,
        "the band starts at the low thumb: {}",
        band.x0
    );
    assert!(
        (band.x1 - 40.0).abs() < 0.001,
        "and ends at the high thumb: {}",
        band.x1
    );
}

#[test]
fn a_drag_moves_the_nearer_thumb() {
    let mut widget = kit().low(25.0).high(75.0).view();
    let id = allocate(&widget);
    let consumed = dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::primary_down(Point::new(20.0, 20.0))),
    );
    assert!(consumed, "the track consumed the press");
    assert_eq!(widget.low(), 10.0, "the low thumb moved to the pointer");
    assert_eq!(widget.high(), 75.0, "the high thumb did not");
}

#[test]
fn the_thumbs_do_not_cross() {
    let mut widget = kit().low(25.0).high(75.0).view();
    let id = allocate(&widget);
    dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::primary_down(Point::new(50.0, 20.0))),
    );
    dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::hover(Point::new(190.0, 20.0))),
    );
    assert_eq!(widget.low(), 75.0, "the low thumb stopped at the high one");
    assert_eq!(widget.high(), 75.0, "and did not pass it");
}

#[test]
fn the_value_lands_on_a_step() {
    let mut widget = kit().low(25.0).high(75.0).step(10.0).view();
    let id = allocate(&widget);
    dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::primary_down(Point::new(63.0, 20.0))),
    );
    assert_eq!(widget.low(), 30.0, "the value snapped to the step");
}

#[test]
fn a_disabled_slider_does_not_answer() {
    let mut widget = kit().low(25.0).high(75.0).disabled(true).view();
    let id = allocate(&widget);
    let consumed = dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::primary_down(Point::new(20.0, 20.0))),
    );
    assert!(!consumed, "a disabled slider consumes nothing");
    assert_eq!(widget.low(), 25.0, "and moves nothing");
    assert_eq!(widget.high(), 75.0, "and moves nothing");
}

#[test]
fn the_callback_reports_the_pair() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new((f32::NAN, f32::NAN)));
    let sink = seen.clone();
    let mut widget = kit()
        .low(25.0)
        .high(75.0)
        .on_change(move |pair| {
            if let Ok(mut guard) = sink.lock() {
                *guard = pair;
            }
        })
        .view();
    let id = allocate(&widget);
    dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::primary_down(Point::new(20.0, 20.0))),
    );
    let reported = *seen.lock().expect("the lock is not poisoned");
    assert_eq!(reported, (10.0, 75.0), "the pair, after the move");
}

#[test]
fn the_two_values_are_shared_with_the_caller() {
    let kit = kit().low(25.0).high(75.0);
    let range = kit.range();
    let mut widget = kit.view();
    let id = allocate(&widget);
    dispatch(
        &mut widget,
        id,
        TRACK,
        Event::Pointer(PointerEvent::primary_down(Point::new(20.0, 20.0))),
    );

    let low = range
        .lock()
        .expect("the lock is not poisoned")
        .get("low")
        .cloned();
    assert_eq!(low.as_deref(), Some("10"), "the shared map holds the value");
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentRangeSlider::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent range slider took a snapshot, not the app-wide signal"
    );
}
