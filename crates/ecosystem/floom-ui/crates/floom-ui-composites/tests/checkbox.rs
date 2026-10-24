//! `Checkbox`'s own logic, and the claim that its default style is the pixels the
//! engine painted before the move.
//!
//! These moved with the widget from `floom_widgets/tests/controls.rs`, for the
//! reason `radio.rs` moved: a test of a widget's behaviour belongs with the
//! widget, and this file is what has to be deleted with it.
//!
//! What is new here is the half the move made possible. The old file could not
//! say "the default style is the old palette", because there was no style -- it
//! could only assert behaviour. With `CheckboxStyle` in existence every value in
//! its table is a promise about pixels, and the tests below are what hold it to
//! that.
//!
//! The second new half is the mark. All three v1 kits drew a vector checkmark and
//! the engine drew the character `U+2714`, so there are now two paths to prove:
//! [`the_default_mark_is_the_engine_character_glyph`] and
//! [`a_supplied_mark_is_painted_as_a_vector_icon`].

use floom_accessibility::SemanticsState;
use floom_core::{ElementId, Point, Rect, Widget};
use floom_types::{Color, Rect as TypesRect};
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{Checkbox, CheckboxStyle, FocusRing};
use floom_widgets::testkit::{dispatch, paint_snapshot, AllocatingCreateContext};
use floom_widgets::VectorIcon;

/// The box the control is laid out in for every test here: wide enough for the
/// indicator and its label, tall enough for the indicator to be centred in it.
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

/// Puts a checkbox into `hovered` by dispatching the pointer event the platform
/// would, rather than by writing to its state -- so the assertion is about the
/// wiring and not about a field the test set itself.
fn hover(checkbox: &mut Checkbox, id: ElementId) {
    dispatch(
        checkbox,
        id,
        ROW,
        floom_types::PointerEvent::hover(Point::new(8.0, 16.0)).into(),
    );
}

/// A checkmark with a path, standing in for what a kit's icon crate hands out.
///
/// A three-point tick on a 16-unit view box, which is the shape the three v1
/// `*_icon_svg("check")` calls produce. Its two extremes sit on the view box's
/// own edges, so the path bounds the recorder prints are the box the control
/// handed `paint_icon` -- which is what makes `mark_scale` assertable rather
/// than merely visible.
fn tick() -> VectorIcon {
    VectorIcon::from_path_data(
        "M 0 9 L 7 16 L 16 0",
        TypesRect {
            x0: 0.0,
            y0: 0.0,
            x1: 16.0,
            y1: 16.0,
        },
    )
}

// --- the widget ------------------------------------------------------------

#[test]
fn checkbox_toggles_and_publishes_checked_state() {
    let mut box_widget = Checkbox::new("Accept terms");
    let id = allocate(&box_widget);
    assert!(!box_widget.is_checked());

    fire_down_up(&mut box_widget, id, Point::new(4.0, 4.0));
    assert!(box_widget.is_checked());
    assert!(box_widget
        .semantics_hint()
        .state
        .contains(SemanticsState::CHECKED));

    fire_down_up(&mut box_widget, id, Point::new(4.0, 4.0));
    assert!(!box_widget.is_checked());
}

/// Helper unrolling `dyn Widget` downcasts for assertions after a dispatch.
fn fire_down_up<W: Widget>(widget: &mut W, id: ElementId, at: Point) -> bool {
    let down = dispatch(
        widget,
        id,
        ROW,
        floom_types::PointerEvent::primary_down(at).into(),
    );
    let up = dispatch(
        widget,
        id,
        ROW,
        floom_types::PointerEvent::primary_up(at).into(),
    );
    down || up
}

// --- the default style is the engine's old paint ----------------------------

