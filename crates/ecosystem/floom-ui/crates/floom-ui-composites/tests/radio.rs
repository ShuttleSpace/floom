//! `Radio`'s own logic, and the claim that its default style is the pixels the
//! engine painted before the move.
//!
//! These moved with the widget from `floom_widgets/tests/controls.rs`, for the reason
//! `slider.rs` moved: a test of a widget's behaviour belongs with the widget, and
//! this file is what has to be deleted with it.
//!
//! What is new here is the half the move made possible. The old file could not
//! say "the default style is the old palette", because there was no style -- it
//! could only assert behaviour. With `RadioStyle` in existence every value in its
//! table is a promise about pixels, and the tests below are what hold it to that.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{ElementId, Key, NamedKey, Point, Rect, Widget};
use floom_types::{Color, KeyEvent, PointerEvent};
use floom_ui_composites::{Radio, RadioHalo, RadioStyle};
use floom_widgets::testkit::{AllocatingCreateContext, dispatch, paint_snapshot};

/// The box the control is laid out in for every test here: wide enough for the
/// ring and its label, tall enough for the ring to be centred in it.
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

/// Puts a radio into `hovered` by dispatching the pointer event the platform
/// would, rather than by writing to its state -- so the assertion is about the
/// wiring and not about a field the test set itself.
fn hover(radio: &mut Radio, id: ElementId) {
    dispatch(
        radio,
        id,
        ROW,
        PointerEvent::hover(Point::new(8.0, 16.0)).into(),
    );
}

/// The same for a press, which also hovers: `PointerPhase::Down` sets both.
fn press(radio: &mut Radio, id: ElementId) {
    dispatch(
        radio,
        id,
        ROW,
        PointerEvent::primary_down(Point::new(8.0, 16.0)).into(),
    );
}

// --- the widget ------------------------------------------------------------

#[test]
fn radio_selects_once_and_stays_selected() {
    let selections = Rc::new(Cell::new(0));
    let counter = selections.clone();
    let mut radio = Radio::new("Option A").on_select(move || counter.set(counter.get() + 1));
    let id = allocate(&radio);

    fire_down_up(&mut radio, id, Point::new(4.0, 4.0));
    fire_down_up(&mut radio, id, Point::new(4.0, 4.0));
    assert!(radio.is_selected());
    assert_eq!(selections.get(), 1, "re-selecting must not re-notify");
}

#[test]
fn the_keyboard_selects_a_focused_radio_and_nothing_else() {
    let mut radio = Radio::new("Option A");
    let id = allocate(&radio);

    // Unfocused: the radio belongs to nobody, and must not select itself.
    assert!(!dispatch(
        &mut radio,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into()
    ));
    assert!(!radio.is_selected());

    radio.set_focused(true);
    assert!(dispatch(
        &mut radio,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into()
    ));
    assert!(radio.is_selected(), "Enter activates the focused radio");

    // A key release is not a second activation.
    let mut other = Radio::new("Option B");
    let other_id = allocate(&other);
    other.set_focused(true);
    assert!(!dispatch(
        &mut other,
        other_id,
        ROW,
        KeyEvent::up(Key::Named(NamedKey::Enter)).into()
    ));
    assert!(!other.is_selected());

    // And a disabled radio answers nothing at all.
    let mut off = Radio::new("Option C").disabled(true);
    let off_id = allocate(&off);
    off.set_focused(true);
    assert!(!dispatch(
        &mut off,
        off_id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into()
    ));
    assert!(!off.is_selected());
}

// --- the default style is the engine's old paint ----------------------------

/// **The control for every default value below.** Each is a literal the old
/// `paint` wrote inline, so a default that drifted by one hex digit or one point
/// would repaint every existing caller without failing anything else here.
#[test]
fn the_default_style_is_the_pre_style_pixels() {
    let style = RadioStyle::default();

    assert_eq!(style.size, 16.0, "the old `min(bounds.height(), 16.0)`");
    assert_eq!(
        style.touch_target, 16.0,
        "the old ring had no larger hit area"
    );
    assert_eq!(
        style.border_width,
        style.size / 2.0,
        "which is what makes the ring the old filled disc"
    );
    assert_eq!(
        style.interior_color,
        Some(Color::WHITE),
        "the old unselected disc was painted white"
    );
    assert_eq!(style.selected_color, Color::rgb(0x9A, 0xA5, 0xB1));
    assert_eq!(
        style.unselected_color,
        Color::WHITE,
        "a white disc's rim is white"
    );
    assert_eq!(style.selected_dot_color, Color::rgb(0x2B, 0x6C, 0xB0));
    // `resolved_color` in the widgets crate is `(base, pressed, hovered)`, and
    // the old inline call passed `#E2E8F0` second -- so hover is the *lighter*
    // of the pair, and these two are in that order.
    assert_eq!(
        style.unselected_hovered_color,
        Some(Color::rgb(0xF5, 0xF7, 0xFA)),
        "the old hover"
    );
    assert_eq!(
        style.unselected_pressed_color,
        Some(Color::rgb(0xE2, 0xE8, 0xF0)),
        "the old press"
    );
    assert_eq!(
        style.disabled_color, None,
        "None means halve the alpha, which is what the old paint did"
    );
    assert_eq!(style.label_font_size, 14.0);
    assert_eq!(style.label_color, Color::BLACK);
    assert_eq!(style.halo, RadioHalo::default(), "the engine drew no halo");
    assert_eq!(
        style.halo.size, 0.0,
        "and a default halo has no diameter, rather than a leftover constant"
    );
}

