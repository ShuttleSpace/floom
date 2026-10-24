//! The ported Fluent stepper -- a row of numbered steps, a tick behind the
//! current one, and a connector between each pair.
//!
//! The engine has no stepper, and its tree is built once, so what is tested
//! here is the translation *and* the widget: the tokens reach the style, the
//! markers and the labels are painted in v1's colours, and a change of
//! selection asks the host to rebuild the row.
//!
//! ## The tripwires
//!
//! `the_current_step_is_filled_and_the_ones_ahead_are_not` and
//! `a_change_of_selection_asks_for_a_new_row` would fail if this kit stopped
//! being this kit.

use floom_core::{ElementId, Event, Point, Rect, Widget, WidgetAny};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_stepper::{FluentStepper, FluentStepperStyle};

/// The box the stepper is laid out in.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 120.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentStepper {
    FluentStepper::new().with_theme(theme())
}

fn steps(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
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

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// One widget's paint ops.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(ROW);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// Dispatch an event to a widget held behind a trait object.
///
/// The testkit's own `dispatch` takes a `&mut dyn Widget`, and the stepper's
/// steps are held as `Box<dyn WidgetAny>`; this is the same thing without the
/// upcast.
fn dispatch_any(widget: &mut dyn WidgetAny, id: ElementId, bounds: Rect, event: Event) -> bool {
    let mut cx = TestEventContext::new(id, bounds, event);
    widget.event(&mut cx, id)
}

/// The row the stepper holds.
fn row_of(widget: &dyn Widget) -> Box<dyn WidgetAny> {
    widget.children().remove(0).build()
}

/// One step of the row, unwrapped from the pressable that makes it clickable.
fn step_at(row: &dyn Widget, index: usize) -> Box<dyn WidgetAny> {
    let built = row.children().remove(index).build();
    if built.as_any().downcast_ref::<Pressable>().is_some() {
        built.children().remove(0).build()
    } else {
        built
    }
}

/// The marker's fill.
fn marker_fill(step: &dyn Widget) -> Option<Color> {
    let marker = step.children().remove(0).build();
    ops_of(&*marker).into_iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(color),
        _ => None,
    })
}

/// The marker's content: a tick is a path, a number is text.
fn marker_content(step: &dyn Widget) -> Vec<PaintOp> {
    let marker = step.children().remove(0).build();
    let content = marker.children().remove(0).build();
    ops_of(&*content)
}

/// The label's text, size and ink.
fn label_of(step: &dyn Widget) -> (String, f32, Color) {
    let label = step.children().remove(1).build();
    ops_of(&*label)
        .into_iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text, size, color)),
            _ => None,
        })
        .expect("the label is painted")
}

/// A connector's fill.
fn connector_fill(row: &dyn Widget, index: usize) -> Option<Color> {
    let connector = row.children().remove(index).build();
    ops_of(&*connector).into_iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(color),
        _ => None,
    })
}

// --- the translation ------------------------------------------------------

/// The tokens reach the style.
#[test]
fn the_tokens_reach_the_style() {
    let style: FluentStepperStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Stepper) {
        floom_core_theme::component_tokens::ComponentTokens::Stepper(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.step_size, tokens.step_size, "the marker's diameter");
    assert_eq!(
        style.connector_width, tokens.connector_width,
        "the connector's thickness"
    );
    assert_eq!(style.gap, tokens.gap, "the gap between the steps");
    assert_eq!(
        style.label_font_size, tokens.label_font_size,
        "the label's size"
    );
}

/// The colours reach the style.
#[test]
fn the_colours_reach_the_style() {
    let style = kit().resolved_style();
    let palette = theme();
    let colors = palette.colors();
    assert_eq!(style.primary, colors.primary);
    assert_eq!(style.on_primary, colors.on_primary);
    assert_eq!(style.surface, colors.surface);
    assert_eq!(style.on_surface, colors.on_surface);
    assert_eq!(style.on_surface_variant, colors.on_surface_variant);
    assert_eq!(style.outline, colors.outline);
    assert_eq!(style.outline_variant, colors.outline_variant);
}