/// **The control for every default value below.** Each is a literal the old
/// `paint` wrote inline, so a default that drifted by one hex digit or one point
/// would repaint every existing caller without failing anything else here.
#[test]
fn the_default_style_is_the_pre_style_pixels() {
    let style = CheckboxStyle::default();

    assert_eq!(style.size, 16.0, "the old `INDICATOR_SIDE`");
    assert_eq!(style.border_radius, 0.0, "the engine filled a square");
    assert_eq!(
        style.border_width, 0.0,
        "no rim: the engine filled the box and stopped"
    );
    assert_eq!(style.label_gap, 6.0, "the old `INDICATOR_LABEL_GAP`");
    assert_eq!(style.label_font_size, 14.0, "the old `LABEL_FONT_SIZE`");

    assert_eq!(style.container_color, Color::rgb(0x2B, 0x6C, 0xB0));
    assert_eq!(
        style.container_color_unchecked,
        Color::WHITE,
        "the unchecked fill was white, not transparent"
    );
    assert_eq!(
        style.container_color_disabled,
        Color::WHITE,
        "the engine did not dim it"
    );
    assert_eq!(style.border_color, Color::rgb(0x2B, 0x6C, 0xB0));
    assert_eq!(style.border_color_unchecked, Color::WHITE);
    assert_eq!(style.border_color_disabled, Color::WHITE);

    assert_eq!(
        style.mark_color,
        Color::WHITE,
        "the U+2714 was painted white"
    );
    assert_eq!(
        style.mark_color_unchecked,
        Color::TRANSPARENT,
        "and was never drawn unchecked"
    );
    assert_eq!(style.mark_color_disabled, Color::WHITE);
    assert_eq!(
        style.mark_scale, 1.0,
        "the glyph was drawn at the full box height"
    );

    assert_eq!(style.label_color, Color::BLACK);
    assert_eq!(style.label_color_disabled, Color::BLACK);
    assert_eq!(style.layer, StateLayer::default(), "no layer existed");
    assert_eq!(style.focus_ring, None, "no focus affordance existed");
}

/// **The load-bearing half of the table above.** A style nothing reads would pass
/// every assertion in it and leave the widget green; this says the colours and
/// the geometry reach the recorder.
#[test]
fn an_unstyled_checkbox_paints_the_engine_colours() {
    let id = allocate(&Checkbox::new("x"));

    let unchecked = paint_snapshot(&Checkbox::new("x"), id, ROW);
    assert!(
        unchecked.contains("#FFFFFF"),
        "the unchecked fill was white; got {unchecked}"
    );
    assert!(
        unchecked.contains("RoundedRect (0.0, 8.0, 16.0, 24.0) r=0.0 #FFFFFFFF"),
        "a 16px square, vertically centred in the row; got {unchecked}"
    );

    let checked = paint_snapshot(&Checkbox::new("x").checked(true), id, ROW);
    assert!(
        checked.contains("#2B6CB0FF"),
        "the checked fill was the hard-coded accent; got {checked}"
    );
    assert!(
        checked.contains("\"x\""),
        "the label follows the box; got {checked}"
    );
}

/// The geometry is the style's own number and is not capped at the old constant.
/// The old `box_bounds` took `min(bounds.height(), 16.0)`, which could only ever
/// return 16 -- a hard-coded constant wearing a cap's clothes -- so a kit whose
/// `CheckboxTokens::size` is 20 or 24 had no way to be drawn at all.
#[test]
fn the_indicator_is_the_style_size_and_the_label_starts_after_the_gap() {
    let box_rect = Checkbox::new("x").box_bounds(ROW);
    assert_eq!((box_rect.width(), box_rect.height()), (16.0, 16.0));
    assert_eq!(box_rect.x0, ROW.x0);
    assert_eq!(box_rect.y0, ROW.y0 + (ROW.height() - 16.0) / 2.0);

    let big = Checkbox::new("x")
        .style(CheckboxStyle {
            size: 24.0,
            ..CheckboxStyle::default()
        })
        .box_bounds(ROW);
    assert_eq!(big.width(), 24.0, "a 24px indicator is 24px wide");
    assert_eq!(big.height(), 24.0);
}

// --- the two mark paths ----------------------------------------------------