/// The same claim one layer down: an unstyled radio still *paints* those
/// colours. A default table nothing read would pass the test above and leave the
/// widget green.
#[test]
fn an_unstyled_radio_paints_the_engine_colours() {
    let id = allocate(&Radio::new("x"));
    let unselected = paint_snapshot(&Radio::new("x"), id, ROW);
    assert!(
        unselected.contains("#FFFFFF"),
        "the unselected disc was white; got {unselected}"
    );

    let selected = paint_snapshot(&Radio::new("x").selected(true), id, ROW);
    assert!(
        selected.contains("#9AA5B1"),
        "the selected disc was #9AA5B1; got {selected}"
    );
    assert!(
        selected.contains("#2B6CB0"),
        "with a #2B6CB0 dot inside it; got {selected}"
    );
}

/// The dot is half the ring. All three kits wrote `size * 0.5` and so did the
/// engine's 25% inset, which is why `RadioStyle` derives it rather than carrying
/// a field a kit could set to disagree with itself.
#[test]
fn the_ring_is_the_style_size_and_the_dot_is_half_of_it() {
    let ring = Radio::new("x").dot_bounds(ROW);
    assert_eq!(ring.width(), 16.0);
    assert_eq!(ring.height(), 16.0, "the ring is a circle");
    assert_eq!(ring.x0, ROW.x0);
    assert_eq!(
        ring.y0,
        ROW.y0 + (ROW.height() - 16.0) / 2.0,
        "centred vertically, as before"
    );

    // A kit's own size is honoured rather than capped at the old constant: the
    // old `min(bounds.height(), 16.0)` could never return anything but 16.
    let big = Radio::new("x")
        .style(RadioStyle {
            size: 20.0,
            ..RadioStyle::default()
        })
        .dot_bounds(ROW);
    assert_eq!(big.width(), 20.0, "a 20px ring is 20px wide");
    assert_eq!(big.height(), 20.0);
}

// --- the interaction states are load-bearing -------------------------------

/// The default style still shifts under the pointer, which is the appearance the
/// engine had and the thing a "close enough" default would silently drop.
///
/// The last paragraph is the point: with `unselected_hovered_color` and
/// `unselected_pressed_color` set to `None`, an idle radio, a hovered one and a
/// pressed one all paint the *same* string and the two `assert_ne!`s above are
/// what fail. That makes them a claim about the resolution rather than a
/// restatement of the table.
#[test]
fn hovering_and_pressing_a_radio_repaint_it() {
    let id = allocate(&Radio::new("x"));
    let idle = paint_snapshot(&Radio::new("x"), id, ROW);

    let mut hovered = Radio::new("x");
    let hovered_id = allocate(&hovered);
    hover(&mut hovered, hovered_id);
    let hover_paint = paint_snapshot(&hovered, hovered_id, ROW);
    assert!(
        hover_paint.contains("#F5F7FA"),
        "the old hover colour; got {hover_paint}"
    );
    assert_ne!(hover_paint, idle, "hovering must not paint the idle radio");

    let mut pressed = Radio::new("x");
    let pressed_id = allocate(&pressed);
    press(&mut pressed, pressed_id);
    let press_paint = paint_snapshot(&pressed, pressed_id, ROW);
    assert!(
        press_paint.contains("#E2E8F0"),
        "the old press colour; got {press_paint}"
    );
    assert_ne!(press_paint, hover_paint, "and a press differs again");
    assert_ne!(press_paint, idle);

    // The load-bearing half: with the two variants removed there is nothing left
    // for the state to change.
    let flat = RadioStyle {
        unselected_hovered_color: None,
        unselected_pressed_color: None,
        ..RadioStyle::default()
    };
    let mut plain = Radio::new("x").style(flat);
    let plain_id = allocate(&mut plain);
    hover(&mut plain, plain_id);
    assert_eq!(
        paint_snapshot(&plain, plain_id, ROW),
        idle,
        "a style with no interaction variants paints one radio for every state"
    );
}

