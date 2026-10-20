//! The ported Material date picker -- a field, and the calendar that opens
//! under it.
//!
//! The calendar is a host overlay, so what is tested here is the translation,
//! the field's pixels, and the request the picker makes of the host: the
//! rectangle it asks for, and what the calendar it asked for actually draws.
//!
//! ## The tripwires
//!
//! `the_calendar_is_asked_of_the_host_below_the_field` and
//! `the_calendar_draws_the_month_and_its_days` would fail if this kit stopped
//! being this kit.

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, OverlayKind, Point, Rect, Widget};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{
    dispatch, AllocatingCreateContext, OverlayCall, PaintOp, RecordingUpdateContext,
    SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::date_picker::DateValue;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_date_picker::{MaterialDatePicker, MaterialDatePickerStyle};

/// The field's box, in the tests' coordinate space.
const FIELD: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 56.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialDatePicker {
    MaterialDatePicker::new().with_theme(theme())
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
    let mut recorder = SceneRecorder::new(FIELD);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The rounded fills of a widget's paint, in order.
fn fills(ops: &[PaintOp]) -> Vec<Color> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::RoundedRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

/// The text runs of a widget's paint, in order.
fn texts(ops: &[PaintOp]) -> Vec<(String, f32, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .collect()
}

/// The calendar's height, from the style's own numbers -- the formula the kit
/// uses, written out again so a change in either is a change in both.
fn calendar_height(style: &MaterialDatePickerStyle) -> f64 {
    let header = f64::from(style.arrow_font_size) * 1.2 + f64::from(style.arrow_padding) * 2.0;
    let weekday = f64::from(style.weekday_font_size) * 1.2;
    let grid = 6.0 * f64::from(style.cell_size) + 5.0 * f64::from(style.grid_gap);
    2.0 * f64::from(style.panel_padding)
        + header
        + f64::from(style.panel_gap)
        + weekday
        + f64::from(style.panel_gap)
        + grid
}

/// The calendar's grid cell, as the kit lays it out.
fn cell_rect(style: &MaterialDatePickerStyle, bounds: Rect, row: usize, column: usize) -> Rect {
    let padding = f64::from(style.panel_padding);
    let inner = Rect {
        x0: bounds.x0 + padding,
        y0: bounds.y0 + padding,
        x1: bounds.x1 - padding,
        y1: bounds.y1 - padding,
    };
    let header = f64::from(style.arrow_font_size) * 1.2 + f64::from(style.arrow_padding) * 2.0;
    let weekday = f64::from(style.weekday_font_size) * 1.2;
    let grid_top =
        inner.y0 + header + f64::from(style.panel_gap) + weekday + f64::from(style.panel_gap);
    let cell = f64::from(style.cell_size);
    let gap = (inner.width() - 7.0 * cell) / 6.0;
    let x0 = inner.x0 + column as f64 * (cell + gap);
    let y0 = grid_top + row as f64 * (cell + f64::from(style.grid_gap));
    Rect {
        x0,
        y0,
        x1: x0 + cell,
        y1: y0 + cell,
    }
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialDatePickerStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::DatePicker) {
            floom_core_theme::component_tokens::ComponentTokens::DatePicker(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.height, tokens.height, "the token's field height");
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.min_width, tokens.min_width);
    assert_eq!(
        style.panel_width, tokens.panel_width,
        "the token's panel width"
    );
    assert_eq!(style.cell_size, tokens.cell_size, "the token's cell size");
    assert_eq!(
        style.header_font_size, tokens.header_font_size,
        "the token's header size"
    );
    assert_eq!(style.surface, theme().colors().surface, "the field's fill");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the date's ink"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the placeholder's ink"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the field's border"
    );
    assert_eq!(style.primary, theme().colors().primary, "the chosen day");
    assert_eq!(style.on_primary, theme().colors().on_primary);
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

/// The numbers v1 wrote as literals are reported, because v1's paint used them
/// and v1's style did not report them.
#[test]
fn the_literals_v1_wrote_are_reported() {
    let style = kit().resolved_style();
    assert_eq!(style.padding_horizontal, 12.0, "v1's field padding");
    assert_eq!(style.border_width, 1.0, "v1's border");
    assert_eq!(style.disabled_border_alpha, 97, "v1's disabled border");
    assert_eq!(style.panel_padding, 8.0, "v1's calendar padding");
    assert_eq!(style.panel_border_alpha, 120, "v1's calendar border");
    assert_eq!(style.panel_gap, 6.0, "v1's gap inside the calendar");
    assert_eq!(style.grid_gap, 2.0, "v1's gap between the grid's rows");
    assert_eq!(style.weekday_font_size, 11.0, "v1's weekday size");
    assert_eq!(style.arrow_font_size, 18.0, "v1's arrow size");
    assert_eq!(style.arrow_padding, 4.0, "v1's arrow padding");
    assert_eq!(style.hover_alpha, 16, "v1's hovered day");
}

/// The token v1 never read is still not read: the field keeps v1's literal 12.
#[test]
fn the_unread_token_stays_unread() {
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::DatePicker) {
            floom_core_theme::component_tokens::ComponentTokens::DatePicker(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(
        kit().resolved_style().padding_horizontal,
        12.0,
        "v1 padded the field by 12 and never read the token's {}",
        tokens.padding_horizontal
    );
}

// --- the pixels -----------------------------------------------------------

/// The field is painted in v1's colours: the border, the fill inside it, and
/// the date.
#[test]
fn the_field_is_painted_in_v1_s_colours() {
    let style = kit().resolved_style();
    let ops = ops_of(
        &kit()
            .value(DateValue {
                year: 2026,
                month: 8,
                day: 20,
            })
            .view(),
    );
    let fills = fills(&ops);
    assert_eq!(fills.first(), Some(&paint(style.outline)), "the border");
    assert_eq!(fills.get(1), Some(&paint(style.surface)), "the fill");

    let texts = texts(&ops);
    assert_eq!(texts.len(), 1, "the date is the field's only text");
    assert_eq!(texts[0].0, "2026-08-20", "v1's date format");
    assert_eq!(texts[0].1, style.text_size, "the field's text size");
    assert_eq!(texts[0].2, paint(style.on_surface), "v1's ink for a date");
}

/// With nothing chosen, the placeholder shows in v1's other ink.
#[test]
fn the_placeholder_is_painted_in_the_variant_ink() {
    let style = kit().resolved_style();
    let ops = ops_of(&kit().placeholder("Select date").view());
    let texts = texts(&ops);
    assert_eq!(texts[0].0, "Select date");
    assert_eq!(texts[0].2, paint(style.on_surface_variant));
}

/// The border turns `primary` while the calendar is open, as v1 did.
#[test]
fn the_border_is_primary_while_the_calendar_is_open() {
    let style = kit().resolved_style();
    let mut widget = kit().view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    dispatch(
        &mut widget,
        id,
        FIELD,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    let fills = fills(&ops_of(&widget));
    assert_eq!(
        fills.first(),
        Some(&paint(style.primary)),
        "v1 drew the open field's border in primary"
    );
}

// --- the structure --------------------------------------------------------

/// The calendar is asked of the host, under the field and as wide as v1's
/// panel.
#[test]
fn the_calendar_is_asked_of_the_host_below_the_field() {
    let style = kit().resolved_style();
    let mut widget = kit().view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    dispatch(
        &mut widget,
        id,
        FIELD,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    let mut cx = RecordingUpdateContext::with_bounds(FIELD);
    widget.update(&mut cx, id);

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "the calendar is asked of the host");
    let OverlayCall::Open {
        kind,
        modal,
        bounds,
        ..
    } = opened[0]
    else {
        panic!("the calendar is asked of the host");
    };
    assert_eq!(*kind, OverlayKind::Popup, "a calendar is a popup");
    assert!(!*modal, "a calendar never blocks the page");
    assert_eq!(bounds.x0, FIELD.x0, "at the field's left edge");
    assert_eq!(bounds.y0, FIELD.y1, "under the field");
    assert_eq!(
        bounds.x1,
        FIELD.x0 + f64::from(style.panel_width),
        "as wide as v1's panel"
    );
    assert_eq!(
        bounds.y1,
        FIELD.y1 + calendar_height(&style),
        "and as tall as its contents"
    );
}

/// The calendar draws the month it shows and the days of that month.
#[test]
fn the_calendar_draws_the_month_and_its_days() {
    let mut widget = kit().view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    dispatch(
        &mut widget,
        id,
        FIELD,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    let mut cx = RecordingUpdateContext::with_bounds(FIELD);
    widget.update(&mut cx, id);
    // The vector is held in a local: a borrow of a temporary would not
    // outlive the statement.
    let opened = cx.opened_overlays();
    let OverlayCall::Open { bounds, content, .. } = opened[0] else {
        panic!("the calendar is asked of the host");
    };

    let mut recorder = SceneRecorder::new(*bounds);
    content.paint(&mut recorder, id);
    recorder.finish();
    let texts = texts(recorder.ops());

    assert!(
        texts.iter().any(|(text, _, _)| text == "August 2026"),
        "v1's default month: {texts:?}"
    );
    let days: Vec<u32> = texts
        .iter()
        .filter_map(|(text, _, _)| text.parse::<u32>().ok())
        .collect();
    assert_eq!(days.len(), 31, "August 2026 has 31 days: {days:?}");
    assert_eq!(days.first(), Some(&1), "the first of the month");
    assert_eq!(days.last(), Some(&31), "the last of the month");
}

/// The weekday row is there, and the two arrows that walk the months.
#[test]
fn the_calendar_draws_its_weekday_row_and_its_arrows() {
    let mut widget = kit().view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    dispatch(
        &mut widget,
        id,
        FIELD,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    let mut cx = RecordingUpdateContext::with_bounds(FIELD);
    widget.update(&mut cx, id);
    // The vector is held in a local: a borrow of a temporary would not
    // outlive the statement.
    let opened = cx.opened_overlays();
    let OverlayCall::Open { bounds, content, .. } = opened[0] else {
        panic!("the calendar is asked of the host");
    };

    let mut recorder = SceneRecorder::new(*bounds);
    content.paint(&mut recorder, id);
    recorder.finish();
    let texts = texts(recorder.ops());
    for letter in ["S", "M", "T", "W", "F"] {
        assert!(
            texts.iter().any(|(text, _, _)| text == letter),
            "the weekday row: {texts:?}"
        );
    }
    assert!(
        texts.iter().any(|(text, _, _)| text == "\u{2039}"),
        "the previous arrow: {texts:?}"
    );
    assert!(
        texts.iter().any(|(text, _, _)| text == "\u{203a}"),
        "the next arrow: {texts:?}"
    );
}

/// Choosing a day sets the value, closes the calendar and tells the caller.
#[test]
fn choosing_a_day_sets_the_value_and_closes_the_calendar() {
    let style = kit().resolved_style();
    let reported: Arc<Mutex<Vec<DateValue>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = reported.clone();
    let picker = kit().on_change(move |value| {
        sink.lock().expect("the sink").push(value);
    });
    let state = picker.state();
    let mut widget = picker.view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    dispatch(
        &mut widget,
        id,
        FIELD,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    // A host with no overlays: the calendar is mounted in the tree instead,
    // which is the only way a test can hold on to it.
    let mut cx = RecordingUpdateContext::without_overlay_support();
    widget.update(&mut cx, id);
    let children = widget.children();
    assert_eq!(children.len(), 1, "the in-tree fallback");
    // The in-tree fallback puts the calendar on the overlay's layer, so the
    // calendar is one level deeper than the picker: the layer, then the
    // calendar the layer holds.
    let layer = children[0].build();
    let mut calendar = layer.children().remove(0).build();

    let bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: f64::from(style.panel_width),
        y1: calendar_height(&style),
    };
    // The first of August 2026 is a Saturday, so it sits in the seventh
    // column of the first week.
    let cell = cell_rect(&style, bounds, 0, 6);
    let mut event = TestEventContext::new(
        id,
        bounds,
        Event::Pointer(PointerEvent::primary_up(cell.center())),
    );
    calendar.event(&mut event, id);

    assert_eq!(
        state.lock().expect("the state cell").value,
        Some(DateValue {
            year: 2026,
            month: 8,
            day: 1
        }),
        "the day that was clicked"
    );
    assert!(
        !state.lock().expect("the state cell").open,
        "and the calendar closed"
    );
    assert_eq!(
        reported.lock().expect("the sink").len(),
        1,
        "the caller is told once"
    );
}

/// A disabled picker does not answer.
#[test]
fn a_disabled_picker_does_not_answer() {
    let picker = kit().disabled(true);
    let state = picker.state();
    let mut widget = picker.view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let handled = dispatch(
        &mut widget,
        id,
        FIELD,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    assert!(!handled, "a disabled picker does not answer a click");
    assert!(
        !state.lock().expect("the state cell").open,
        "and the calendar does not open"
    );
}

/// The props report what v1 reported.
#[test]
fn the_props_report_the_date_and_the_placeholder() {
    let kit = kit()
        .value(DateValue {
            year: 2026,
            month: 8,
            day: 20,
        })
        .placeholder("Pick a day")
        .disabled(true);
    assert_eq!(
        kit.props().value,
        Some(DateValue {
            year: 2026,
            month: 8,
            day: 20
        })
    );
    assert_eq!(kit.props().placeholder, "Pick a day");
    assert!(kit.props().disabled);
}

/// The theme: v1's Material date picker read the *app-wide* theme signal, so a
/// change of theme re-themes it with everything else.
#[test]
fn the_theme_is_the_app_wide_one() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_eq!(
        MaterialDatePicker::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material date picker read the app-wide signal, not a snapshot"
    );
}