/// **The default mark is the engine's glyph, byte for byte.** The old `paint`
/// wrote `U+2714` with `fill_text` at the box's own height, from a hard-coded
/// two-and-one nudge off the corner, in white. Every one of those numbers is
/// here, and a glyph that turned into a path -- or into an icon -- would fail
/// this rather than pass it.
#[test]
fn the_default_mark_is_the_engine_character_glyph() {
    let id = allocate(&Checkbox::new("x"));
    let checked = paint_snapshot(&Checkbox::new("x").checked(true), id, ROW);
    assert!(
        checked.contains("Text (2.0, 9.0) 16.0 \"\u{2714}\" #FFFFFFFF"),
        "the engine's own glyph, at the box's height, in white; got {checked}"
    );
    assert!(
        !checked.contains("Path"),
        "and no vector path: the default has no icon; got {checked}"
    );

    // Unchecked paints no mark at all -- not a transparent one. The old paint
    // drew the glyph only while checked.
    let unchecked = paint_snapshot(&Checkbox::new("x"), id, ROW);
    assert_eq!(
        unchecked.matches("Text").count(),
        1,
        "the label is the only text an unchecked checkbox paints; got {unchecked}"
    );
}

/// **The other mark path.** All three v1 kits drew `*_icon_svg("check")` at
/// `size * 0.7`, so the icon has to reach the recorder as a path rather than
/// being silently dropped -- and it has to be *sized* by the style rather than
/// filling the indicator.
#[test]
fn a_supplied_mark_is_painted_as_a_vector_icon() {
    let id = allocate(&Checkbox::new("x"));
    let glyph = paint_snapshot(&Checkbox::new("x").checked(true), id, ROW);

    let icon = paint_snapshot(&Checkbox::new("x").checked(true).mark(tick()), id, ROW);
    assert!(
        icon.contains("Path "),
        "a vector mark reaches the recorder as a path; got {icon}"
    );
    assert!(
        icon.contains("#FFFFFFFF"),
        "in the style's mark colour; got {icon}"
    );
    assert!(
        !icon.contains("\"\u{2714}\""),
        "and the character glyph is not drawn as well; got {icon}"
    );
    assert_ne!(
        icon, glyph,
        "the two mark paths are genuinely different paint"
    );

    // `mark_scale` is what a kit's `size * 0.7` is written as, so it has to
    // reach the geometry: 0.7 of a 16px indicator is an 11.2px path, centred on
    // the indicator's centre rather than on the control's box.
    let scaled = paint_snapshot(
        &Checkbox::new("x")
            .checked(true)
            .mark(tick())
            .style(CheckboxStyle {
                mark_scale: 0.7,
                ..CheckboxStyle::default()
            }),
        id,
        ROW,
    );
    assert!(
        scaled.contains("Path (2.4, 10.4, 13.6, 21.6)"),
        "size * 0.7 -- an 11.2px box centred on the indicator's centre (8, 16) -- \
         not the full 16px one; got {scaled}"
    );

    // An unchecked checkbox draws no mark whichever path it was given.
    let unchecked_icon = paint_snapshot(&Checkbox::new("x").mark(tick()), id, ROW);
    assert!(
        !unchecked_icon.contains("Path"),
        "an unchecked box has no mark; got {unchecked_icon}"
    );
}

// --- the states the three kits disagree about ------------------------------

