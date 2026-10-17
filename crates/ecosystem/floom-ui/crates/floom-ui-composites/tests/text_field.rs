//! `TextField`'s own logic, and the claim that its default style is the pixels
//! the engine painted before the move.
//!
//! These moved with the widget from `floom_widgets/tests/controls.rs` and from
//! `floom-testkit/tests/testkit.rs`, for the reason `radio.rs` moved: a test of a
//! widget's behaviour belongs with the widget, and this file is what has to be
//! deleted with it. The two IME tests came from the harness crate rather than
//! the widgets crate and are the awkward part of the move -- see their own
//! section below.
//!
//! What is new here is the half the move made possible. The old files could not
//! say "the default style is the old palette", because there was no style --
//! they could only assert behaviour. With `TextFieldStyle` in existence every
//! value in its table is a promise about pixels, and the tests below are what
//! hold it to that.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{ElementId, Key, NamedKey, Point, Rect, Widget};
use floom_testkit::TestApp;
use floom_text::measure_text;
use floom_types::{Color, KeyEvent, Length, Modifiers, PointerEvent, Size};
use floom_ui_composites::{TextField, TextFieldStyle};
use floom_widgets::testkit::{
    dispatch, paint_snapshot, semantics_snapshot, AllocatingCreateContext,
};
use floom_widgets::text_width_at;

/// The box the field is laid out in for every test here: wide enough for the
/// text plus its padding, tall enough for a row of 14px type.
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

/// Puts a field into `hovered` by dispatching the pointer event the platform
/// would, rather than by writing to its state -- so the assertion is about the
/// wiring and not about a field the test set itself.
fn hover(field: &mut TextField, id: ElementId) {
    dispatch(
        field,
        id,
        ROW,
        PointerEvent::hover(Point::new(8.0, 16.0)).into(),
    );
}

/// Presses a field, dispatching the pointer-down the platform would.
fn press(field: &mut TextField, id: ElementId) {
    dispatch(
        field,
        id,
        ROW,
        PointerEvent::primary_down(Point::new(8.0, 16.0)).into(),
    );
}

/// The pixel value of a `LayoutRequest` dimension.
///
/// `LayoutRequest` speaks in `Option<Length>`, which is "unspecified" rather
/// than zero, so every assertion about an intrinsic extent has to say which of
/// the two it is looking at.
fn px(length: Option<Length>) -> f32 {
    match length {
        Some(Length::Px(value)) => value,
        other => panic!("expected a pixel length, got {other:?}"),
    }
}

/// Types `text` one character key at a time, which is exactly how committed
/// text reaches the widget: the runner turns an OS `ImeCommit` into one
/// character key-down per scalar, so there is no other door.
fn type_text(field: &mut TextField, id: ElementId, text: &str) {
    for ch in text.chars() {
        dispatch(field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
}

// --- the widget ------------------------------------------------------------

#[test]
fn text_field_edits_at_the_caret() {
    let mut field = TextField::new().placeholder("name");
    let id = allocate(&field);
    field.set_focused(true);

    for ch in ['a', 'b', 'c'] {
        dispatch(&mut field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
    assert_eq!(field.text(), "abc");
    assert_eq!(field.caret(), 3);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::ArrowLeft)).into(),
    );
    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Backspace)).into(),
    );
    assert_eq!(field.text(), "ac");
    assert_eq!(field.caret(), 1);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Delete)).into(),
    );
    assert_eq!(field.text(), "a");
}

#[test]
fn text_field_caret_never_splits_a_codepoint() {
    let mut field = TextField::new();
    let id = allocate(&field);
    field.set_focused(true);

    for ch in ['中', '文'] {
        dispatch(&mut field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
    assert_eq!(field.text(), "中文");

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::ArrowLeft)).into(),
    );
    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Backspace)).into(),
    );
    assert_eq!(field.text(), "文", "caret moved one char, not one byte");
}

#[test]
fn text_field_shortcuts_are_not_text_input() {
    let mut field = TextField::new();
    let id = allocate(&field);
    field.set_focused(true);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Char('a'))
            .with_modifiers(Modifiers::CONTROL)
            .into(),
    );
    assert_eq!(field.text(), "", "Ctrl+A is a shortcut, not typing");
}

