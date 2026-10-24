//! Fluent's text field as a kit crate: the token translation, and the one
//! thing this kit answers that the other two answer differently.
//!
//! It is a focus affordance -- it rings the box with a `2.0`-wide
//! `primary` outline outside the rim, rather than thickening that rim
//! or tinting it on hover.
//!
//! `tests/api.rs` in this crate has never been compiled: it was written against
//! the v1 API and nothing built it, which is how three kits came to disagree
//! with themselves about a text field for this long. It has been rewritten for 2.0
//! and now runs; this file covers what it does not.
//!
//! ## Why every test here pins the design system first
//!
//! `MultiTheme::activate` is documented in the theme crate as a no-op
//! placeholder, and `MultiTheme::ds()` reads a process-wide signal at access
//! time. So `with_theme(MultiTheme::new_for(DesignSystem::Fluent))` does **not**
//! make this kit read Fluent's tokens: it reads whichever design system the process
//! happens to be in, which is Material until something says otherwise.
//! `theme()` below says otherwise, per thread, before every number is read.
//!
//! That is a property of the theme crate and not of this kit, so nothing in the
//! kit works around it -- but it is the only reason this kit's own 32pt
//! row and its own 14pt type are observable at all today, and it is why
//! the numbers are pinned after the pin rather than instead of it.
//!
//! ## The tripwires
//!
//! * `the_focus_affordance_is_a_ring_outside_the_box`, which fails if this kit
//!   swaps its ring for Material's thicker rim or Cupertino's hovered tint,
//! * `focusing_strokes_the_ring_and_the_rim_and_at_rest_only_the_rim`, which
//!   fails if the ring moves inside the layout box where it cannot be seen,
//! * `the_placeholder_and_the_preedit_are_inherited_not_themed`, which fails if
//!   one of the two colours v1 never named quietly acquires a theme colour,
//! * `the_field_measures_its_own_content_rather_than_filling_its_parent`, which
//!   fails if someone re-adds the full-width layout v1 had.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{ElementId, Key, Length, Point, Rect, Widget};
use floom_types::{Color, KeyEvent, PointerEvent, PointerPhase};
use floom_ui_composites::{TextField, TextFieldStyle};
use floom_widgets::testkit::{dispatch, AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::text_width_at;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, TextFieldTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color as TokenColor;
use floom_widget_fluent_text_field::FluentTextField;

/// The box the field is laid out and painted in: tall enough for the tallest of
/// the three kits' token rows, so a paint is never clipped and a focus ring has
/// room to sit outside it.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 240.0,
    y1: 48.0,
};

/// v1's disabled ink and disabled rim: `on_surface` at 38. Written out so the
/// assertion can name the number rather than only that it is not 97.
const DISABLED_ALPHA: u8 = 38;

/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;

/// What all three v1 `resolved_style()` functions claimed for the disabled rim
/// and the disabled ink, and what none of them ever painted: nothing read those
/// structs, and every `view()` resolved its colours inline. If a number here ever
/// becomes 97, this crate has started trusting a dead struct.
const V1_DEAD_ALPHA: u8 = 97;

/// Pins the design system for this test's thread, then hands back a theme.
///
/// Per thread because the theme crate keeps the signal in a `thread_local` and
/// the harness gives each test its own, so pinning here cannot move another
/// test's numbers.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

/// The design-system colour at an explicit alpha, in the engine's type. The same
/// conversion the kit makes, so a test states a colour and not a type.
fn tint(color: TokenColor, alpha: u8) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/// Every style is read through an explicit theme rather than the app-wide signal,
/// so these tests cannot be moved by another test's global state.
fn style() -> TextFieldStyle {
    FluentTextField::new().with_theme(theme()).resolved_style()
}

