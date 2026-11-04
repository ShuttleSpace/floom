//! Interaction, state-variant and semantics tests for the Tier-1 controls.

use std::cell::Cell;
use std::rc::Rc;

use floom_accessibility::SemanticsState;
use floom_core::{
    Axis, Child, ChildKey, ElementId, Key, KeyPhase, LayoutRequest, NamedKey, Point, Rect, Size, Widget, WidgetAny,
};
use floom_scene::SceneNode;
use floom_types::{Brush, Color, KeyEvent, Length, Modifiers, PointerEvent, PointerPhase};
use floom_widgets::Stack;
use floom_widgets::controls::{Button, Pressable, Progress, Switch, SwitchPalette};
use floom_widgets::primitives::Text;
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, TestEventContext,
    dispatch, paint_snapshot,
};

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Helper unrolling `dyn Widget` downcasts for assertions after a dispatch.
fn fire_down_up<W: Widget>(widget: &mut W, id: ElementId, at: Point) -> bool {
    let down = dispatch(widget, id, ROW, PointerEvent::primary_down(at).into());
    let up = dispatch(widget, id, ROW, PointerEvent::primary_up(at).into());
    down || up
}

// --- Button ----------------------------------------------------------------

#[test]
fn button_click_sequence_invokes_callback_once() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("Increment").on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    assert!(dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::primary_down(Point::new(10.0, 10.0)).into()
    ));
    assert_eq!(clicks.get(), 0, "press alone must not activate");

    assert!(dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::primary_up(Point::new(10.0, 10.0)).into()
    ));
    assert_eq!(clicks.get(), 1);

    assert!(!button.interaction().pressed);
    assert!(button.interaction().hovered);
}

#[test]
fn button_release_without_press_does_not_activate() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("No-op").on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::primary_up(Point::new(10.0, 10.0)).into(),
    );
    assert_eq!(clicks.get(), 0);
}

#[test]
fn button_disabled_ignores_everything() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("Disabled")
        .disabled(true)
        .on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    fire_down_up(&mut button, id, Point::new(10.0, 10.0));
    assert_eq!(clicks.get(), 0);
    assert!(!button.interaction().hovered);
}

#[test]
fn button_keyboard_activation_requires_focus() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("Go").on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    dispatch(&mut button, id, ROW, KeyEvent::down(Key::Char(' ')).into());
    assert_eq!(clicks.get(), 0, "unfocused controls ignore the keyboard");

    button.set_focused(true);
    dispatch(&mut button, id, ROW, KeyEvent::down(Key::Char(' ')).into());
    assert_eq!(clicks.get(), 1);

    dispatch(
        &mut button,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into(),
    );
    assert_eq!(clicks.get(), 2);
}

#[test]
fn button_reports_disabled_and_focusable_semantics() {
    let enabled = Button::new("Ok");
    assert!(
        enabled
            .semantics_hint()
            .state
            .contains(SemanticsState::ENABLED)
    );
}

#[test]
fn a_focusable_button_takes_the_keyboard_and_a_chrome_button_does_not() {
    // A normal button focuses itself on press, so Enter reaches what the
    // pointer just touched.
    let mut button = Button::new("Ok");
    let id = allocate(&button);
    let mut cx =
        TestEventContext::pointer(id, ROW, PointerEvent::primary_down(Point::new(10.0, 10.0)));
    button.event(&mut cx, id);
    assert_eq!(cx.focus_requests(), &[id]);

    // A chrome button still answers the pointer — the press is consumed and the
    // callback still runs — but it leaves the keyboard where it was.
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut chrome = Button::new("Elements")
        .focusable(false)
        .on_click(move || counter.set(counter.get() + 1));
    let chrome_id = allocate(&chrome);
    let mut cx = TestEventContext::pointer(
        chrome_id,
        ROW,
        PointerEvent::primary_down(Point::new(10.0, 10.0)),
    );
    assert!(chrome.event(&mut cx, chrome_id));
    assert!(
        cx.focus_requests().is_empty(),
        "a chrome button must not take the keyboard"
    );
    let mut cx = TestEventContext::pointer(
        chrome_id,
        ROW,
        PointerEvent::primary_up(Point::new(10.0, 10.0)),
    );
    assert!(chrome.event(&mut cx, chrome_id));
    assert_eq!(clicks.get(), 1, "and it still activates");
}