#[test]
fn text_field_shows_placeholder_until_filled() {
    let empty = TextField::new().placeholder("name");
    let id = allocate(&empty);
    let hint = empty.semantics_hint();
    assert_eq!(hint.label.as_deref(), Some("name"));
    assert_eq!(
        semantics_snapshot(&empty, id, ROW),
        "label=\"name\" (0.0, 0.0, 200.0, 32.0)"
    );
}

#[test]
fn text_field_submit_reports_current_value() {
    let submitted = Rc::new(Cell::new(String::new()));
    let sink = submitted.clone();
    let mut field = TextField::new().value("hi").on_submit(move |text| {
        sink.set(text.to_string());
    });
    let id = allocate(&field);
    field.set_focused(true);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into(),
    );
    assert_eq!(submitted.take(), "hi");
}

// --- the default style is the engine's old paint ----------------------------

/// **The control for every default value below.** Each is a literal the old
/// `paint` wrote inline, so a default that drifted by one hex digit or one point
/// would repaint every existing caller without failing anything else here.
#[test]
fn the_default_style_is_the_pre_style_pixels() {
    let style = TextFieldStyle::default();

    assert_eq!(style.height, None, "the old height was derived, not pinned");
    assert_eq!(
        style.vertical_padding, 4.0,
        "the old `BUTTON_VERTICAL_PADDING`, doubled into the row height"
    );
    assert_eq!(
        style.padding_horizontal, 8.0,
        "the old `BUTTON_PADDING`, used as the text inset"
    );
    assert_eq!(style.border_radius, 0.0, "the engine drew square corners");
    assert_eq!(style.min_width, 120.0, "the old `TEXTFIELD_MIN_WIDTH`");

    assert_eq!(
        style.border_width, 0.0,
        "the engine drew no rim at all: it filled the bounds and stopped"
    );
    assert_eq!(
        style.border_width_focused, 0.0,
        "so there was no focus affordance on the rim either"
    );
    assert_eq!(
        style.border_color,
        Color::WHITE,
        "unreadable at width 0; white is the fill it would vanish into"
    );
    assert_eq!(style.border_color_focused, Color::WHITE, "ditto");
    assert_eq!(style.border_color_hovered, None);
    assert_eq!(style.border_color_disabled, Color::WHITE, "ditto");

    assert_eq!(
        style.focus_outline_width, 0.0,
        "the engine had no focus ring to draw"
    );
    assert_eq!(style.focus_outline_color, Color::TRANSPARENT, "ditto");

    assert_eq!(style.background_color, Color::WHITE);
    assert_eq!(
        style.background_color_hovered,
        Some(Color::rgb(0xF5, 0xF7, 0xFA)),
        "the engine's one and only hover response, verbatim"
    );
    // Not white: the old fill went through `floom_widgets::resolved_color`, which
    // ranks `disabled` first and halves alpha. `255 / 2` is 127, and a default
    // of `Color::WHITE` here would repaint every disabled field in the repo.
    assert_eq!(
        style.background_color_disabled,
        Color::rgba(0xFF, 0xFF, 0xFF, 0x7F),
        "the half-alpha white `resolved_color` resolved the old fill to"
    );

    assert_eq!(style.text_color, Color::BLACK);
    assert_eq!(
        style.text_color_disabled,
        Color::BLACK,
        "the old paint never dimmed its text"
    );
    assert_eq!(style.placeholder_color, Color::rgb(0x9A, 0xA5, 0xB1));
    assert_eq!(style.preedit_color, Color::rgb(0x2B, 0x6C, 0xB0));
    assert_eq!(style.font_size, 14.0, "the old `LABEL_FONT_SIZE`");
}

/// The same claim one layer down: an unstyled field still *paints* those
/// colours. A default table nothing read would pass the test above and leave
/// the widget green.
#[test]
fn an_unstyled_text_field_paints_the_engine_colours() {
    let id = allocate(&TextField::new());

    let empty = paint_snapshot(&TextField::new().placeholder("name"), id, ROW);
    assert!(
        empty.contains("Rect (0.0, 0.0, 200.0, 32.0) #FFFFFFFF"),
        "the old fill was an unpadded white rect over the whole box; got {empty}"
    );
    assert!(
        empty.contains("\"name\" #9AA5B1FF"),
        "the old placeholder; got {empty}"
    );
    assert_eq!(
        empty.lines().count(),
        2,
        "and nothing else: an empty, non-composing field paints exactly the \
         fill and the placeholder; got {empty}"
    );

    let filled = paint_snapshot(&TextField::new().value("hi"), id, ROW);
    assert!(
        filled.contains("\"hi\" #000000FF"),
        "committed text was black at the shared label size; got {filled}"
    );
}