/// This kit's token row, read the way the kit reads it.
fn field_tokens() -> TextFieldTokens {
    match theme().component(ComponentId::TextField) {
        ComponentTokens::TextField(tokens) => tokens,
        _ => unreachable!("ComponentId::TextField must resolve to TextFieldTokens"),
    }
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Paint into a recorder and hand back both the ops and the snapshot text, so a
/// failing assertion can say what was actually drawn.
fn paint(field: &TextField) -> (Vec<PaintOp>, String) {
    let id = allocate(field);
    let mut recorder = SceneRecorder::new(ROW);
    field.paint(&mut recorder, id);
    recorder.finish();
    (recorder.ops().to_vec(), recorder.snapshot())
}

/// Every stroked path as `(width, ink)`, in the order `paint` issued them.
///
/// That order is load-bearing for the ring: `TextField::paint` strokes the focus
/// ring *before* the rim, because the ring sits outside the bounds and the rim
/// inside them and neither covers the other.
fn strokes(ops: &[PaintOp]) -> Vec<(f64, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::Path {
                stroke: Some((width, color)),
                ..
            } => Some((*width, *color)),
            _ => None,
        })
        .collect()
}

/// Every filled rounded box as `(bounds, ink)` -- the fill, and only the fill.
fn fills(ops: &[PaintOp]) -> Vec<(Rect, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::RoundedRect { bounds, color, .. } => Some((*bounds, *color)),
            _ => None,
        })
        .collect()
}

/// The pixel value of a `LayoutRequest` dimension. `LayoutRequest` speaks
/// `Option<Length>`, which is "unspecified" rather than zero, so every assertion
/// about an intrinsic extent has to say which of the two it is looking at.
fn px(length: Option<Length>) -> f32 {
    match length {
        Some(Length::Px(value)) => value,
        other => panic!("expected a pixel length, got {other:?}"),
    }
}

fn hover(field: &mut TextField, id: ElementId) {
    dispatch(
        field,
        id,
        ROW,
        PointerEvent::hover(Point::new(8.0, 24.0)).into(),
    );
}

fn leave(field: &mut TextField, id: ElementId) {
    let event = PointerEvent {
        phase: PointerPhase::Leave,
        ..PointerEvent::hover(Point::new(8.0, 24.0))
    };
    dispatch(field, id, ROW, event.into());
}

/// Types `text` one character key at a time, which is exactly how committed text
/// reaches the widget -- including a committed IME composition, because the
/// runner turns an OS `ImeCommit` into one character key-down per scalar.
fn type_text(field: &mut TextField, id: ElementId, text: &str) {
    for ch in text.chars() {
        dispatch(field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
}

// --- the tokens --------------------------------------------------------------

/// The three numbers that are this design system's answer, read from
/// `TextFieldTokens` rather than written down -- and pinned as literals too,
/// because a literal is the only assertion that would notice the theme crate
/// changing one.
#[test]
fn the_geometry_is_this_kits_text_field_tokens() {
    let tokens = field_tokens();
    let style = style();

    assert_eq!(style.height, Some(tokens.height), "the pinned row height");
    assert_eq!(style.height, Some(32.0), "v1 read `tokens.height`");
    assert_eq!(
        style.padding_horizontal, tokens.padding_horizontal,
        "the text inset, either side"
    );
    assert_eq!(style.padding_horizontal, 8.0);
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the rim's corners"
    );
    assert_eq!(style.border_radius, 4.0);

    // v1 wrote `Some(tokens.height)`'s un-versioned form: the height was pinned,
    // never derived, so `vertical_padding` was dead in v1 and is inherited here.
    assert_eq!(
        style.vertical_padding,
        TextFieldStyle::default().vertical_padding,
        "read only when `height` is `None`, and this kit never leaves it `None`"
    );
}

/// The fill is `surface` and the ink is `on_surface` at the body-medium size,
/// both read from the theme rather than written down.
#[test]
fn the_fill_is_surface_and_the_ink_is_on_surface_at_body_medium() {
    let binding = theme();
    let colors = binding.colors();
    let typography = binding.typography();
    let style = style();

    assert_eq!(style.background_color, tint(colors.surface, 255));
    assert_eq!(style.text_color, tint(colors.on_surface, 255));
    assert_eq!(style.font_size, typography.body.medium.font_size);
    assert_eq!(
        style.font_size, 14.0,
        "v1 wrote `typography().body.medium.font_size` -- the token, not a number"
    );
}

// --- the rim -----------------------------------------------------------------

/// `1.0` of `outline` at rest, `primary` while focused, one point thick in both.
#[test]
fn the_resting_rim_is_one_point_of_outline_and_focus_recolours_it() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(
        style.border_width, 1.0,
        "v1 wrote `.border(1.0)` -- all three kits"
    );
    assert_eq!(
        style.border_color,
        tint(colors.outline, 255),
        "the resting rim"
    );
    assert_eq!(
        style.border_color_focused,
        tint(colors.primary, 255),
        "and `primary` while focused, which all three kits wrote"
    );
}