#[test]
fn a_chrome_button_is_not_offered_as_a_keyboard_stop() {
    let chrome = Button::new("Elements").focusable(false);
    let state = chrome.semantics_hint().state;
    assert!(
        state.contains(SemanticsState::ENABLED),
        "still an enabled control"
    );
    assert!(
        !state.contains(SemanticsState::FOCUSABLE),
        "a control the keyboard cannot reach must not advertise FOCUSABLE"
    );
    // A disabled button says so either way.
    let disabled = Button::new("Elements").focusable(false).disabled(true);
    assert!(
        disabled
            .semantics_hint()
            .state
            .contains(SemanticsState::DISABLED)
    );
}

// --- Switch ----------------------------------------------------------------

/// `Checkbox` used to open this section. Its two cases moved with the widget to
/// `floom-ui-composites/tests/checkbox.rs`: the control is no longer reachable
/// from the root workspace, so a test of it has to live where the widget does.
#[test]
fn switch_knob_tracks_state() {
    let mut switch = Switch::new("Wi-Fi");
    let id = allocate(&switch);

    let off_knob = switch.knob_bounds(ROW);
    fire_down_up(&mut switch, id, Point::new(4.0, 4.0));
    assert!(switch.is_on());

    let on_knob = switch.knob_bounds(ROW);
    assert!(on_knob.x0 > off_knob.x0, "knob slides right when on");
    assert!(switch.track_bounds(ROW).width() <= ROW.width());
}

/// **The control for every override test below.** Without any override the
/// switch derives half-as-tall and a knob that fills the track, flush to one
/// end. Every override assertion is therefore "and *not* this", and the three
/// numbers the kits use (52 / 32 / 24) satisfy none of the derivations — which is
/// why they need overrides rather than a scaled default.
#[test]
fn a_switch_derives_its_geometry_by_default() {
    let switch = Switch::new("Wi-Fi");
    let track = switch.track_bounds(ROW);
    assert_eq!(track.width(), 40.0, "the default track width");
    assert_eq!(track.height(), 20.0, "half the width, derived");
    assert_eq!(
        switch.knob_bounds(ROW).height(),
        20.0,
        "knob fills the track"
    );
}

/// An explicit track size is honoured as given, not rounded toward the
/// derivation. 52 × 32 is the kit geometry, and `52 * 0.5` is 26 — so a switch
/// that derived here would be 6px short and every kit would silently get a
/// squashed track.
#[test]
fn an_explicit_track_size_is_used_verbatim() {
    let track = Switch::new("x").track_size(52.0, 32.0).track_bounds(ROW);
    assert_eq!((track.width(), track.height()), (52.0, 32.0));
    assert_eq!(
        track.height(),
        track.width() * 0.5 + 6.0,
        "and not re-derived"
    );
}

/// The switch reproduces the kit geometry exactly: track 52 × 32, knob 24,
/// inset 2 — the knob does not fill the track and does not touch either end.
#[test]
fn switch_tokens_reproduce_kit_geometry() {
    let off = Switch::new("x")
        .track_size(52.0, 32.0)
        .knob_size(24.0)
        .knob_inset(2.0)
        .knob_bounds(ROW);
    let on = Switch::new("x")
        .track_size(52.0, 32.0)
        .knob_size(24.0)
        .knob_inset(2.0)
        .on(true)
        .knob_bounds(ROW);

    assert_eq!((off.width(), off.height()), (24.0, 24.0), "square knob");
    assert_eq!(
        off.x0,
        ROW.x0 + 2.0,
        "2px from the start"
    );
    assert_eq!(on.x1, ROW.x0 + 52.0 - 2.0, "2px from the far end");
    assert_eq!(off.x1, ROW.x0 + 26.0);
    assert_eq!(on.x0, ROW.x0 + 26.0);
}

/// The knob travels the full gap between the two inset positions, so the travel
/// is *track minus knob minus both insets* — not the track width. A knob that
/// moved by the track width would overlap the track's edge by its own inset.
#[test]
fn the_knob_travels_inside_the_track() {
    let build = |on: bool| {
        Switch::new("x")
            .track_size(52.0, 32.0)
            .knob_size(24.0)
            .knob_inset(2.0)
            .on(on)
            .knob_bounds(ROW)
    };
    let (off, on) = (build(false), build(true));
    assert_eq!(on.x0 - off.x0, 24.0, "52 - 24 - 2 - 2");
    assert!(on.x1 <= ROW.x0 + 52.0, "the knob stays inside the track");
    assert!(off.x0 >= ROW.x0);
}