/// The disabled rule, read off the pixels rather than off the table: the old
/// fill was `floom_widgets::resolved_color(WHITE, WHITE, hover, state)` and
/// `resolved_color` halves alpha when disabled. A default of plain white would
/// pass the table test above and repaint every disabled field.
#[test]
fn a_disabled_field_paints_the_halved_white_the_old_fill_resolved_to() {
    let id = allocate(&TextField::new());
    let dimmed = paint_snapshot(
        &TextField::new().placeholder("name").disabled(true),
        id,
        ROW,
    );
    assert!(
        dimmed.contains("Rect (0.0, 0.0, 200.0, 32.0) #FFFFFF7F"),
        "white at half alpha is what the old fill resolved to; got {dimmed}"
    );
    assert!(
        dimmed.contains("\"name\" #9AA5B1FF"),
        "and the placeholder was never dimmed by the old paint; got {dimmed}"
    );
}

// --- the interaction states are load-bearing -------------------------------

/// The hover variant is the engine's only interaction response, so
/// `background_color_hovered` is the one field in the table a default that
/// quietly dropped would be caught by -- and only if this test exists.
///
/// The last paragraph is the point: with `background_color_hovered` set to
/// `None` an idle field and a hovered one paint the *same* string, which is
/// what makes the `assert_eq!` a claim about the resolution rather than a
/// restatement of the table.
#[test]
fn hovering_repaints_the_field_because_the_hover_fill_is_load_bearing() {
    let id = allocate(&TextField::new().placeholder("name"));
    let idle = paint_snapshot(&TextField::new().placeholder("name"), id, ROW);

    let mut hovered = TextField::new().placeholder("name");
    let hovered_id = allocate(&hovered);
    hover(&mut hovered, hovered_id);
    let hover_paint = paint_snapshot(&hovered, hovered_id, ROW);
    assert!(
        hover_paint.contains("#F5F7FAFF"),
        "the old hover fill; got {hover_paint}"
    );
    assert_ne!(hover_paint, idle, "hovering must not paint the idle field");

    // The load-bearing half: with the variant removed there is nothing left for
    // the state to change, because `None` falls back to the base colour rather
    // than to "no fill at all".
    let flat = TextFieldStyle {
        background_color_hovered: None,
        ..TextFieldStyle::default()
    };
    let mut plain = TextField::new().placeholder("name").style(flat);
    let plain_id = allocate(&mut plain);
    hover(&mut plain, plain_id);
    assert_eq!(
        paint_snapshot(&plain, plain_id, ROW),
        idle,
        "a style with no hover variant paints one field for every state"
    );
}