/// **38, not the 97 that v1's dead `resolved_style` claimed.** Every one of the
/// three kits' `resolved_style()` functions wrote `on_surface` at 97 and none of
/// them was ever called; every one of their `view()` bodies wrote 38 inline and
/// that is what a screen showed. The dead number is asserted against so a later
/// reader who finds it in git history knows which one was painted.
#[test]
fn the_disabled_palette_is_on_surface_at_38_and_4() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(
        style.border_color_disabled,
        tint(colors.on_surface, DISABLED_ALPHA),
        "the disabled rim"
    );
    assert_eq!(
        style.text_color_disabled,
        tint(colors.on_surface, DISABLED_ALPHA),
        "and the disabled ink, which v1 wrote twice from the same expression"
    );
    assert_eq!(
        style.background_color_disabled,
        tint(colors.on_surface, DISABLED_FILL_ALPHA),
        "and a fill at 4, which is a different number again"
    );
    assert_ne!(
        style.border_color_disabled.a, V1_DEAD_ALPHA,
        "control: 97 is what the dead `resolved_style` claimed and what v1 never painted"
    );
    assert_ne!(
        style.text_color_disabled.a, V1_DEAD_ALPHA,
        "ditto for the ink"
    );
}

/// The two interaction fills stay at the engine's defaults on purpose. v1 styled
/// its `TextInput` with `color` and `font_size` only and left the fill to the
/// wrapping `Container`, which had no hover or press rule either -- so this kit
/// never had a fill response to port, and replacing the engine's with `None`
/// would be a design decision about hover rather than a port.
#[test]
fn the_interaction_fills_are_inherited_because_v1_named_none() {
    let default = TextFieldStyle::default();
    let style = style();

    assert_eq!(
        style.background_color_hovered,
        default.background_color_hovered
    );
    assert_eq!(
        style.background_color_pressed,
        default.background_color_pressed
    );
    assert_eq!(
        default.background_color_hovered,
        Some(Color::rgb(0xF5, 0xF7, 0xFA)),
        "the engine's own hover fill -- engine behaviour, not this kit's"
    );
}

// --- the two colours v1 never named ------------------------------------------