/// The three overrides are independent: setting one does not quietly re-derive
/// another. Without this, a `knob_size` that fell back to "track height" would
/// still pass a test that only checked the knob's *width*.
#[test]
fn each_override_is_independent_of_the_others() {
    let only_track = Switch::new("x").track_size(52.0, 32.0).knob_bounds(ROW);
    assert_eq!(
        only_track.height(),
        32.0,
        "knob still derives from the new height"
    );

    let sized_knob = Switch::new("x").knob_size(12.0);
    assert_eq!(
        sized_knob.knob_bounds(ROW).width(),
        12.0,
        "the knob takes its own size"
    );
    assert_eq!(
        sized_knob.track_bounds(ROW).height(),
        20.0,
        "while the track height still derives from the width"
    );

    let only_inset = Switch::new("x").knob_inset(4.0).knob_bounds(ROW);
    assert_eq!(only_inset.x0, ROW.x0 + 4.0);
    assert_eq!(
        only_inset.width(),
        20.0,
        "inset moves it, does not resize it"
    );
}

/// An explicit track height survives `layout_request`, which is where a caller
/// learning the widget's size would read it. If only the painting path honoured
/// the override, a parent laying out siblings would still reserve 20px.
#[test]
fn an_explicit_track_height_reaches_the_layout_request() {
    let px = |r: LayoutRequest| match r.height {
        Some(Length::Px(v)) => v,
        other => panic!("expected a pixel height, got {other:?}"),
    };
    let derived = px(Switch::new("Wi-Fi").layout_request().unwrap());
    let explicit = px(Switch::new("Wi-Fi")
        .track_size(52.0, 32.0)
        .layout_request()
        .unwrap());

    assert!(explicit >= 32.0, "must fit a 32-tall track, got {explicit}");
    assert!(
        explicit > derived,
        "the explicit height must change what is reserved ({derived} -> {explicit})"
    );
}

/// A knob wider than its track would hang out of both ends. The knob is capped
/// at the track's height, which is also what stops a kit typo from drawing
/// outside its own widget.
#[test]
fn a_knob_larger_than_the_track_is_capped() {
    let knob = Switch::new("x")
        .track_size(52.0, 32.0)
        .knob_size(999.0)
        .knob_bounds(ROW);
    let track = Switch::new("x").track_size(52.0, 32.0).track_bounds(ROW);
    assert!(
        knob.height() <= track.height(),
        "knob {} vs track {}",
        knob.height(),
        track.height()
    );
}

/// **The control for the palette tests.** A default switch paints exactly the
/// colours it painted before `SwitchPalette` existed. Every default value in
/// `SwitchPalette::default` is load-bearing for that claim, and a "close
/// enough" default would silently restyle every existing caller.
#[test]
fn the_default_palette_is_the_pre_palette_colours() {
    let p = SwitchPalette::default();
    assert_eq!(p.track_on, Color::rgb(0x38, 0xA1, 0x69));
    assert_eq!(p.track_off, Color::rgb(0xCB, 0xD5, 0xE0));
    assert_eq!(p.thumb_on, Color::WHITE);
    assert_eq!(p.track_knob_off, Color::WHITE);
    assert_eq!(p.track_disabled, None, "None means halve alpha, as before");
    assert_eq!(p.thumb_disabled, None);

    // And the widget still reports those colours when asked, unchanged.
    let switch = Switch::new("Wi-Fi");
    assert_eq!(switch.switch_palette(), p);
}

/// A kit-supplied palette reaches the painted pixels, not just the stored
/// struct. A palette that resolved but was ignored by `paint` would pass a
/// field-by-field assertion and still draw the default green.
#[test]
fn a_supplied_palette_reaches_the_pixels() {
    let palette = SwitchPalette {
        track_on: Color::rgb(0x11, 0x22, 0x33),
        track_off: Color::rgb(0x44, 0x55, 0x66),
        thumb_on: Color::rgb(0x77, 0x88, 0x99),
        track_knob_off: Color::rgb(0xAA, 0xBB, 0xCC),
        track_disabled: None,
        thumb_disabled: None,
    };
    let on = Switch::new("x").palette(palette).on(true);
    let id = ElementId::new(1, 1);
    let painted = paint_snapshot(&on, id, ROW);
    assert!(
        painted.contains("#112233"),
        "the supplied track_on must be painted; got {painted}"
    );

    let off = Switch::new("x").palette(palette);
    let painted_off = paint_snapshot(&off, id, ROW);
    assert!(
        painted_off.contains("#445566"),
        "the supplied track_off must be painted; got {painted_off}"
    );
}