/// The variants replace *every* layer of the unselected ring, which is what
/// makes one pair of fields enough: a style with no rim has no rim to change, so
/// the interior changes instead.
#[test]
fn the_variants_reach_the_interior_when_there_is_no_rim() {
    let no_rim = RadioStyle {
        border_width: 0.0,
        interior_color: Some(Color::WHITE),
        ..RadioStyle::default()
    };
    let id = allocate(&Radio::new("x"));
    let idle = paint_snapshot(&Radio::new("x").style(no_rim), id, ROW);
    assert!(
        !idle.contains("#F5F7FA"),
        "nothing is hovered yet; got {idle}"
    );

    let mut hovered = Radio::new("x").style(no_rim);
    let hovered_id = allocate(&hovered);
    hover(&mut hovered, hovered_id);
    let hover_paint = paint_snapshot(&hovered, hovered_id, ROW);
    assert!(
        hover_paint.contains("#F5F7FA"),
        "the interior is the only layer left to change; got {hover_paint}"
    );
}

/// A `border_width` of `size / 2.0` fills the ring, so there is no interior left
/// to see through -- which is what the engine's disc was, and why the default can
/// carry a white interior without the default radio looking like anything but
/// white.
#[test]
fn a_full_width_rim_leaves_no_interior_to_see_through() {
    let id = allocate(&Radio::new("x"));
    // Interior white under a white rim: identical strings, so the disc reads as
    // one colour rather than two stacked ones.
    let painted = paint_snapshot(&Radio::new("x"), id, ROW);
    assert_eq!(
        painted.matches("#FFFFFF").count(),
        2,
        "the interior and the rim both paint white; got {painted}"
    );

    // A kit that leaves its interior unpainted paints one layer fewer, because
    // there is nothing underneath for it to cover.
    let see_through = RadioStyle {
        border_width: 2.0,
        interior_color: None,
        ..RadioStyle::default()
    };
    let ring = paint_snapshot(&Radio::new("x").style(see_through), id, ROW);
    assert!(
        !ring.contains("RoundedRect"),
        "an unpainted interior is the absence of a layer, not a white one; got {ring}"
    );
    assert!(ring.contains("stroke=2.0"), "a 2px rim; got {ring}");
    // And the default's interior really is a layer, which is what the assertion
    // above is contrasting with: two ops for one colour, not one.
    assert!(
        paint_snapshot(&Radio::new("x"), id, ROW).contains("RoundedRect"),
        "the default paints an interior under its rim"
    );
}

// --- the disabled rule -----------------------------------------------------

/// With `disabled_color: None` the engine's half-alpha dim applies, and it
/// applies to the *base* colour however the pointer is behaving -- which is the
/// order `floom_widgets::resolved_color` ranked disabled in, and the reason it
/// was exported rather than re-derived here.
#[test]
fn a_disabled_unselected_radio_is_halved_in_alpha() {
    let id = allocate(&Radio::new("x"));
    let dimmed = paint_snapshot(&Radio::new("x").disabled(true), id, ROW);
    // `255 / 2` is 127, not 128: the tint floors rather than rounds, and the
    // old paint went through the same function, so this is the old byte.
    assert!(
        dimmed.contains("#FFFFFF7F"),
        "white at half alpha is what the old paint resolved; got {dimmed}"
    );

    // Disabled outranks the interaction variants, so a disabled radio the pointer
    // is over still dims its base rather than its hover.
    let mut under_the_pointer = Radio::new("x").disabled(true);
    let hovered_id = allocate(&mut under_the_pointer);
    hover(&mut under_the_pointer, hovered_id);
    let hovered_dim = paint_snapshot(&under_the_pointer, hovered_id, ROW);
    assert!(
        !hovered_dim.contains("#F5F7FA"),
        "the hover variant must not beat the dim; got {hovered_dim}"
    );
}

/// A disabled *selected* radio keeps full strength: the old paint branched on
/// `selected` before it consulted the state at all. This is the asymmetry the
/// `disabled_color` doc comment is about.
#[test]
fn a_disabled_selected_radio_stays_at_full_strength() {
    let id = allocate(&Radio::new("x"));
    let dimmed = paint_snapshot(&Radio::new("x").selected(true).disabled(true), id, ROW);
    assert!(
        dimmed.contains("#9AA5B1FF"),
        "a full-alpha rim; got {dimmed}"
    );
    assert!(
        dimmed.contains("#2B6CB0FF"),
        "and a full-alpha dot; got {dimmed}"
    );
}