/// **The gap, kept visible.** `placeholder_color` and `preedit_color` were in no
/// kit's style struct, because v1's `TextInput` supplied its own defaults and
/// those defaults died with the v1 widget. They are inherited here, which is
/// *visible* rather than *right*, and the two `assert_ne` lines say which theme
/// answers this port declined to pick: `on_surface_variant` for a placeholder and
/// `primary` for a preedit are the obvious candidates, and choosing either in
/// three kit crates would be three unreviewed decisions wearing a port's clothes.
#[test]
fn the_placeholder_and_the_preedit_are_inherited_not_themed() {
    let binding = theme();
    let colors = binding.colors();
    let default = TextFieldStyle::default();
    let style = style();

    assert_eq!(style.placeholder_color, default.placeholder_color);
    assert_eq!(style.preedit_color, default.preedit_color);

    // Named, because "same as the default" is a claim that a later edit to the
    // default would silently satisfy.
    assert_eq!(default.placeholder_color, Color::rgb(0x9A, 0xA5, 0xB1));
    assert_eq!(default.preedit_color, Color::rgb(0x2B, 0x6C, 0xB0));

    assert_ne!(
        style.placeholder_color,
        tint(colors.on_surface_variant, 255),
        "control: `on_surface_variant` is the obvious theme answer and it is not this one"
    );
    assert_ne!(
        style.preedit_color,
        tint(colors.primary, 255),
        "control: `primary` is the obvious theme answer and it is not this one"
    );
}

// --- layout ------------------------------------------------------------------

/// **The loss, pinned.** v1 wrapped the input in a `Container` with
/// `.width_pct(100.0)`, so a text field was always as wide as its parent. The
/// engine's `TextField` reports an intrinsic width and lets the host size it, so
/// the field is now as wide as the text in it.
///
/// The alternative -- an `Option<f32>` on `TextFieldStyle` -- is a layout policy
/// wearing a style field, and `TextFieldStyle` describes ink. Putting the
/// "fill the row" behaviour back belongs to the widget or to the host, not here.
#[test]
fn the_field_measures_its_own_content_rather_than_filling_its_parent() {
    let style = style();
    let field = FluentTextField::new()
        .with_theme(theme())
        .value("hi")
        .view();
    let request = field.layout_request().expect("an intrinsic size");

    assert_eq!(
        px(request.height),
        style.height.expect("this kit pins its height"),
        "the row is still 32pt tall; only the width is in question"
    );
    assert_eq!(
        px(request.width),
        (text_width_at("hi", style.font_size) + style.padding_horizontal * 2.0)
            .max(style.min_width),
        "as wide as the text plus its padding, floored at `min_width`"
    );
    assert!(
        px(request.width) < ROW.width() as f32,
        "control: v1's `.width_pct(100.0)` would have filled this {}pt-wide box",
        ROW.width()
    );
    assert_eq!(style.min_width, 120.0, "the engine's floor, inherited");

    // An empty field is still the floor rather than zero.
    let empty = FluentTextField::new()
        .with_theme(theme())
        .placeholder("Name")
        .view();
    let request = empty.layout_request().expect("an intrinsic size");
    assert!(
        px(request.width) >= style.min_width,
        "an empty field is usable: the placeholder sets the floor"
    );
}

// --- max_length --------------------------------------------------------------

/// v1's `max_length` clamped only the string the kit was handed, in three
/// methods, all of them keeping its own `value` signal in step. So
/// `max_length(8)` bounded the value you passed in and nothing you typed -- a
/// half-implemented limit, shipped identically by three designs.
#[test]
fn max_length_reaches_the_engine_and_clamps_typing_too() {
    let capped = FluentTextField::new()
        .with_theme(theme())
        .value("abcdefghij")
        .max_length(8)
        .view();
    assert_eq!(
        capped.length_limit(),
        Some(8),
        "the prop crossed into the widget"
    );
    assert_eq!(
        capped.text(),
        "abcdefgh",
        "and the builder path still clamps"
    );

    // The widening: the typing path, which v1 did not reach.
    let mut typed = FluentTextField::new()
        .with_theme(theme())
        .max_length(8)
        .view();
    let id = allocate(&typed);
    typed.set_focused(true);
    type_text(&mut typed, id, "abcdefghij");
    assert_eq!(
        typed.text(),
        "abcdefgh",
        "the ninth and tenth keystrokes stop at the cap, which v1 never did"
    );
    assert_eq!(
        typed.text().chars().last(),
        Some('h'),
        "and the refusal is not a truncation: the oldest character is the one kept"
    );
}