/// An explicit disabled colour wins over the derived half-alpha, and it wins
/// over the interaction tints too. Without the second half, a palette that set a
/// disabled colour but left `tints` on would get a hover-shifted disabled track,
/// a state no design system asked for.
#[test]
fn an_explicit_disabled_colour_wins_over_everything() {
    let palette = SwitchPalette {
        track_disabled: Some(Color::rgb(0xDE, 0xAD, 0x00)),
        thumb_disabled: Some(Color::rgb(0xBE, 0xEF, 0x00)),
        ..SwitchPalette::default()
    };
    let mut switch = Switch::new("x").palette(palette).disabled(true);
    let id = ElementId::new(1, 1);
    switch.set_focused(false);
    let painted = paint_snapshot(&switch, id, ROW);
    assert!(
        painted.contains("#DEAD00"),
        "explicit disabled track; got {painted}"
    );
    assert!(
        painted.contains("#BEEF00"),
        "explicit disabled knob; got {painted}"
    );
}

/// `tints(false)` removes hover and press feedback while leaving the base
/// colours alone. The knob is never tinted either way -- it is the affordance,
/// and shifting it would move the thing the pointer is aimed at.
#[test]
fn turning_tints_off_removes_interaction_feedback() {
    let palette = SwitchPalette {
        track_off: Color::rgb(0x44, 0x55, 0x66),
        ..SwitchPalette::default()
    };
    let mut tinted = Switch::new("x").palette(palette);
    let mut flat = Switch::new("x").palette(palette).tints(false);
    let id = ElementId::new(1, 1);

    // The control: at rest the flag changes nothing, so a difference below is
    // about the hover state and not about two unrelated renders.
    assert_eq!(
        paint_snapshot(&tinted, id, ROW),
        paint_snapshot(&flat, id, ROW),
        "at rest the flag must change nothing"
    );

    // Hover only. This is the assertion the flag exists for: with tints on the
    // track shifts, with tints off it must not.
    tinted.state_mut().hovered = true;
    flat.state_mut().hovered = true;
    let hovered_tinted = paint_snapshot(&tinted, id, ROW);
    let hovered_flat = paint_snapshot(&flat, id, ROW);

    assert_ne!(
        hovered_tinted, hovered_flat,
        "hovering must be visible with tints on and invisible with them off"
    );
    assert_eq!(
        hovered_flat,
        paint_snapshot(&Switch::new("x").palette(palette), id, ROW),
        "with tints off, hovering must paint exactly what rest paints"
    );

    // Press too -- press is a separate code path from hover.
    let mut pressed_tinted = Switch::new("x").palette(palette);
    let mut pressed_flat = Switch::new("x").palette(palette).tints(false);
    pressed_tinted.state_mut().pressed = true;
    pressed_flat.state_mut().pressed = true;
    assert_ne!(
        paint_snapshot(&pressed_tinted, id, ROW),
        paint_snapshot(&pressed_flat, id, ROW),
        "pressing must be visible with tints on and invisible with them off"
    );

    assert_eq!(
        flat.switch_palette().track_off,
        palette.track_off,
        "base colours survive"
    );
}

/// The disabled path with no explicit colour is still "halve the alpha", which
/// is what the widget did before a palette existed. Asserted so that adding the
/// field cannot quietly change it to a fixed tint.
#[test]
fn a_disabled_switch_without_a_palette_still_halves_alpha() {
    let base = Color::rgb(0xCB, 0xD5, 0xE0);
    let switch = Switch::new("x").disabled(true);
    let id = ElementId::new(1, 1);
    let painted = paint_snapshot(&switch, id, ROW);
    let halved = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        (base.r as f32 * base.a as f32 / 255.0) as u8,
        (base.g as f32 * base.a as f32 / 255.0) as u8,
        (base.b as f32 * base.a as f32 / 255.0) as u8,
        base.a / 2,
    );
    assert!(
        painted.contains(&halved),
        "expected halved-alpha track {halved} in {painted}"
    );
}