/// The numbers v1 wrote as literals are in the style, because v1's style did
/// not report them and the paint used them.
#[test]
fn the_literals_are_in_the_style() {
    let style = kit().resolved_style();
    assert_eq!(style.marker_font_size, 14.0, "the number in a marker");
    assert_eq!(style.marker_radius, 9999.0, "the marker's radius");
    assert_eq!(style.marker_border, 1.0, "the marker's outline");
    assert_eq!(
        style.item_gap, 8.0,
        "the gap between a marker and its label"
    );
    assert_eq!(style.padding_horiz, 8.0, "the padding around the row");
    assert_eq!(style.padding_vert, 12.0, "the padding around the row");
    assert_eq!(
        style.connector_min_width, 16.0,
        "the connector's smallest width"
    );
    assert_eq!(style.check_scale, 0.55, "the tick's size");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

/// The props report the two flags v1 had.
#[test]
fn the_props_report_the_flags() {
    let kit = kit().editable(true).non_linear(true);
    assert!(kit.props().editable);
    assert!(kit.props().non_linear);
}

// --- the structure --------------------------------------------------------

/// Three steps and two connectors: the row alternates.
#[test]
fn the_row_has_a_connector_between_each_pair_of_steps() {
    let widget = kit().items(steps(&["One", "Two", "Three"])).view();
    let row = row_of(&widget);
    assert_eq!(row.children().len(), 5, "three steps and two connectors");
}

/// The row is padded the way v1 padded it.
#[test]
fn the_row_is_padded_the_way_v1_padded_it() {
    let widget = kit().items(steps(&["One"])).view();
    let request = widget
        .layout_request()
        .expect("the stepper asks for its own space");
    let padding = request.padding.expect("the row is padded");
    assert_eq!(padding.left, 8.0);
    assert_eq!(padding.right, 8.0);
    assert_eq!(padding.top, 12.0);
    assert_eq!(padding.bottom, 12.0);
}

/// The gap between the steps is the token's.
#[test]
fn the_gap_between_the_steps_is_the_tokens() {
    let widget = kit().items(steps(&["One", "Two"])).view();
    let row = row_of(&widget);
    let request = row
        .layout_request()
        .expect("the row asks for its own space");
    assert_eq!(request.gap, Some(kit().resolved_style().gap));
}

// --- the pixels -----------------------------------------------------------

/// The current step and the one behind it are filled; the ones ahead are not.
#[test]
fn the_current_step_is_filled_and_the_ones_ahead_are_not() {
    let palette = theme();
    let colors = palette.colors();
    let widget = kit()
        .items(steps(&["One", "Two", "Three"]))
        .selected(1)
        .view();
    let row = row_of(&widget);

    assert_eq!(
        marker_fill(&*step_at(&*row, 0)),
        Some(paint(colors.primary)),
        "the step behind the current one is filled"
    );
    assert_eq!(
        marker_fill(&*step_at(&*row, 2)),
        Some(paint(colors.primary)),
        "the current step is filled"
    );
    assert_eq!(
        marker_fill(&*step_at(&*row, 4)),
        Some(paint(colors.surface)),
        "the step ahead is not"
    );
}

/// The labels follow the selection: `on_surface` for the current step,
/// `primary` behind it, `on_surface_variant` ahead of it.
#[test]
fn the_labels_follow_the_selection() {
    let palette = theme();
    let colors = palette.colors();
    let widget = kit()
        .items(steps(&["One", "Two", "Three"]))
        .selected(1)
        .view();
    let row = row_of(&widget);

    let (text, size, ink) = label_of(&*step_at(&*row, 0));
    assert_eq!(text, "One");
    assert_eq!(
        size,
        kit().resolved_style().label_font_size,
        "the label's size"
    );
    assert_eq!(ink, paint(colors.primary), "a step behind is `primary`");

    let (_, _, ink) = label_of(&*step_at(&*row, 2));
    assert_eq!(
        ink,
        paint(colors.on_surface),
        "the current step is `on_surface`"
    );

    let (_, _, ink) = label_of(&*step_at(&*row, 4));
    assert_eq!(
        ink,
        paint(colors.on_surface_variant),
        "a step ahead is `on_surface_variant`"
    );
}

/// A step behind the current one shows a tick; a step ahead shows its number.
#[test]
fn a_step_behind_shows_a_tick_and_a_step_ahead_shows_its_number() {
    let widget = kit().items(steps(&["One", "Two"])).selected(1).view();
    let row = row_of(&widget);

    let behind = marker_content(&*step_at(&*row, 0));
    assert!(
        behind
            .iter()
            .any(|op| matches!(op, PaintOp::Path { fill: Some(_), .. })),
        "a completed step's marker holds a tick: {behind:?}"
    );

    let ahead = marker_content(&*step_at(&*row, 2));
    let number = ahead
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("the marker holds a number");
    assert_eq!(number, "2", "the second step, numbered from one");
}

/// The connectors follow the selection too.
#[test]
fn the_connectors_follow_the_selection() {
    let palette = theme();
    let colors = palette.colors();
    let widget = kit()
        .items(steps(&["One", "Two", "Three"]))
        .selected(1)
        .view();
    let row = row_of(&widget);

    assert_eq!(
        connector_fill(&*row, 1),
        Some(paint(colors.primary)),
        "the connector behind the current step"
    );
    assert_eq!(
        connector_fill(&*row, 3),
        Some(paint(colors.outline_variant)),
        "the connector ahead of it"
    );
}

// --- the interaction ------------------------------------------------------

/// A change of selection asks the host for a new row: 2.0's tree is built
/// once, so the row's colours cannot follow the selection otherwise.
#[test]
fn a_change_of_selection_asks_for_a_new_row() {
    let mut widget = kit().items(steps(&["One", "Two"])).selected(0).view();
    let id = allocate(&widget);

    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    assert!(
        update.children_dirty().is_empty(),
        "nothing moved, so nothing is rebuilt"
    );

    // The selection moves.
    widget.set_selected(1);

    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    assert!(
        !update.children_dirty().is_empty(),
        "the selection moved, so the row is rebuilt"
    );
}

/// A step behind the current one is clickable when the stepper is editable,
/// and the click moves the selection and fires the callback.
#[test]
fn a_click_on_an_editable_step_moves_the_selection() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(usize::MAX));
    let sink = seen.clone();
    let widget = kit()
        .items(steps(&["One", "Two"]))
        .selected(1)
        .editable(true)
        .on_change(move |index| {
            if let Ok(mut guard) = sink.lock() {
                *guard = index;
            }
        })
        .view();
    let row = row_of(&widget);

    // The first step is behind the current one, so it is the clickable one.
    let mut step = row.children().remove(0).build();
    assert!(
        step.as_any().downcast_ref::<Pressable>().is_some(),
        "an editable step behind the current one is clickable"
    );

    let id = allocate(&*step);
    dispatch_any(
        &mut *step,
        id,
        ROW,
        Event::Pointer(PointerEvent::primary_down(Point::new(5.0, 5.0))),
    );
    dispatch_any(
        &mut *step,
        id,
        ROW,
        Event::Pointer(PointerEvent::primary_up(Point::new(5.0, 5.0))),
    );

    assert_eq!(widget.selected(), 0, "the selection moved");
    assert_eq!(
        *seen.lock().expect("the lock is not poisoned"),
        0,
        "and the callback fired"
    );
}

/// A step that is not clickable is not a pressable at all: v1's rule was that
/// only the steps behind the current one answer, unless the stepper is
/// non-linear.
#[test]
fn a_step_that_may_not_be_clicked_is_not_a_pressable() {
    let widget = kit().items(steps(&["One", "Two"])).selected(1).view();
    let row = row_of(&widget);

    let step = row.children().remove(0).build();
    assert!(
        step.as_any().downcast_ref::<Pressable>().is_none(),
        "the step behind the current one is not clickable when the stepper is not editable"
    );
}

/// A non-linear stepper makes every step clickable.
#[test]
fn a_non_linear_stepper_makes_every_step_clickable() {
    let widget = kit()
        .items(steps(&["One", "Two"]))
        .selected(1)
        .non_linear(true)
        .view();
    let row = row_of(&widget);

    let step = row.children().remove(0).build();
    assert!(
        step.as_any().downcast_ref::<Pressable>().is_some(),
        "a non-linear stepper lets any step be clicked"
    );
}

// --- the theme source -----------------------------------------------------

/// The theme: v1's Fluent stepper took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentStepper::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent stepper took a snapshot, not the app-wide signal"
    );
}