/// **The press rank, which looks wrong and is not.** The engine's fill went
/// through `resolved_color(base, pressed, hovered, state)`, which checks
/// `pressed` first, and the engine passed white for pressed -- so the old field
/// went back to white when you pressed it, lighter than its own hover. That
/// reads like an artifact, and it is kept anyway: the alternative is a pixel
/// change nobody asked for. `pressed` outranking `hovered` is the part that
/// matters, because a pointer-down sets both flags at once, so checking hover
/// first would mean a press could never be seen.
#[test]
fn pressing_a_hovered_field_is_not_the_same_as_hovering_it() {
    let id = allocate(&TextField::new().placeholder("name"));
    let idle = paint_snapshot(&TextField::new().placeholder("name"), id, ROW);

    let mut hovered = TextField::new().placeholder("name");
    let hovered_id = allocate(&mut hovered);
    hover(&mut hovered, hovered_id);
    let hover_paint = paint_snapshot(&hovered, hovered_id, ROW);

    let mut pressed = TextField::new().placeholder("name");
    let pressed_id = allocate(&mut pressed);
    hover(&mut pressed, pressed_id);
    press(&mut pressed, pressed_id);
    let press_paint = paint_snapshot(&pressed, pressed_id, ROW);

    assert_ne!(
        press_paint, hover_paint,
        "the press must be visible against the hover, which is what ranking \
         `pressed` first buys"
    );
    assert_eq!(
        press_paint, idle,
        "the engine's pressed fill *was* white, so this is byte-identical to \
         the field at rest -- kept rather than quietly changed"
    );

    // `None` means "no variant of my own", which everywhere else in this crate
    // means the *base* colour rather than the state below it. So a style that
    // drops the press variant paints a pressed field the same as an idle one --
    // and saying so here is what stops a later reader assuming it inherits hover.
    let without = TextFieldStyle {
        background_color_pressed: None,
        ..TextFieldStyle::default()
    };
    let mut plain = TextField::new().placeholder("name").style(without);
    let plain_id = allocate(&mut plain);
    hover(&mut plain, plain_id);
    press(&mut plain, plain_id);
    assert_eq!(
        paint_snapshot(&plain, plain_id, ROW),
        idle,
        "`None` falls back to the base fill, not to the hover fill"
    );
    assert_ne!(
        paint_snapshot(&plain, plain_id, ROW),
        hover_paint,
        "and it is therefore not the hover fill either, which is the other \
         reading and the one this assertion exists to rule out"
    );
}

// --- the rim and the focus ring ---------------------------------------------

/// The default draws no rim, because it never did. A kit that wants one names a
/// thickness and gets a stroke whose *outer* edge is the layout bounds, with
/// the text inset by that thickness so a 2px rim cannot sit under the first
/// glyph.
#[test]
fn a_style_with_a_rim_strokes_one_inside_the_bounds_and_insets_the_text() {
    let rimmed = TextFieldStyle {
        border_width: 2.0,
        border_radius: 4.0,
        border_color: Color::rgb(0x11, 0x22, 0x33),
        ..TextFieldStyle::default()
    };
    let id = allocate(&TextField::new());
    let painted = paint_snapshot(&TextField::new().value("hi").style(rimmed), id, ROW);

    assert!(
        painted.contains("RoundedRect (0.0, 0.0, 200.0, 32.0) r=4.0 #FFFFFFFF"),
        "the fill is rounded once the style gives it a radius; got {painted}"
    );
    assert!(
        painted.contains("2.0/#112233FF"),
        "a 2px #112233 rim; got {painted}"
    );
    // The stroke runs from 1.0 to 3.0 inside the box, so the path's own bounds
    // are inset by half a width.
    assert!(
        painted.contains("Path (1.0, 1.0, 199.0, 31.0)"),
        "half a width inside each edge, which is what puts the outer edge on \
         the layout bounds; got {painted}"
    );
    assert!(
        painted.contains("Text (10.0, 9.0) 14.0 \"hi\""),
        "the text origin is padding_horizontal + border_width, so the first \
         glyph clears the rim; got {painted}"
    );
}

/// A kit's focus answer is either a thicker rim or a ring outside the box.
/// Only the ring is new geometry, and it has to be drawn *outside*: a ring
/// stroked inside the layout box is clipped by it and therefore never seen.
#[test]
fn the_focus_ring_is_drawn_outside_the_layout_box() {
    let ringed = TextFieldStyle {
        focus_outline_width: 2.0,
        focus_outline_color: Color::rgb(0x2B, 0x6C, 0xB0),
        border_radius: 4.0,
        ..TextFieldStyle::default()
    };
    let id = allocate(&TextField::new());
    let mut focused = TextField::new().value("hi").style(ringed);
    focused.set_focused(true);
    let painted = paint_snapshot(&focused, id, ROW);

    // `stroke_rounded_outline` strokes a path inset by half the width, so the
    // centreline runs one point outside each edge and the stroke covers
    // -2.0..=0.0: the ring sits wholly outside the layout box, which is the
    // only place a ring is visible at all.
    assert!(
        painted.contains("Path (-1.0, -1.0, 201.0, 33.0)"),
        "the ring's centreline is half a width outside every edge; got {painted}"
    );
    assert!(
        painted.contains("2.0/#2B6CB0FF"),
        "a 2px #2B6CB0 ring; got {painted}"
    );

    // And it is a focus affordance: the same field unfocused draws no ring at
    // all, which is the state the default is in.
    let idle = TextField::new().value("hi").style(ringed);
    assert!(
        !paint_snapshot(&idle, id, ROW).contains("2.0/#2B6CB0FF"),
        "an unfocused field draws no ring"
    );
}

