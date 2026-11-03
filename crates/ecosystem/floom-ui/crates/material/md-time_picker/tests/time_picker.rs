
use floom_core::{ElementId, Event, OverlayKind, Point, Rect, Widget};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{
    dispatch, AllocatingCreateContext, OverlayCall, PaintOp, RecordingUpdateContext, SceneRecorder,
    TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::time_picker::TimeValue as ThemeTimeValue;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_time_picker::{MaterialTimePicker, MaterialTimePickerStyle, TimeValue};

/// The field's box, in the tests' coordinate space.
const FIELD: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 56.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialTimePicker {
    MaterialTimePicker::new().with_theme(theme())
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

/// The panel's height, from the style's own numbers -- the formula the kit
/// uses, written out again so a change in either is a change in both.
fn panel_height(style: &MaterialTimePickerStyle) -> f64 {
    2.0 * f64::from(style.panel_padding)
        + 2.0 * f64::from(style.cell_height)
        + 2.0 * f64::from(style.panel_gap)
        + f64::from(style.done_height)
}

/// The panel's box, as the kit lays it out.
fn panel_rect(style: &MaterialTimePickerStyle) -> Rect {
    Rect {
        x0: 0.0,
        y0: 0.0,
        x1: f64::from(style.panel_width),
        y1: panel_height(style),
    }
}

/// The hour stepper that steps down, as the kit lays it out.
fn hour_down(style: &MaterialTimePickerStyle, panel: Rect) -> Rect {
    let padding = f64::from(style.panel_padding);
    let stepper = f64::from(style.stepper_size);
    let row_width = stepper * 2.0 + f64::from(style.value_width) + f64::from(style.row_gap) * 2.0;
    let row_x0 = panel.x0 + padding + (panel.width() - padding * 2.0 - row_width) / 2.0;
    Rect {
        x0: row_x0,
        y0: panel.y0 + padding,
        x1: row_x0 + stepper,
        y1: panel.y0 + padding + f64::from(style.cell_height),
    }
}


/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialTimePickerStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::TimePicker) {
            floom_core_theme::component_tokens::ComponentTokens::TimePicker(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.height, tokens.height, "the token's field height");
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.min_width, tokens.min_width);
    assert_eq!(
        style.panel_width, tokens.panel_width,
        "the token's panel width"
    );
    assert_eq!(
        style.cell_height, tokens.cell_height,
        "the token's row height"
    );
    assert_eq!(style.surface, theme().colors().surface, "the field's fill");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the time's ink"
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
    assert_eq!(style.primary, theme().colors().primary, "the 'Done' button");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_literals_v1_wrote_are_reported() {
    let style = kit().resolved_style();
    assert_eq!(style.padding_horizontal, 12.0, "v1's field padding");
    assert_eq!(style.border_width, 1.0, "v1's border");
    assert_eq!(style.disabled_border_alpha, 97, "v1's disabled border");
    assert_eq!(style.panel_padding, 12.0, "v1's panel padding");
    assert_eq!(style.panel_border_alpha, 120, "v1's panel border");
    assert_eq!(style.panel_gap, 8.0, "v1's gap inside the panel");
    assert_eq!(style.row_gap, 8.0, "v1's gap inside a row");
    assert_eq!(style.stepper_size, 32.0, "v1's stepper");
    assert_eq!(style.stepper_radius, 16.0, "v1's stepper's radius");
    assert_eq!(style.stepper_fill_alpha, 8, "v1's stepper's fill");
    assert_eq!(style.stepper_font_size, 18.0, "v1's stepper's sign");
    assert_eq!(style.value_font_size, 24.0, "v1's number");
    assert_eq!(style.value_width, 48.0, "v1's number's width");
    assert_eq!(style.done_height, 32.0, "v1's 'Done' button");
}


#[test]
fn the_field_is_painted_in_v1_s_colours() {
    let style = kit().resolved_style();
    let ops = ops_of(
        &kit()
            .value(TimeValue {
                hour: 7,
                minute: 30,
            })
            .view(),
    );
    let fills = fills(&ops);
    assert_eq!(fills.first(), Some(&paint(style.outline)), "the border");
    assert_eq!(fills.get(1), Some(&paint(style.surface)), "the fill");

    let texts = texts(&ops);
    assert_eq!(texts.len(), 1, "the time is the field's only text");
    assert_eq!(texts[0].0, "07:30", "v1's time format");
    assert_eq!(texts[0].1, style.text_size);
    assert_eq!(texts[0].2, paint(style.on_surface), "v1's ink for a time");
}

#[test]
fn the_placeholder_is_painted_in_the_variant_ink() {
    let style = kit().resolved_style();
    let ops = ops_of(&kit().placeholder("Select time").view());
    let texts = texts(&ops);
    assert_eq!(texts[0].0, "Select time");
    assert_eq!(texts[0].2, paint(style.on_surface_variant));
}


#[test]
fn the_panel_is_asked_of_the_host_below_the_field() {
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
    assert_eq!(opened.len(), 1, "the panel is asked of the host");
    let OverlayCall::Open {
        kind,
        modal,
        bounds,
        ..
    } = opened[0]
    else {
        panic!("the panel is asked of the host");
    };
    assert_eq!(*kind, OverlayKind::Popup, "a panel is a popup");
    assert!(!*modal, "a panel never blocks the page");
    assert_eq!(bounds.x0, FIELD.x0, "at the field's left edge");
    assert_eq!(bounds.y0, FIELD.y1, "under the field");
    assert_eq!(
        bounds.x1,
        FIELD.x0 + f64::from(style.panel_width),
        "as wide as v1's panel"
    );
    assert_eq!(
        bounds.y1,
        FIELD.y1 + panel_height(&style),
        "and as tall as its contents"
    );
}

/// The panel draws the hour, the minute and the "Done" button.
#[test]
fn the_panel_draws_the_hour_the_minute_and_done() {
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
    let OverlayCall::Open {
        bounds, content, ..
    } = opened[0]
    else {
        panic!("the panel is asked of the host");
    };

    let mut recorder = SceneRecorder::new(*bounds);
    content.paint(&mut recorder, id);
    recorder.finish();
    let texts = texts(recorder.ops());

    assert_eq!(
        texts.iter().filter(|(text, _, _)| text == "00").count(),
        2,
        "the hour and the minute: {texts:?}"
    );
    assert!(
        texts.iter().any(|(text, _, _)| text == "Done"),
        "the button: {texts:?}"
    );
    let signs = texts
        .iter()
        .filter(|(text, _, _)| text == "\u{2212}" || text == "+")
        .count();
    assert_eq!(signs, 4, "two steppers on each of the two rows: {texts:?}");
}

#[test]
fn the_hour_wraps_over_twenty_four() {
    let style = kit().resolved_style();
    let picker = kit().value(TimeValue {
        hour: 0,
        minute: 30,
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
    let mut cx = RecordingUpdateContext::without_overlay_support();
    widget.update(&mut cx, id);
    let children = widget.children();
    assert_eq!(children.len(), 1, "the in-tree fallback");
    let layer = children[0].build();
    let mut panel = layer.children().remove(0).build();

    let bounds = panel_rect(&style);
    let down = hour_down(&style, bounds);
    let mut event = TestEventContext::new(
        id,
        bounds,
        Event::Pointer(PointerEvent::primary_up(down.center())),
    );
    panel.event(&mut event, id);

    assert_eq!(
        state.lock().expect("the state cell").value,
        Some(TimeValue {
            hour: 23,
            minute: 30
        }),
        "v1's hour wrapped over 24"
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
        "and the panel does not open"
    );
}

#[test]
fn the_props_report_the_time_and_the_placeholder() {
    let kit = kit()
        .value(TimeValue {
            hour: 7,
            minute: 30,
        })
        .placeholder("Pick a time")
        .disabled(true);
    assert_eq!(
        kit.props().value,
        Some(ThemeTimeValue {
            hour: 7,
            minute: 30
        })
    );
    assert_eq!(kit.props().placeholder, "Pick a time");
    assert!(kit.props().disabled);
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialTimePicker::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material time picker read the app-wide signal, not a snapshot"
    );
}