/// Geometry and palette are separate concerns: changing colours must not move
/// anything. Without this, a palette that also nudged the knob would pass the
/// colour tests while silently changing layout.
#[test]
fn a_palette_does_not_move_anything() {
    let plain = Switch::new("x")
        .track_size(52.0, 32.0)
        .knob_size(24.0)
        .knob_inset(2.0);
    let painted = Switch::new("x")
        .track_size(52.0, 32.0)
        .knob_size(24.0)
        .knob_inset(2.0)
        .palette(SwitchPalette {
            track_on: Color::rgb(0x01, 0x02, 0x03),
            track_off: Color::rgb(0x04, 0x05, 0x06),
            thumb_on: Color::rgb(0x07, 0x08, 0x09),
            track_knob_off: Color::rgb(0x0A, 0x0B, 0x0C),
            track_disabled: None,
            thumb_disabled: None,
        });
    let row = Rect::from_origin_size(Point::ZERO, ROW.size());
    assert_eq!(plain.track_bounds(row), painted.track_bounds(row));
    assert_eq!(plain.knob_bounds(row), painted.knob_bounds(row));
}

// --- Progress / Select -----------------------------------------------------

#[test]
fn progress_clamps_and_describes_itself() {
    let mut bar = Progress::determinate(1.5);
    assert_eq!(bar.value(), Some(1.0));
    bar.set_value(Some(-3.0));
    assert_eq!(bar.value(), Some(0.0));

    assert_eq!(
        Progress::determinate(0.42)
            .semantics_hint()
            .label
            .as_deref(),
        Some("42%")
    );
    assert_eq!(
        Progress::indeterminate().semantics_hint().label.as_deref(),
        Some("busy")
    );
}

// --- shared behaviour ------------------------------------------------------

#[test]
fn pointer_down_requests_focus() {
    let mut button = Button::new("Focus me");
    let id = allocate(&button);
    let mut cx = TestEventContext::new(
        id,
        ROW,
        PointerEvent::primary_down(Point::new(1.0, 1.0)).into(),
    );
    assert!(button.event(&mut cx, id));
    assert_eq!(cx.focus_requests(), &[id]);
}

#[test]
fn leave_clears_hover_state() {
    let mut button = Button::new("Hover");
    let id = allocate(&button);
    dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::hover(Point::ZERO).into(),
    );
    assert!(button.interaction().hovered);

    dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent {
            phase: PointerPhase::Leave,
            ..PointerEvent::hover(Point::ZERO)
        }
        .into(),
    );
    assert!(!button.interaction().hovered);
}

#[test]
fn disabled_controls_report_disabled_semantics() {
    let disabled = Button::new("Nope").disabled(true);
    let state = disabled.semantics_hint().state;
    assert!(state.contains(SemanticsState::DISABLED));
    assert!(!state.contains(SemanticsState::ENABLED));
}

#[test]
fn control_paint_snapshots_are_stable() {
    let button = Button::new("Save");
    let id = allocate(&button);
    let snapshot = paint_snapshot(
        &button,
        id,
        Rect::from_origin_size(Point::ZERO, Size::new(80.0, 24.0)),
    );
    assert!(
        snapshot.contains("RoundedRect (0.0, 0.0, 80.0, 24.0) r=4.0 #2B6CB0FF"),
        "background should render at the configured colour, got:\n{snapshot}"
    );
    assert!(
        snapshot.contains("\"Save\""),
        "label is painted: {snapshot}"
    );

    // The checkbox that shared this test moved with the widget to
    // `floom-ui-composites/tests/checkbox.rs`.
}

// --- A3: a gradient in a real control ---------------------------------------