// --- on_change ---------------------------------------------------------------

/// **The `initialized` flag is gone.** v1 fired `on_change` from a
/// `create_effect` that had to be told, by an `Rc<Cell<bool>>`, to ignore its own
/// first run -- so a field built with a value reported that value back to the app
/// as though the user had typed it. The engine reports a keystroke and nothing
/// else, which is why this test starts from "nothing was reported".
#[test]
fn on_change_reports_typing_and_not_the_builders_value() {
    let seen = Rc::new(Cell::new(String::new()));
    let sink = seen.clone();
    let mut field = FluentTextField::new()
        .with_theme(theme())
        .value("Ada")
        .on_change(move |text| sink.set(text.to_string()))
        .view();

    assert_eq!(
        seen.take(),
        "",
        "v1 needed `initialized` to swallow this call; the engine never makes it"
    );

    let id = allocate(&field);
    field.set_focused(true);
    assert_eq!(
        field.caret(),
        3,
        "the builder ends the caret after the text it was handed. `set_value` \
         itself only clamps the caret into range, which is right for a \
         programmatic update mid-typing -- but at construction there is no \
         position to keep, so leaving it at zero put the first keystroke \
         *before* the A"
    );
    dispatch(&mut field, id, ROW, KeyEvent::down(Key::Char('!')).into());
    let reported = seen.take();
    assert_eq!(field.text(), "Ada!", "appended, not prepended");
    assert_eq!(
        reported,
        field.text(),
        "and `on_change` reports the engine's own value, because the kit does not \
         keep a second copy of it that could disagree"
    );
}

// --- password ----------------------------------------------------------------

/// **A prop that renders nothing, on purpose.** `is_password` was already dead
/// in v1 -- all three kits carried it and none of their `view()` bodies read it
/// -- and the engine's `TextField` has no mask. The prop and the builder stay, so
/// a caller asking for a mask still asks for one and the gap stays visible; a
/// kit that started masking on its own would be disagreeing with the widget
/// about what is on screen.
#[test]
fn password_is_carried_and_still_renders_in_the_clear() {
    let kit = FluentTextField::new()
        .with_theme(theme())
        .value("hunter2")
        .password(true);
    assert!(
        kit.props().is_password,
        "the prop is carried, so a caller asking for a mask is recorded as asking"
    );

    let (_, snapshot) = paint(&kit.view());
    assert!(
        snapshot.contains("\"hunter2\""),
        "v1 drew the value in the clear too -- `is_password` was never read:\n{snapshot}"
    );
    assert!(
        !snapshot.contains('*') && !snapshot.contains('\u{2022}'),
        "and this port does not start masking on its own:\n{snapshot}"
    );
}

// --- disabled ----------------------------------------------------------------

#[test]
fn a_disabled_field_is_not_the_enabled_one() {
    let binding = theme();
    let colors = binding.colors();

    let enabled = FluentTextField::new()
        .with_theme(theme())
        .value("hi")
        .view();
    let disabled = FluentTextField::new()
        .with_theme(theme())
        .value("hi")
        .disabled(true)
        .view();
    assert!(
        disabled.interaction().disabled,
        "the flag reached the widget"
    );

    let (_, enabled_snapshot) = paint(&enabled);
    let (disabled_ops, disabled_snapshot) = paint(&disabled);

    assert_ne!(
        enabled_snapshot, disabled_snapshot,
        "a disabled field must not paint the enabled one:\n{disabled_snapshot}"
    );
    assert_eq!(
        fills(&disabled_ops)[0].1,
        tint(colors.on_surface, DISABLED_FILL_ALPHA),
        "the fill goes to `on_surface` at 4:\n{disabled_snapshot}"
    );
    assert_eq!(
        strokes(&disabled_ops)[0].1,
        tint(colors.on_surface, DISABLED_ALPHA),
        "and so does the rim, at 38:\n{disabled_snapshot}"
    );
}