/// The rim, the interior and the label are each resolved per state rather than
/// carried as one colour. All three kits write `primary`/`outline` for the pair
/// and `on_surface`@38 across the board when disabled, and the default names
/// white where the engine named white, so this is the table being read rather
/// than a design being applied.
#[test]
fn the_three_states_read_three_columns_of_the_table() {
    let style = CheckboxStyle {
        border_width: 2.0,
        container_color: Color::rgb(0x01, 0x02, 0x03),
        container_color_unchecked: Color::rgb(0x0A, 0x0B, 0x0C),
        container_color_disabled: Color::rgb(0x10, 0x11, 0x12),
        border_color: Color::rgb(0x04, 0x05, 0x06),
        border_color_unchecked: Color::rgb(0x0D, 0x0E, 0x0F),
        border_color_disabled: Color::rgb(0x13, 0x14, 0x15),
        mark_color: Color::rgb(0x07, 0x08, 0x09),
        mark_color_disabled: Color::rgb(0x16, 0x17, 0x18),
        label_color: Color::rgb(0x19, 0x1A, 0x1B),
        label_color_disabled: Color::rgb(0x1C, 0x1D, 0x1E),
        ..CheckboxStyle::default()
    };
    let id = allocate(&Checkbox::new("x"));

    let checked = paint_snapshot(&Checkbox::new("x").checked(true).style(style), id, ROW);
    for ink in ["#010203", "#040506", "#070809", "#191A1B"] {
        assert!(
            checked.contains(ink),
            "{ink} is the checked column; got {checked}"
        );
    }
    for ink in ["#0A0B0C", "#0D0E0F", "#1C1D1E"] {
        assert!(
            !checked.contains(ink),
            "{ink} is not the checked column; got {checked}"
        );
    }

    let unchecked = paint_snapshot(&Checkbox::new("x").style(style), id, ROW);
    for ink in ["#0A0B0C", "#0D0E0F", "#191A1B"] {
        assert!(
            unchecked.contains(ink),
            "{ink} is the unchecked column; got {unchecked}"
        );
    }
    assert!(
        !unchecked.contains("#010203"),
        "and not the checked fill; got {unchecked}"
    );

    let disabled = paint_snapshot(
        &Checkbox::new("x").checked(true).disabled(true).style(style),
        id,
        ROW,
    );
    for ink in ["#101112", "#131415", "#161718", "#1C1D1E"] {
        assert!(
            disabled.contains(ink),
            "{ink} is the disabled column; got {disabled}"
        );
    }
    assert!(
        !disabled.contains("#010203") && !disabled.contains("#070809"),
        "disabled overrides checked rather than dimming it; got {disabled}"
    );
}

/// A rim is drawn as a stroke inside the indicator's own edge, so its outer edge
/// is the indicator's and its inner edge is `border_width` in. The engine drew
/// no rim at all, which is why the default's width is `0.0`.
#[test]
fn a_rim_is_stroked_inside_the_indicators_own_edge() {
    let style = CheckboxStyle {
        border_width: 2.0,
        border_radius: 2.0,
        ..CheckboxStyle::default()
    };
    let id = allocate(&Checkbox::new("x"));
    let snapshot = paint_snapshot(&Checkbox::new("x").style(style), id, ROW);
    assert!(
        snapshot.contains("RoundedRect (0.0, 8.0, 16.0, 24.0) r=2.0"),
        "the container carries the style's own radius; got {snapshot}"
    );
    assert!(
        snapshot.contains("Path (1.0, 9.0, 15.0, 23.0) segs=10 fill=none stroke=2.0"),
        "and the rim is stroked *inside* the indicator's edge -- a 2px stroke \
         whose centreline sits a pixel in -- so its outer edge is the box's; got \
         {snapshot}"
    );

    // The default's own rim is none at all: the engine filled the box and stopped.
    let unrounded = paint_snapshot(&Checkbox::new("x"), id, ROW);
    assert!(
        !unrounded.contains("Path"),
        "no rim at width 0.0; got {unrounded}"
    );
}

// --- the two mechanisms the three kits split over -------------------------