/// A progress bar's gradient is anchored to its *track*, not squeezed into
/// whatever fraction is currently filled — which is what "the brush carries
/// its own coordinates" buys, and the difference a screenshot cannot show you.
///
/// The evidence: paint the same bar at 25 % and at 90 %. The brush that reaches
/// the renderer is byte-identical in both (nothing was rebuilt per value), and
/// the ink at the fill's own centre has moved along the ramp — which it could
/// only do if the ramp stayed put while the fill grew over it. A ramp
/// compressed into the filled width would sample the same midpoint both times.
#[test]
fn a_progress_gradient_stays_anchored_to_the_track_as_the_fill_grows() {
    let ramp = Brush::linear(
        &[
            (0.0, Color::rgb(40, 60, 220)),
            (1.0, Color::rgb(220, 40, 40)),
        ],
        Point::new(0.0, 0.0),
        Point::new(200.0, 0.0),
    );
    let quarter = Progress::determinate(0.25).gradient(ramp.clone());
    let almost = Progress::determinate(0.9).gradient(ramp.clone());

    let painted = |widget: &Progress| {
        let id = allocate(widget);
        let mut recorder = SceneRecorder::new(ROW);
        widget.paint(&mut recorder, id);
        recorder.finish();
        let fills: Vec<_> = recorder
            .scene()
            .iter()
            .filter_map(|(_, node)| match node {
                SceneNode::Rect { fill, .. } => Some(fill.clone()),
                _ => None,
            })
            .collect();
        (recorder.snapshot(), fills)
    };

    let (quarter_ops, quarter_fills) = painted(&quarter);
    let (almost_ops, almost_fills) = painted(&almost);

    // Two rects each: the dim track, then the gradient fill.
    assert_eq!(quarter_fills.len(), 2);
    assert_eq!(
        quarter_fills[1], ramp,
        "the whole ramp reaches the renderer at 25 %"
    );
    assert_eq!(
        almost_fills[1], ramp,
        "and the identical ramp at 90 % — nothing was respecified"
    );
    let track_line = "Rect (0.0, 0.0, 200.0, 32.0) #E2E8F0FF";
    let quarter_fill_line = quarter_ops
        .lines()
        .nth(1)
        .expect("the fill is the second op")
        .to_string();
    let almost_fill_line = almost_ops
        .lines()
        .nth(1)
        .expect("the fill is the second op")
        .to_string();
    assert_eq!(quarter_ops.lines().next().unwrap(), track_line);
    assert_eq!(almost_ops.lines().next().unwrap(), track_line);
    assert!(
        quarter_fill_line.starts_with("BrushRect (0.0, 0.0, 50.0, 32.0)"),
        "a quarter of the 200px track: {quarter_fill_line}"
    );
    assert!(
        almost_fill_line.starts_with("BrushRect (0.0, 0.0, 180.0, 32.0)"),
        "ninety percent of the track: {almost_fill_line}"
    );
    assert_ne!(
        quarter_fill_line, almost_fill_line,
        "the fill's own centre landed further along the ramp"
    );
}

/// Without a gradient the bar is unchanged: one flat fill at the colour it has
/// always used. The new door must not repaint yesterday's bar.
#[test]
fn a_gradientless_progress_paints_the_same_flat_fill_as_before() {
    let id = allocate(&Progress::determinate(0.5));
    assert_eq!(
        paint_snapshot(&Progress::determinate(0.5), id, ROW),
        "Rect (0.0, 0.0, 200.0, 32.0) #E2E8F0FF\nRect (0.0, 0.0, 100.0, 32.0) #2B6CB0FF"
    );
}

// --- Pressable -------------------------------------------------------------

fn pressable(child: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Pressable {
    Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), child)).label("header")
}

/// A click is a press *and* a release over the same spot. Sending one half is
/// not a weaker click, it is not a click.
#[test]
fn a_pressable_fires_on_a_matched_press_and_release() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(move || Box::new(Text::new("row"))).on_click(move || {
        sink.set(sink.get() + 1);
    });
    let id = allocate(&widget);

    assert!(fire_down_up(&mut widget, id, Point::new(10.0, 10.0)));

    assert_eq!(hits.get(), 1, "the callback runs exactly once");
}

/// The whole point of the widget: `Button` paints its own label, so a row of
/// widgets has no other way to be clickable.
#[test]
fn a_pressable_wraps_a_subtree_rather_than_a_label() {
    let row = Stack::new(Axis::Horizontal)
        .push(|| Box::new(Text::new("Title")))
        .push(|| Box::new(Text::new(">")));
    let widget = pressable(move || Box::new(row.clone()));

    let children = widget.children();
    assert_eq!(children.len(), 1);
    let built = children[0].build();
    assert!(
        built.as_any().downcast_ref::<Stack>().is_some(),
        "the pressable's child is the caller's subtree, untouched"
    );
}