/// v1's kit styled the disabled colours itself, and v1's `TextInput` was styled
/// with `set_disabled` too. The engine refuses key input while disabled, so a
/// disabled field is a field that cannot be typed into -- which is what makes the
/// dim worth having.
#[test]
fn a_disabled_field_does_not_accept_typing() {
    let mut disabled = FluentTextField::new()
        .with_theme(theme())
        .disabled(true)
        .view();
    let id = allocate(&disabled);
    disabled.set_focused(true);
    type_text(&mut disabled, id, "nope");
    assert_eq!(disabled.text(), "", "the engine's own rule, not this kit's");
}

// --- the props cross into the widget -----------------------------------------

#[test]
fn the_props_cross_into_the_widget() {
    let field = FluentTextField::new()
        .with_theme(theme())
        .placeholder("Name")
        .value("Ada")
        .password(true)
        .max_length(8)
        .disabled(true)
        .view();

    assert_eq!(field.text(), "Ada", "the value");
    assert_eq!(
        field.semantics_hint().label.as_deref(),
        Some("Ada"),
        "a filled field is announced by its value; only an empty one falls back \
         to the placeholder"
    );
    assert!(field.interaction().disabled, "and the disabled flag");
    assert_eq!(field.length_limit(), Some(8), "and the cap");

    let empty = FluentTextField::new()
        .with_theme(theme())
        .placeholder("Name")
        .view();
    assert_eq!(
        empty.semantics_hint().label.as_deref(),
        Some("Name"),
        "so the placeholder does reach assistive technology while empty"
    );
}

// --- the focus affordance --

/// **The tripwire for the three kits converging.** Fluent rings the box on
/// focus, Material thickens its rim, Cupertino tints it on hover.
///
/// The two `border_width_focused` lines are the whole of this kit's answer
/// against Material's, and the `assert_ne` on `focus_ring` is the whole
/// of it against Cupertino's -- so all three answers are named in one place.
#[test]
fn the_focus_affordance_is_a_ring_outside_the_box() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    let ring = style.focus_ring.expect(
        "v1 drew `.focus(|s| s.outline(2.0).outline_color(primary))`, and \
                 that ring IS this kit's focus affordance",
    );
    assert_eq!(ring.width, 2.0, "v1 wrote `outline(2.0)`");
    assert_eq!(
        ring.color,
        tint(colors.primary, 255),
        "v1's ring was an undimmed `primary`"
    );
    assert_eq!(
        ring.gap, 0.0,
        "flush against the field: Compose's `outline()` has no inset"
    );
    assert_eq!(
        ring.radius, None,
        "no pinned radius, so the ring follows the field's own corners"
    );
    assert_eq!(style.border_width, 1.0, "v1 wrote `.border(1.0)`");
    assert_eq!(
        style.border_width_focused, 1.0,
        "unconditionally, so the rim itself never thickens -- Material's is the \
         one that does"
    );
    assert!(
        style.border_color_hovered.is_none(),
        "and v1 gave this kit no hovered rim: Cupertino's `primary` at 120 is the \
         other answer"
    );
    assert_eq!(style.border_color_focused, tint(colors.primary, 255));

    // Controls: the two answers that are not this kit's.
    assert_ne!(
        style.border_width_focused, 2.0,
        "control: Material thickens its rim from 1 to 2 on focus"
    );
    assert_ne!(
        style.focus_ring, None,
        "control: Material and Cupertino both draw no ring at all"
    );
    assert_ne!(
        ring.color,
        tint(colors.primary, 120),
        "control: Cupertino's answer tints the *rim* at 120, it does not ring"
    );
    assert_ne!(
        style.border_color_hovered,
        Some(tint(colors.primary, 120)),
        "control: Cupertino's hovered rim"
    );
}