// --- layout reads the style ------------------------------------------------

/// A kit's 56pt field must be measured at the size it is drawn at. The old
/// `layout_request` measured the label at `LABEL_FONT_SIZE` and derived the
/// height from the same constant, so a style that changed `font_size` would
/// have reserved the engine's row.
#[test]
fn layout_reads_the_style_rather_than_the_engine_constants() {
    let request = |field: &TextField| field.layout_request().expect("an intrinsic size");
    // Long enough to clear the 120px floor, which is what makes the *width*
    // assertions below about the padding rather than about the floor.
    let long = "a reasonably long piece of typed text";

    // The default is the old arithmetic: a measured line height plus 2x4, and
    // the shown text plus 2x8 with a 120px floor.
    let derived = request(&TextField::new().value("hi"));
    let line = measure_text(" ", 14.0, f64::INFINITY).height as f32;
    assert_eq!(px(derived.height), line + 4.0 * 2.0, "the old row height");
    assert_eq!(
        px(derived.width),
        (text_width_at("hi", 14.0) + 8.0 * 2.0).max(120.0),
        "shown text + 2x padding_horizontal, floored at min_width"
    );

    let pinned = TextFieldStyle {
        height: Some(56.0),
        font_size: 16.0,
        ..TextFieldStyle::default()
    };
    let kit = request(&TextField::new().value("hi").style(pinned));
    assert_eq!(
        px(kit.height),
        56.0,
        "a pinned height wins over the measurement"
    );
    assert!(
        px(request(&TextField::new().value(long).style(pinned)).width)
            > px(request(&TextField::new().value(long)).width),
        "and the text is measured wider at 16px than at 14px"
    );

    // The floor and the inset are the style's too.
    let wide = TextFieldStyle {
        padding_horizontal: 20.0,
        ..TextFieldStyle::default()
    };
    assert_eq!(
        px(request(&TextField::new().value(long).style(wide)).width),
        text_width_at(long, 14.0) + 40.0,
        "2x padding_horizontal around the shown text"
    );
    assert_eq!(
        px(request(&TextField::new().style(TextFieldStyle {
            min_width: 300.0,
            ..TextFieldStyle::default()
        }))
        .width),
        300.0,
        "an empty field is still floored at min_width"
    );
}

// --- max_length: v1's half, and the half it never had ----------------------

/// What v1's three kits did, exactly: `value("...")` clamps the string it is
/// given. `set_value` is the same clamp through the other door.
#[test]
fn max_length_clamps_the_value_it_is_given() {
    let field = TextField::new().value("abcdefghij").max_length(Some(8));
    assert_eq!(field.text(), "abcdefgh");
    assert_eq!(field.length_limit(), Some(8));

    // The other door. `set_value` is the in-place spelling of `value()` and goes
    // through the same clamp.
    let mut field = TextField::new().max_length(Some(4));
    field.set_value("abcdefghij");
    assert_eq!(field.text(), "abcd");
    field.set_value("ab");
    assert_eq!(field.text(), "ab", "and a shorter value is legal again");
}

/// **The widening.** v1's `max_length(8)` truncated the value you passed in and
/// did nothing about the eight characters you then typed -- all three kits
/// shipped that half-implemented. Here the typing path is clamped too, and this
/// is the test that says so.
#[test]
fn max_length_clamps_typing_and_not_only_the_builder() {
    let mut field = TextField::new().max_length(Some(8));
    let id = allocate(&field);
    field.set_focused(true);

    type_text(&mut field, id, "abcdefghij");
    assert_eq!(
        field.text(),
        "abcdefgh",
        "the ninth and tenth keystrokes stop at the cap"
    );
    assert_eq!(field.caret(), 8, "and the caret stops with the value");

    // The refusal is not a truncation: the eighth character is still the
    // eighth, rather than the first being pushed off the end.
    assert_eq!(
        field.text().chars().last(),
        Some('h'),
        "the oldest character is the one kept, because a limit that dropped it \
         would be deleting something the user did not ask to lose"
    );
}