/// A named `disabled_color` overrides the dim for every layer at once -- rim,
/// dot and label together, which is what all three kits dim to one colour.
#[test]
fn a_named_disabled_colour_covers_the_rim_dot_and_label() {
    let style = RadioStyle {
        disabled_color: Some(Color::rgb(0x61, 0x2A, 0x2A)),
        ..RadioStyle::default()
    };
    let id = allocate(&Radio::new("x"));
    let painted = paint_snapshot(
        &Radio::new("x").selected(true).disabled(true).style(style),
        id,
        ROW,
    );
    assert!(
        !painted.contains("#9AA5B1") && !painted.contains("#2B6CB0"),
        "neither the old rim nor the old dot survives; got {painted}"
    );
    assert!(
        painted.matches("#612A2A").count() >= 3,
        "all three layers take the one colour; got {painted}"
    );
}

/// The default leaves `disabled_color` as `None` because the engine never tinted
/// its label at all, so dimming it would be a repaint rather than a move.
#[test]
fn a_disabled_radio_keeps_its_label_undimmed_by_default() {
    let id = allocate(&Radio::new("x"));
    let painted = paint_snapshot(&Radio::new("x").disabled(true), id, ROW);
    assert!(
        painted.contains("#000000FF"),
        "the label stays black; got {painted}"
    );
}

// --- the halo --------------------------------------------------------------

/// The gates are per state, because the kits disagree about *which* states draw
/// a halo. One "enabled" boolean would have to be Material's answer, and would
/// then put Cupertino's halo in the wrong moments.
#[test]
fn a_halo_draws_only_for_the_states_its_gates_open() {
    let style = RadioStyle {
        halo: RadioHalo {
            color: Color::rgb(0x1A, 0x73, 0xE8),
            color_pressed: Some(Color::rgb(0x0B, 0x57, 0xA0)),
            size: 40.0,
            on_hover: true,
            on_press: true,
            on_focus: false,
            suppressed_when_disabled: true,
        },
        ..RadioStyle::default()
    };
    let id = allocate(&Radio::new("x"));
    let idle = paint_snapshot(&Radio::new("x").style(style), id, ROW);
    assert!(!idle.contains("#1A73E8"), "no gate is open yet; got {idle}");

    let mut hovered = Radio::new("x").style(style);
    let hovered_id = allocate(&hovered);
    hover(&mut hovered, hovered_id);
    assert!(
        paint_snapshot(&hovered, hovered_id, ROW).contains("#1A73E8"),
        "hover opens its gate"
    );

    let mut pressed = Radio::new("x").style(style);
    let pressed_id = allocate(&pressed);
    press(&mut pressed, pressed_id);
    let press_paint = paint_snapshot(&pressed, pressed_id, ROW);
    assert!(
        press_paint.contains("#0B57A0"),
        "and the press swaps in `color_pressed`; got {press_paint}"
    );

    // Focus alone draws nothing: this halo's `on_focus` is false.
    let mut focused = Radio::new("x").style(style);
    let focused_id = allocate(&mut focused);
    focused.set_focused(true);
    assert!(
        !paint_snapshot(&focused, focused_id, ROW).contains("#1A73E8"),
        "a closed gate stays closed"
    );

    // And `suppressed_when_disabled` takes the halo away even while hovering --
    // Cupertino's and Fluent's rule, which Material's does not follow.
    let mut disabled = Radio::new("x").style(style).disabled(true);
    let disabled_id = allocate(&mut disabled);
    hover(&mut disabled, disabled_id);
    assert!(
        !paint_snapshot(&disabled, disabled_id, ROW).contains("#1A73E8"),
        "a suppressed halo is not drawn while disabled"
    );
}

/// The halo sits *behind* the ring, so it has to be painted first. Painted last
/// it would cover the very control it belongs to.
#[test]
fn the_halo_is_painted_before_the_ring() {
    let style = RadioStyle {
        unselected_hovered_color: None,
        halo: RadioHalo {
            color: Color::rgb(0x1A, 0x73, 0xE8),
            size: 40.0,
            on_hover: true,
            ..RadioHalo::default()
        },
        ..RadioStyle::default()
    };
    let mut hovered = Radio::new("x").style(style);
    let id = allocate(&mut hovered);
    hover(&mut hovered, id);
    let painted = paint_snapshot(&hovered, id, ROW);
    let halo_at = painted.find("#1A73E8").expect("the halo was painted");
    let ring_at = painted.find("#FFFFFF").expect("the ring was painted");
    assert!(halo_at < ring_at, "the halo comes first; got {painted}");
}