/// Cupertino's and Fluent's layer is a rounded box `size + 8` across with four
/// points of padding, and it answers hover alone and vanishes when disabled.
/// Material's is a flat 40 circle that also answers focus and stays while
/// disabled. `StateLayer`'s four gate flags are what tell those apart, and the
/// shape is what tells the checkbox's box from the radio's circle.
#[test]
fn the_state_layer_is_a_box_around_the_indicator_and_gated_four_ways() {
    let cupertino = CheckboxStyle {
        layer: StateLayer {
            color: Color::rgb(0x11, 0x22, 0x33),
            color_pressed: None,
            size: 24.0,
            shape: LayerShape::Box { padding: 4.0 },
            on_hover: true,
            on_press: false,
            on_focus: false,
            suppressed_when_disabled: true,
        },
        ..CheckboxStyle::default()
    };
    let id = allocate(&Checkbox::new("x"));
    let resting = paint_snapshot(&Checkbox::new("x").style(cupertino), id, ROW);
    assert_eq!(
        resting.matches("RoundedRect").count(),
        1,
        "an unhovered checkbox paints only its indicator; got {resting}"
    );

    let mut hovered = Checkbox::new("x").style(cupertino);
    let hovered_id = allocate(&hovered);
    hover(&mut hovered, hovered_id);
    let hover_paint = paint_snapshot(&hovered, hovered_id, ROW);
    assert!(
        hover_paint.contains("RoundedRect (-4.0, 4.0, 20.0, 28.0) r=4.0 #112233FF"),
        "a 24px box -- size + 2*padding -- with the padding added to the radius, \
         centred on the indicator; got {hover_paint}"
    );

    // And suppressed while disabled, which is the flag Cupertino and Fluent set
    // and Material does not.
    let mut off = Checkbox::new("x").style(cupertino).disabled(true);
    let off_id = allocate(&mut off);
    hover(&mut off, off_id);
    let off_paint = paint_snapshot(&off, off_id, ROW);
    assert_eq!(
        off_paint.matches("RoundedRect").count(),
        1,
        "a disabled checkbox draws no layer; got {off_paint}"
    );

    // Material's arm: a circle, and a gate that also opens on focus.
    let material = CheckboxStyle {
        layer: StateLayer {
            color: Color::rgb(0x44, 0x55, 0x66),
            color_pressed: Some(Color::rgb(0x77, 0x88, 0x99)),
            size: 40.0,
            shape: LayerShape::Circle,
            on_hover: true,
            on_press: true,
            on_focus: true,
            suppressed_when_disabled: false,
        },
        ..CheckboxStyle::default()
    };
    let mut focused = Checkbox::new("x").style(material);
    let focused_id = allocate(&mut focused);
    focused.set_focused(true);
    let focus_paint = paint_snapshot(&focused, focused_id, ROW);
    assert!(
        focus_paint.contains("RoundedRect (-12.0, -4.0, 28.0, 36.0) r=20.0 #445566FF"),
        "a flat 40 circle centred on the indicator, opened by focus alone; got \
         {focus_paint}"
    );

    let mut focused_pressed = Checkbox::new("x").style(material);
    let fp_id = allocate(&mut focused_pressed);
    dispatch(
        &mut focused_pressed,
        fp_id,
        ROW,
        floom_types::PointerEvent::primary_down(Point::new(8.0, 16.0)).into(),
    );
    assert!(
        paint_snapshot(&focused_pressed, fp_id, ROW).contains("#778899FF"),
        "and a press takes the pressed tint; a gate with only `on_hover` would \
         not"
    );
}

/// Cupertino's and Fluent's v1 checkboxes both drew
/// `outline(2.0)` in the theme's `primary` around the indicator *and the label*.
/// Material's drew none, because its layer already answers focus. So the two
/// mechanisms must not both fire: the default has neither, and a kit picks one.
#[test]
fn a_focus_ring_goes_around_the_whole_control_and_only_while_focused() {
    let style = CheckboxStyle {
        focus_ring: Some(FocusRing {
            width: 2.0,
            color: Color::rgb(0xAA, 0xBB, 0xCC),
            gap: 0.0,
            radius: None,
        }),
        ..CheckboxStyle::default()
    };
    let id = allocate(&Checkbox::new("x"));
    let unfocused = paint_snapshot(&Checkbox::new("x").style(style), id, ROW);
    assert!(
        !unfocused.contains("#AABBCCFF"),
        "no ring until there is focus; got {unfocused}"
    );

    let mut focused = Checkbox::new("x").style(style);
    let focused_id = allocate(&mut focused);
    focused.set_focused(true);
    let focused_paint = paint_snapshot(&focused, focused_id, ROW);
    assert!(
        focused_paint.contains("stroke=2.0/#AABBCCFF"),
        "a 2px primary ring; got {focused_paint}"
    );
    assert!(
        // The row is 200x32 and the ring is drawn *outside* it, half a stroke
        // width in: (-1, -1, 201, 33). A ring around the 16px indicator would
        // say `Path (-1.0, 7.0, 17.0, 25.0)` instead.
        focused_paint.contains("Path (-1.0, -1.0, 201.0, 33.0)"),
        "around the whole control -- the indicator *and* the label -- not around \
         the 16px box; got {focused_paint}"
    );
}