/// A limit counted in *bytes* would cut a CJK or emoji value mid-scalar. v1
/// clamped with `value.chars().take(n)` and so does this, which is what keeps
/// `String` valid UTF-8 at all.
#[test]
fn max_length_counts_characters_and_never_bytes() {
    // Five characters, fifteen bytes.
    let mut field = TextField::new().max_length(Some(3));
    let id = allocate(&field);
    field.set_focused(true);

    type_text(&mut field, id, "你好世界");
    assert_eq!(field.text(), "你好世", "three characters, not three bytes");
    assert_eq!(field.text().len(), 9, "which is three three-byte scalars");
    assert_eq!(field.caret(), 3, "the caret counts characters too");

    // The builder path is byte-safe for the same reason.
    let emoji = TextField::new().value("a🎉b").max_length(Some(2));
    assert_eq!(emoji.text(), "a🎉");
    assert_eq!(emoji.text().chars().count(), 2);
}

/// `on_change` is the app's own view of the value, so a keystroke the cap
/// swallowed must not report one.
#[test]
fn a_swallowed_keystroke_does_not_notify() {
    let seen = Rc::new(Cell::new(String::new()));
    let sink = seen.clone();
    let mut field = TextField::new()
        .max_length(Some(2))
        .on_change(move |text| sink.set(text.to_string()));
    let id = allocate(&field);
    field.set_focused(true);

    type_text(&mut field, id, "ab");
    assert_eq!(seen.take(), "ab");

    type_text(&mut field, id, "c");
    assert_eq!(
        seen.take(),
        "",
        "nothing changed, so there is nothing to report"
    );
    assert_eq!(field.text(), "ab");
}

// --- the IME tests, moved from the harness crate ----------------------------

/// These two were `floom-testkit`'s, not the widgets crate's, and they moved here
/// rather than being deleted.
///
/// They test the *harness*: that `TestApp::type_text` focuses the target and
/// delivers committed text, and that a preedit reaches the widget without
/// becoming its value. The field is the only widget in the tree that has either
/// a value or a composition, so it is the only thing they can be written
/// against -- and `floom-testkit` is a root-workspace crate that must not depend
/// on the satellite `floom-ui` workspace, so they could not stay.
///
/// They still test the harness: `floom-ui-composites` depends on `floom-testkit` as
/// a dev-dependency, so the same `TestApp`, the same `commit`, the same
/// `preedit` and the same `widget::<TextField>()` recovery run underneath. What
/// changed is which crate the test is compiled into, not what it exercises.
#[test]
fn type_text_focuses_the_target_then_commits() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), TextField::new().placeholder("name"));
    assert_eq!(app.widget::<TextField>(app.root()).text(), "");

    assert!(app.type_text(app.root(), "hi"), "typing repainted");
    assert_eq!(app.widget::<TextField>(app.root()).text(), "hi");
    assert!(app.is_focused(app.root()), "typing focused the field");

    // Committed text goes through the same path, and the field announces it.
    app.commit(" there");
    assert_eq!(app.widget::<TextField>(app.root()).text(), "hi there");
    app.assert_text(app.root(), "hi there");
}

#[test]
fn a_preedit_does_not_change_the_value() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), TextField::new().placeholder("name"));
    app.focus(app.root());

    app.preedit("ni");
    let field = app.widget::<TextField>(app.root());
    assert_eq!(
        field.text(),
        "",
        "an in-progress composition is not text yet"
    );
    assert_eq!(field.preedit(), "ni");
    assert!(field.is_composing());

    app.commit("你");
    let field = app.widget::<TextField>(app.root());
    assert_eq!(field.text(), "你", "committing finalizes the composition");
    assert!(!field.is_composing());
}

/// The third door, and the one the two above only half-cover: an OS
/// `ImeCommit` is turned into character key-downs, so a commit is typing. A cap
/// that only guarded `value()` would let a whole committed composition past
/// it, which for CJK is several characters arriving at once.
#[test]
fn max_length_clamps_a_committed_composition_too() {
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        TextField::new().placeholder("name").max_length(Some(3)),
    );
    app.focus(app.root());

    assert!(app.commit("你好世界"));
    assert_eq!(
        app.widget::<TextField>(app.root()).text(),
        "你好世",
        "the commit arrives as four character keys and the cap is on that path"
    );
}