/// The whole surface consumes, so a press inside it never reaches something
/// further down. (`Ripple` is the opposite case and says so.)
#[test]
fn a_pressable_consumes_the_press() {
    let mut widget = pressable(|| Box::new(Text::new("row")));
    let id = allocate(&widget);
    assert!(
        dispatch(
            &mut widget,
            id,
            ROW,
            PointerEvent::primary_down(Point::new(4.0, 4.0)).into()
        ),
        "the press belongs to the pressable"
    );
}

/// A release with no press in flight is not a click. Without this a pointer
/// that appears already up — a synthetic event, a missed down — would fire the
/// action.
#[test]
fn a_release_without_a_press_does_not_fire() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(|| Box::new(Text::new("row"))).on_click(move || {
        sink.set(sink.get() + 1);
    });
    let id = allocate(&widget);

    dispatch(
        &mut widget,
        id,
        ROW,
        PointerEvent::primary_up(Point::new(4.0, 4.0)).into(),
    );

    assert_eq!(hits.get(), 0, "a release alone is not a click");
}

#[test]
fn a_disabled_pressable_is_inert() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(|| Box::new(Text::new("row")))
        .disabled(true)
        .on_click(move || {
            sink.set(sink.get() + 1);
        });
    let id = allocate(&widget);

    assert!(!fire_down_up(&mut widget, id, Point::new(4.0, 4.0)));

    assert_eq!(hits.get(), 0);
}

#[test]
fn a_focused_pressable_activates_from_the_keyboard() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(|| Box::new(Text::new("row"))).on_click(move || {
        sink.set(sink.get() + 1);
    });
    let id = allocate(&widget);
    widget.set_focused(true);

    let event = KeyEvent {
        key: Key::Named(NamedKey::Enter),
        phase: KeyPhase::Down,
        modifiers: Modifiers::empty(),
    };
    assert!(dispatch(&mut widget, id, ROW, event.into()));

    assert_eq!(
        hits.get(),
        1,
        "a disclosure has to be operable without a pointer"
    );
}

/// The reported state follows a *reader*, not a value captured at build time.
/// A header that is not rebuilt across its own toggle would otherwise report
/// the state it was built with — the opposite of what the user sees.
#[test]
fn the_reported_expanded_state_follows_a_shared_flag() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let open = Arc::new(AtomicBool::new(false));
    let reader = open.clone();
    let mut widget = pressable(|| Box::new(Text::new("row")))
        .expanded_from(move || reader.load(Ordering::SeqCst));
    let id = allocate(&widget);

    let mut cx = RecordingUpdateContext::new();
    widget.update(&mut cx, id);
    assert!(
        widget
            .semantics_hint()
            .state
            .contains(SemanticsState::COLLAPSED),
        "closed panels report collapsed"
    );

    open.store(true, Ordering::SeqCst);
    let mut cx = RecordingUpdateContext::new();
    widget.update(&mut cx, id);
    assert!(
        widget
            .semantics_hint()
            .state
            .contains(SemanticsState::EXPANDED),
        "a flag that moved since the last update has to invalidate the node, \
         or the semantics tree keeps reporting last frame's answer"
    );
    assert_eq!(cx.dirty_count(), 1);
}

/// The settled frame marks nothing: a widget that dirties itself every frame
/// would invalidate its whole subtree for a tree standing still.
#[test]
fn a_settled_pressable_marks_nothing_dirty() {
    let mut widget = pressable(|| Box::new(Text::new("row")));
    let id = allocate(&widget);

    let mut first = RecordingUpdateContext::new();
    widget.update(&mut first, id);
    let mut second = RecordingUpdateContext::new();
    widget.update(&mut second, id);

    assert_eq!(first.dirty_count(), 1, "the first sync paints once");
    assert_eq!(
        second.dirty_count(),
        0,
        "an unchanged widget must not invalidate again"
    );
}

#[test]
fn a_pressable_reports_its_role_and_name() {
    let widget = pressable(|| Box::new(Text::new("row")));
    let hint = widget.semantics_hint();
    assert_eq!(hint.role, floom_accessibility::SemanticsRole::Button);
    assert_eq!(hint.label.as_deref(), Some("header"));
    assert!(
        hint.state.contains(SemanticsState::FOCUSABLE),
        "an activatable surface must be reachable by keyboard"
    );
}

#[test]
fn a_pressable_paints_nothing() {
    let widget = pressable(|| Box::new(Text::new("row")));
    let id = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, ROW), "(empty)");
}