/// The ring reaching the screen: **two** strokes where the other two kits draw
/// one, the ring first and the rim second, because `TextField::paint` strokes
/// the ring outside the bounds and the rim inside them.
///
/// The path bounds are the load-bearing assertion. The ring is stroked around a
/// box grown by its own width and then inset by half of that, so its centreline
/// is one point outside every edge: a ring drawn *inside* the layout box would be
/// clipped and never seen at all, which is the failure mode this test is really
/// about.
#[test]
fn focusing_strokes_the_ring_and_the_rim_and_at_rest_only_the_rim() {
    let binding = theme();
    let colors = binding.colors();

    let mut focused = FluentTextField::new()
        .with_theme(theme())
        .value("hi")
        .view();
    focused.set_focused(true);
    let (ops, snapshot) = paint(&focused);

    let paths = strokes(&ops);
    assert_eq!(
        paths.len(),
        2,
        "the ring and the rim, in that order:\n{snapshot}"
    );
    assert_eq!(paths[0].0, 2.0, "the ring is v1's two points:\n{snapshot}");
    assert_eq!(
        paths[1].0, 1.0,
        "and the rim underneath keeps its one:\n{snapshot}"
    );
    assert_eq!(paths[0].1, tint(colors.primary, 255));
    assert_eq!(
        paths[1].1,
        tint(colors.primary, 255),
        "both are `primary` while focused:\n{snapshot}"
    );
    assert!(
        snapshot.contains("Path (-1.0, -1.0, 241.0, 49.0)"),
        "the ring's centreline is half a width outside every edge, which is the \
         only place a ring is visible at all:\n{snapshot}"
    );

    let idle = FluentTextField::new()
        .with_theme(theme())
        .value("hi")
        .view();
    let (ops, snapshot) = paint(&idle);
    assert_eq!(
        strokes(&ops).len(),
        1,
        "an unfocused field draws its rim and no ring:\n{snapshot}"
    );
    assert_eq!(strokes(&ops)[0].0, 1.0);
    assert_eq!(strokes(&ops)[0].1, tint(colors.outline, 255));
}

/// The other half of the tripwire: hover changes nothing about the rim. It does
/// change the *fill*, because the engine's `background_color_hovered` is
/// inherited rather than themed -- see `the_interaction_fills_are_inherited_because_v1_named_none`
/// -- so this assertion is about the stroke and nothing else.
#[test]
fn hovering_leaves_this_kits_rim_alone() {
    let binding = theme();
    let colors = binding.colors();
    let mut hovered = FluentTextField::new()
        .with_theme(theme())
        .value("hi")
        .view();
    let id = allocate(&hovered);
    hover(&mut hovered, id);
    let (ops, snapshot) = paint(&hovered);

    assert_eq!(
        strokes(&ops)[0].1,
        tint(colors.outline, 255),
        "the resting rim:\n{snapshot}"
    );
    assert_ne!(
        strokes(&ops)[0].1,
        tint(colors.primary, 120),
        "control: Cupertino tints its rim on hover, and this kit must not"
    );
    assert_eq!(
        strokes(&ops).len(),
        1,
        "and a hover opens no ring either:\n{snapshot}"
    );

    leave(&mut hovered, id);
    let (ops, snapshot) = paint(&hovered);
    assert_eq!(
        strokes(&ops)[0].1,
        tint(colors.outline, 255),
        "leaving changes nothing about this kit's rim either:\n{snapshot}"
    );
}

// --- the kit's own theme binding ---------------------------------------------

/// Material's `new()` follows the app-wide theme signal; Cupertino's and
/// Fluent's bind a fixed `MultiTheme`, as v1 did. Asserted through `theme()`,
/// which hands in an explicit binding either way, so this is about the default
/// constructor rather than about `with_theme`.
#[test]
fn the_theme_binding_is_reachable_on_both_constructors() {
    let kit = FluentTextField::new();
    let _ = kit.theme();
    assert_eq!(kit.props().placeholder, "", "and the props start empty");
}
