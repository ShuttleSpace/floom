//! Fluent's checkbox as a kit crate: the token translation, plus the numbers
//! that are this kit's own rather than the tokens'.
//!
//! This crate has never been compiled, which is how three kits came to disagree
//! with themselves about a five-layer control for this long. So the point of this
//! file is not to exercise the engine's `Checkbox` -- that is what
//! `floom-ui-composites`' own checkbox tests are for -- but to pin the numbers
//! *this kit wrote*, in the places where a plausible-looking edit would change a
//! pixel.
//!
//! ## The dead 97, and why it is asserted against
//!
//! v1 had a `resolved_style` that reported the disabled rim as `on_surface` at
//! **97**, and a `view()` that painted `on_surface` at **38**. Nothing ever read
//! the struct -- every kit resolved its colours inline -- so 97 is a number that
//! was never once on a screen. It is named here and asserted *against*, so a
//! later reader who finds it in git history knows which one was painted and the
//! trap cannot re-arm itself.
//!
//! ## What this kit's identity actually is
//!
//! Every colour of the indicator, the mark and the label is shared by all three
//! kits; what is this kit's alone is a state layer **four points of padding wider
//! than the indicator** (tinted `on_surface`@8), gated on hover alone and suppressed while
//! disabled -- and a 2px `primary` focus ring, where Material answers focus with
//! its layer instead.
//!
//! ## The tripwires
//!
//! These tests would fail if the kit stopped being this kit:
//! * `the_layer_is_the_indicator_plus_four_points_of_padding` -- Material's is a
//!   flat 40.
//! * `the_layer_is_on_surface_at_8` -- the one number that separates this
//!   kit from its sibling.
//! * `only_the_hover_gate_is_open_and_the_layer_is_suppressed_while_disabled` --
//!   Material opens all three and suppresses none.
//! * `the_focus_affordance_is_a_ring_around_the_whole_control` -- Material draws
//!   no ring.
//! They are why the three kits stay three crates rather than becoming one.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{Color, ElementId, Point, Rect, Widget};
use floom_types::PointerEvent;
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{Checkbox, CheckboxStyle};
use floom_widgets::testkit::{dispatch, AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::text_width_at;
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{CheckboxTokens, ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_checkbox::FluentCheckbox;
use floom_widget_fluent_icon_paths::fluent_icon_path;

/// The checkmark's path, from this kit's own icon crate. v1 handed
/// `fluent_icon_path("check")` to an `svg` view; 2.0 parses the same `d` attribute into a
/// `VectorIcon`. Read at call time, because the icon crate's accessor is not
/// `const`.
fn check_path() -> &'static str {
    fluent_icon_path("check")
}

/// The 24x24 view box the icon set is authored in.
const CHECK_VIEW_BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

/// The box the checkbox is laid out in: wide enough for the indicator and its
/// label, tall enough for the indicator to be centred in it.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

/// v1's disabled ink: `on_surface` at 38. Written out so the assertion can name
/// the number rather than only that it is not 97.
const DISABLED_ALPHA: u8 = 38;
/// v1's disabled fill: `on_surface` at 12. A different number from the ink, which
/// is why `CheckboxStyle` has a disabled field per layer rather than one
/// `disabled_color`.
const DISABLED_CONTAINER_ALPHA: u8 = 12;

/// What all three v1 `resolved_style` functions claimed for the disabled rim, and
/// what none of them ever painted: nothing read those structs, and every `view()`
/// resolved its colours inline. If a number here ever becomes 97, this crate has
/// started trusting a dead struct.
const V1_DEAD_ALPHA: u8 = 97;

/// The mark's edge as a fraction of the indicator. v1 sized its icon view
/// `size * 0.7` on both axes.
const MARK_SCALE: f32 = 0.7;
/// v1 wrote `gap(8.0)` between the indicator and the label; the engine's shared
/// `INDICATOR_LABEL_GAP` is 6.0. A kit that does not say so silently loses two
/// points, which no screenshot comparison catches because the label is still
/// there, just closer.
const V1_LABEL_GAP: f32 = 8.0;
/// What the engine would have used, pinned so the two cannot be confused.
const ENGINE_LABEL_GAP: f32 = 6.0;

/// v1's `let pad = 4.0`. Written out here so the tripwire can say what the layer
/// is padded by rather than only that it is not 40.
const LAYER_PADDING: f32 = 4.0;

/// The layer's alpha. `on_surface` at 8 -- the faintest of the three, and the one number
/// that separates this kit from Cupertino's `primary` at 10.
const LAYER_ALPHA: u8 = 8;

/// What Material writes instead: a flat 40, not `size + 8`.
const MATERIAL_LAYER_SIZE: f32 = 40.0;

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Fluent)
}

/// Every style is read through an explicit theme rather than the app-wide signal,
/// so these tests cannot be moved by another test's global state.
fn style() -> CheckboxStyle {
    FluentCheckbox::new("A")
        .with_theme(theme())
        .resolved_style()
}

fn disabled_style() -> CheckboxStyle {
    FluentCheckbox::new("A")
        .with_theme(theme())
        .disabled(true)
        .resolved_style()
}

fn checkbox_tokens() -> CheckboxTokens {
    match theme().component(ComponentId::Checkbox) {
        ComponentTokens::Checkbox(tokens) => tokens,
        _ => unreachable!("ComponentId::Checkbox must resolve to CheckboxTokens"),
    }
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Paint into a recorder and hand back both the ops and the snapshot text, so a
/// failing assertion can say what was actually drawn.
fn paint(checkbox: &Checkbox, bounds: Rect) -> (Vec<PaintOp>, String) {
    let id = allocate(checkbox);
    let mut recorder = SceneRecorder::new(bounds);
    checkbox.paint(&mut recorder, id);
    recorder.finish();
    (recorder.ops().to_vec(), recorder.snapshot())
}

/// Every filled rounded rect in the recorded scene, in the order `paint` drew
/// them: the state layer, then the indicator's fill.
///
/// The rim is not among them -- the engine draws it as a stroked path -- and
/// neither is the mark, which is a filled `Path`.
fn fills(ops: &[PaintOp]) -> Vec<(Rect, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::RoundedRect { bounds, color, .. } => Some((*bounds, *color)),
            _ => None,
        })
        .collect()
}

/// Only the fills that are the indicator itself, which is how a test tells the
/// state layer apart from the box it sits behind: the two are concentric and the
/// layer is the wider of them.
fn box_fills(ops: &[PaintOp]) -> Vec<(Rect, Color)> {
    let side = f64::from(style().size);
    fills(ops)
        .into_iter()
        .filter(|(b, _)| (b.width() - side).abs() < 1e-9)
        .collect()
}

/// Only the fills that are the state layer.
fn layer_fills(ops: &[PaintOp]) -> Vec<(Rect, Color)> {
    let side = f64::from(style().layer.size);
    fills(ops)
        .into_iter()
        .filter(|(b, _)| (b.width() - side).abs() < 1e-9)
        .collect()
}

/// The rim as `(stroke width, ink)`.
fn rim(ops: &[PaintOp]) -> Option<(f64, Color)> {
    ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((width, color)),
            ..
        } => Some((*width, *color)),
        _ => None,
    })
}

/// The checkmark as `(bounds, ink)`. Filled, so it is the one `Path` with a fill
/// and no stroke.
fn mark(ops: &[PaintOp]) -> Option<(Rect, Color)> {
    ops.iter().find_map(|op| match op {
        PaintOp::Path {
            bounds,
            fill: Some(color),
            stroke: None,
            ..
        } => Some((*bounds, *color)),
        _ => None,
    })
}

/// The label run as `(text, origin, font size, ink)`.
fn label(ops: &[PaintOp]) -> Option<(String, Point, f32, Color)> {
    ops.iter().find_map(|op| match op {
        PaintOp::Text {
            text,
            origin,
            size,
            color,
            ..
        } => Some((text.clone(), *origin, *size, *color)),
        _ => None,
    })
}

/// The design-system colour at an explicit alpha, in the engine's type. The same
/// conversion the kit makes, so a test states a colour and not a type.
fn tint(color: floom_core_theme::tokens::color::Color, alpha: u8) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

fn alpha_of(c: Color) -> u8 {
    c.a
}

/// The mark's painted bounds when the engine is asked to draw this kit's
/// checkmark at `scale`.
///
/// Built on the engine directly rather than through the kit, because the
/// glyph does not fill its own 24x24 view box -- the path spans about 17.6 of
/// the 24 units -- so an absolute width assertion would be a fact about the
/// glyph and not about `mark_scale`. Two of these at different scales and the
/// ratio is the scale, whatever the glyph is.
fn mark_bounds_at(scale: f32) -> Rect {
    let mut scaled = style();
    scaled.mark_scale = scale;
    let checkbox = Checkbox::new("A")
        .checked(true)
        .style(scaled)
        .mark(VectorIcon::from_path_data(check_path(), CHECK_VIEW_BOX));
    let (ops, snapshot) = paint(&checkbox, ROW);
    mark(&ops)
        .unwrap_or_else(|| panic!("a mark was filled:\n{snapshot}"))
        .0
}

/// A `side`-wide box centred on `centre`.
fn centred(centre: Point, side: f64) -> Rect {
    let half = side / 2.0;
    Rect {
        x0: centre.x - half,
        y0: centre.y - half,
        x1: centre.x + half,
        y1: centre.y + half,
    }
}

/// Whether `inner` lies within `outer`, to a rounding tolerance.
fn contains(outer: Rect, inner: Rect) -> bool {
    let eps = 1e-6;
    inner.x0 >= outer.x0 - eps
        && inner.y0 >= outer.y0 - eps
        && inner.x1 <= outer.x1 + eps
        && inner.y1 <= outer.y1 + eps
}

fn hover(checkbox: &mut Checkbox, id: ElementId) {
    dispatch(
        checkbox,
        id,
        ROW,
        PointerEvent::hover(Point::new(8.0, 16.0)).into(),
    );
}

/// A full press-and-release, which is how the engine sees an activation.
fn click(checkbox: &mut Checkbox, id: ElementId) {
    let at = Point::new(8.0, 16.0);
    dispatch(checkbox, id, ROW, PointerEvent::primary_down(at).into());
    dispatch(checkbox, id, ROW, PointerEvent::primary_up(at).into());
}

// --- the indicator ---------------------------------------------------------

/// The indicator is `CheckboxTokens::size` across with `CheckboxTokens`' own
/// corner radius, and a 2px rim the engine never drew.
///
/// The rim is pinned because it is the number that says *checkbox*: the engine's
/// default is `0.0`, which is a bare fill and reads as a third state rather than
/// as an outline.
#[test]
fn the_indicator_is_the_checkbox_tokens_with_a_two_point_rim() {
    let tokens = checkbox_tokens();
    let style = style();
    assert_eq!(style.size, tokens.size);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.border_width, 2.0, "v1 wrote `.border(2.0)`");
    assert_eq!(
        CheckboxStyle::default().border_width,
        0.0,
        "control: the engine drew no rim at all, so `2.0` here is this kit's and \
         not an inherited default"
    );
    assert_ne!(
        style.size,
        CheckboxStyle::default().size,
        "control: the engine's default indicator is 16, so the token is what \
         sizes this one"
    );
}

/// The fill is `primary` while checked and fully transparent while unchecked.
///
/// Transparent, not white: an unchecked checkbox shows its parent's background,
/// and a filled unchecked box reads as a third state.
#[test]
fn the_container_is_primary_while_checked_and_transparent_while_not() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(style.container_color, tint(colors.primary, 255));
    assert_eq!(
        alpha_of(style.container_color_unchecked),
        0,
        "transparent -- not white, not a dimmed primary"
    );
    assert_eq!(
        alpha_of(CheckboxStyle::default().container_color_unchecked),
        255,
        "control: the engine's default unchecked fill is opaque white, which is a \
         third state"
    );
}

/// And the transparency really reaches the paint: the engine always fills the
/// box, so an unchecked checkbox's one fill carries no ink at all.
#[test]
fn the_unchecked_container_is_transparent_when_painted() {
    let binding = theme();
    let colors = binding.colors();

    let unchecked = FluentCheckbox::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unchecked, ROW);
    let boxed = box_fills(&ops);
    assert_eq!(
        boxed.len(),
        1,
        "the engine fills the box in every state:\n{snapshot}"
    );
    assert_eq!(alpha_of(boxed[0].1), 0, "with nothing in it:\n{snapshot}");
    assert_eq!(boxed[0].0.width(), f64::from(style().size));

    let checked = FluentCheckbox::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&checked, ROW);
    let boxed = box_fills(&ops);
    assert_eq!(boxed.len(), 1, "\n{snapshot}");
    assert_eq!(
        boxed[0].1,
        tint(colors.primary, 255),
        "a checked box fills `primary`:\n{snapshot}"
    );
}

/// The rim is `primary` while checked and `outline` while unchecked, with no
/// alpha on either.
#[test]
fn the_rim_is_primary_while_checked_and_outline_while_not() {
    let binding = theme();
    let colors = binding.colors();

    let unchecked = FluentCheckbox::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unchecked, ROW);
    let (width, ink) = rim(&ops).expect("a rim was stroked");
    assert_eq!(width, 2.0);
    assert_eq!(
        ink,
        tint(colors.outline, 255),
        "the resting rim:\n{snapshot}"
    );

    let checked = FluentCheckbox::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&checked, ROW);
    let (_, ink) = rim(&ops).expect("a rim was stroked");
    assert_eq!(
        ink,
        tint(colors.primary, 255),
        "the checked rim:\n{snapshot}"
    );
}

// --- the mark --------------------------------------------------------------

/// The mark is `on_primary` while checked and transparent while unchecked -- so
/// it is not drawn at all -- and it is asked for at `size * 0.7`.
///
/// The engine's own default mark is a font glyph at the *full* box height with
/// `mark_scale: 1.0`, which is a visibly different checkmark; both numbers are
/// asserted against it.
#[test]
fn the_mark_is_on_primary_at_seven_tenths_of_the_box() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(style.mark_color, tint(colors.on_primary, 255));
    assert_eq!(
        alpha_of(style.mark_color_unchecked),
        0,
        "an unchecked mark is a third state, not a faint one"
    );
    assert_eq!(style.mark_scale, MARK_SCALE);
    assert_eq!(style.mark_scale, 0.7);
    assert_eq!(
        CheckboxStyle::default().mark_scale,
        1.0,
        "control: the engine's glyph is drawn at the full box height"
    );

    let checked = FluentCheckbox::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&checked, ROW);
    let (bounds, ink) = mark(&ops).expect("a mark was filled");
    assert_eq!(ink, tint(colors.on_primary, 255), "\n{snapshot}");

    // Asked for at `size * 0.7`, in a box centred on the indicator. The glyph
    // does not fill its own 24x24 view box -- the path spans about 17.6 of the
    // 24 units, and not symmetrically -- so the *painted* bounds sit inside that
    // box rather than filling it, and containment is the claim to make.
    let requested = mark_bounds_at(MARK_SCALE);
    assert_eq!(
        bounds, requested,
        "the kit's own mark is the one the engine draws at 0.7:\n{snapshot}"
    );
    let indicator = checked.box_bounds(ROW);
    let asked_for = centred(indicator.center(), f64::from(style.size * MARK_SCALE));
    assert!(
        contains(asked_for, bounds),
        "the mark is inside the box it was asked for, because an icon is centred \
         and preserves aspect ratio:\n{snapshot} -- asked for {asked_for:?}"
    );
    assert!(
        (asked_for.width() - f64::from(checkbox_tokens().size) * f64::from(MARK_SCALE)).abs()
            < 1e-3,
        "18 * 0.7, so `mark_scale` is read off the token rather than hardcoded: \
         {asked_for:?}"
    );
    assert!(
        (asked_for.center().x - indicator.center().x).abs() < 1e-9,
        "and that box is centred on the indicator:\n{snapshot}"
    );
}

/// **`mark_scale` really is the scale, proven from the paint rather than read off
/// the struct.** The glyph does not fill its 24x24 view box, so an absolute width
/// says nothing; two scales and a ratio does, whatever the glyph is.
#[test]
fn the_scale_is_a_scale() {
    let at_one = mark_bounds_at(1.0).width();
    let at_kit = mark_bounds_at(MARK_SCALE).width();
    assert!(
        (at_kit / at_one - f64::from(MARK_SCALE)).abs() < 1e-6,
        "{at_kit} / {at_one} is the scale, and v1 wrote 0.7"
    );
    assert!(
        at_kit < at_one,
        "a fractional mark is smaller than a full-height one"
    );
}

/// An unchecked checkbox draws no mark at all, which is the stronger claim:
/// `mark_color_unchecked` is transparent so a kit need not suppress the draw
/// itself, but the control only fills a mark while checked.
#[test]
fn an_unchecked_box_draws_no_mark() {
    let unchecked = FluentCheckbox::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unchecked, ROW);
    assert!(
        mark(&ops).is_none(),
        "an unchecked box draws a rim and a label and no mark:\n{snapshot}"
    );
}

// --- the label -------------------------------------------------------------

/// The label is `on_surface` at `typography().body.medium.font_size`, eight
/// points to the right of the indicator.
#[test]
fn the_label_is_on_surface_at_the_body_size_eight_points_on() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(
        style.label_font_size,
        binding.typography().body.medium.font_size
    );
    assert_eq!(style.label_color, tint(colors.on_surface, 255));
    assert_eq!(style.label_gap, V1_LABEL_GAP, "v1 wrote `gap(8.0)`");
    assert_ne!(
        style.label_gap, ENGINE_LABEL_GAP,
        "if these are equal the two constants have been conflated and this \
         test no longer proves anything about the gap"
    );
    assert_eq!(
        CheckboxStyle::default().label_gap,
        ENGINE_LABEL_GAP,
        "control: the engine's shared gap is 6, so 8 is this kit's"
    );

    let checkbox = FluentCheckbox::new("Ship it").with_theme(theme()).view();
    let (ops, snapshot) = paint(&checkbox, ROW);
    let (text, origin, size, ink) = label(&ops).expect("a label run");
    assert_eq!(text, "Ship it", "\n{snapshot}");
    assert_eq!(size, style.label_font_size);
    assert_eq!(ink, tint(colors.on_surface, 255), "\n{snapshot}");

    // And it sits the gap to the right of the indicator, not at the row's edge.
    let indicator = checkbox.box_bounds(ROW);
    assert_eq!(
        origin.x - indicator.x1,
        f64::from(style.label_gap),
        "\n{snapshot}"
    );
}

// --- the disabled palette -------------------------------------------------

/// **38, not the 97 that v1's dead `resolved_style` claimed.** Every one of the
/// three kits' `resolved_style` functions wrote `on_surface` at 97 and none of
/// them was ever called; every one of their `view()` bodies wrote 38 inline and
/// that is what a screen showed.
///
/// The fill is a different number again -- 12 -- which is why `CheckboxStyle`
/// carries a disabled colour per layer rather than one `disabled_color`.
#[test]
fn the_disabled_palette_is_on_surface_at_38_and_12_and_not_97() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(
        style.border_color_disabled,
        tint(colors.on_surface, DISABLED_ALPHA),
        "the disabled rim"
    );
    assert_eq!(
        style.mark_color_disabled,
        tint(colors.on_surface, DISABLED_ALPHA),
        "and the disabled mark, which v1 wrote from the same expression"
    );
    assert_eq!(
        style.label_color_disabled,
        tint(colors.on_surface, DISABLED_ALPHA),
        "and the disabled label, which v1 wrote a third time"
    );
    assert_eq!(
        style.container_color_disabled,
        tint(colors.on_surface, DISABLED_CONTAINER_ALPHA),
        "and a fill at 12, which is a different number again"
    );
    assert_ne!(
        DISABLED_CONTAINER_ALPHA, DISABLED_ALPHA,
        "control: two different numbers, or one field would do"
    );

    assert_ne!(
        style.border_color_disabled.a, V1_DEAD_ALPHA,
        "control: 97 is what the dead `resolved_style` claimed and what v1 never painted"
    );
    assert_ne!(
        style.mark_color_disabled.a, V1_DEAD_ALPHA,
        "ditto for the mark"
    );
    assert_ne!(
        style.label_color_disabled.a, V1_DEAD_ALPHA,
        "ditto for the label"
    );
}

/// And the disabled colours really reach all four layers when mounted, which is
/// the claim the per-layer fields make.
///
/// Disabled is resolved *first* in the engine and covers both values, so a
/// disabled *checked* box is `on_surface`@12 -- not `primary` dimmed. That is
/// v1's own behaviour and the reason the fields exist separately.
#[test]
fn the_disabled_colour_reaches_every_layer() {
    let binding = theme();
    let colors = binding.colors();
    let dimmed = tint(colors.on_surface, DISABLED_ALPHA);
    let dim_fill = tint(colors.on_surface, DISABLED_CONTAINER_ALPHA);

    let checkbox = FluentCheckbox::new("A")
        .with_theme(theme())
        .checked(true)
        .disabled(true)
        .view();
    let (ops, snapshot) = paint(&checkbox, ROW);
    assert_eq!(
        box_fills(&ops)[0].1,
        dim_fill,
        "a disabled checked box is `on_surface`@12, not `primary` dimmed:\n{snapshot}"
    );
    assert_eq!(rim(&ops).expect("a rim").1, dimmed, "the rim:\n{snapshot}");
    assert_eq!(
        mark(&ops).expect("a mark").1,
        dimmed,
        "the mark:\n{snapshot}"
    );
    assert_eq!(
        label(&ops).expect("a label").3,
        dimmed,
        "the label:\n{snapshot}"
    );
}

/// An enabled box says `on_surface`@38 is *only* for the disabled one: none of
/// the enabled colours carry it.
#[test]
fn an_enabled_box_carries_no_disabled_alpha() {
    let style = style();
    for (name, c) in [
        ("container", style.container_color),
        ("rim", style.border_color),
        ("rim unchecked", style.border_color_unchecked),
        ("mark", style.mark_color),
        ("label", style.label_color),
    ] {
        assert_ne!(
            alpha_of(c),
            DISABLED_ALPHA,
            "{name} must not be the disabled ink while enabled"
        );
    }
    assert_eq!(alpha_of(style.border_color), 255, "undimmed");
    assert_eq!(alpha_of(style.mark_color), 255, "undimmed");
}

// --- mounting -------------------------------------------------------------

/// The props cross into the widget, and both of the two places `disabled` lives
/// are set from the same builder flag.
#[test]
fn mounting_carries_the_props_across() {
    let checkbox = FluentCheckbox::new("Ship it")
        .with_theme(theme())
        .checked(true)
        .disabled(true)
        .view();
    assert!(checkbox.is_checked(), "the checked prop reached the widget");
    assert!(checkbox.interaction().disabled, "and so did disabled");
}

/// **`on_toggle` is `Fn(bool)` and reports the value the engine moved to**, which
/// is what v1's `on_change` did too -- so the callback crosses unchanged. Unlike
/// the radio ports there is no `!was` reconstruction: the engine flips the value
/// and fires, and a checkbox really does report both directions.
#[test]
fn the_callback_reports_both_directions() {
    let seen = Rc::new(Cell::new(Vec::<bool>::new()));
    let sink = seen.clone();
    let mut checkbox = FluentCheckbox::new("A")
        .with_theme(theme())
        .on_change(move |value| {
            let mut v = sink.take();
            v.push(value);
            sink.set(v);
        })
        .view();
    let id = allocate(&checkbox);

    click(&mut checkbox, id);
    assert!(checkbox.is_checked(), "the engine owns the value now");
    assert_eq!(seen.take(), vec![true], "and reports the new one");

    click(&mut checkbox, id);
    assert!(!checkbox.is_checked(), "and back");
    assert_eq!(seen.take(), vec![false]);
}

/// The layout reserves the indicator, this kit's gap and the label at this kit's
/// font size -- the three numbers a host would otherwise take from the engine's
/// defaults and clip a fifth of the control with.
#[test]
fn the_layout_reserves_the_indicator_the_gap_and_the_label() {
    let style = style();
    let checkbox = FluentCheckbox::new("Ship it").with_theme(theme()).view();
    let request = checkbox
        .layout_request()
        .expect("a labelled checkbox asks for a size");

    let expected_width =
        style.size + style.label_gap + text_width_at("Ship it", style.label_font_size);
    assert_eq!(
        request.width.expect("a width").px().expect("in px"),
        expected_width,
        "the indicator, this kit's gap, then the label at this kit's size"
    );
    assert!(
        request.height.expect("a height").px().expect("in px") >= style.size,
        "the height fits the indicator"
    );
    assert!(
        style.size != CheckboxStyle::default().size,
        "control: the engine's default reserves 16, so this assertion is not vacuous"
    );
}

/// **The three design systems still resolve identical tokens, so a kit's identity
/// lives in its own constants.** `MultiTheme::new_for` calls `activate`, which the
/// theme crate documents as a no-op placeholder, and the variant is picked at read
/// time from a `thread_local` that every test thread starts at Material -- so
/// Material, Cupertino and Fluent hand back one identical set of colours and one
/// `CheckboxTokens`. Every colour assertion above is therefore on a number *this
/// kit wrote*, never on a token value -- a token assertion would pass for all
/// three kits whatever any of them did. This is the tripwire for the day per-DS
/// tokens land.
#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let own = theme();
    let material = MultiTheme::new_for(DesignSystem::Material);
    let cupertino = MultiTheme::new_for(DesignSystem::Cupertino);
    let fluent = MultiTheme::new_for(DesignSystem::Fluent);
    let colors = own.colors();

    assert_eq!(material.colors().primary, colors.primary);
    assert_eq!(cupertino.colors().outline, colors.outline);
    assert_eq!(fluent.colors().on_primary, colors.on_primary);

    for (name, other) in [
        ("material", material),
        ("cupertino", cupertino),
        ("fluent", fluent),
    ] {
        let tokens = match other.component(ComponentId::Checkbox) {
            ComponentTokens::Checkbox(tokens) => tokens,
            _ => unreachable!("ComponentId::Checkbox must resolve to CheckboxTokens"),
        };
        assert_eq!(tokens.size, checkbox_tokens().size, "{name} size");
        assert_eq!(
            tokens.border_radius,
            checkbox_tokens().border_radius,
            "{name} radius"
        );
    }
    assert_eq!(checkbox_tokens().size, style().size);
    assert_eq!(checkbox_tokens().border_radius, style().border_radius);
}

/// `resolved_style` does not branch on `disabled` at all -- the engine resolves
/// the state -- so the two styles are the same struct and what differs is which
/// of the layer colours the widget picks. Asserted because it is the reason the
/// port reads `*color_disabled` fields instead of composing one `disabled_color`
/// the way the radio kits do.
#[test]
fn the_disabled_flag_does_not_move_the_style() {
    assert_eq!(
        disabled_style(),
        style(),
        "`CheckboxStyle` names a colour per state; `Checkbox` picks"
    );
}

// --- the state layer -------------------------------------------------------

/// **The tripwire for the three kits converging.** v1 wrote `size + pad * 2.0`
/// here and `.size(40.0, 40.0)` in Material. A "harmonise the kits" edit that
/// reached for the absolute form would land here at `MATERIAL_LAYER_SIZE`, and one
/// that reached for the relative one would land in Material's crate at `size + 8`.
#[test]
fn the_layer_is_the_indicator_plus_four_points_of_padding() {
    let style = style();
    assert_eq!(
        style.layer.size,
        style.size + LAYER_PADDING * 2.0,
        "four points of padding on each side"
    );
    assert_eq!(
        style.layer.shape,
        LayerShape::Box {
            padding: LAYER_PADDING
        }
    );
    assert_ne!(
        style.layer.shape,
        LayerShape::Circle,
        "Material's layer is a circle; this one is a rounded box"
    );
    assert_ne!(
        style.layer.size, MATERIAL_LAYER_SIZE,
        "this kit's is relative; Material's is a flat 40"
    );
    assert!(
        style.layer.size < MATERIAL_LAYER_SIZE,
        "control: the two forms really do differ at today's token, or this \
         assertion proves nothing"
    );
}

/// The layer is `on_surface` at 8, and has no second tint: v1 read one
/// alpha, from `hover` alone.
#[test]
fn the_layer_is_on_surface_at_8() {
    let binding = theme();
    let colors = binding.colors();
    let layer = style().layer;

    assert_eq!(layer.color, tint(colors.on_surface, LAYER_ALPHA));
    assert_eq!(alpha_of(layer.color), 8);
    assert_ne!(
        layer.color.r, colors.primary.r,
        "control: this kit tints the layer with `on_surface`, where its sibling uses \
         `primary`"
    );
    assert_eq!(
        layer.color_pressed, None,
        "`None` means the layer keeps `color` while pressed; Material's `Some` is \
         the other answer"
    );
}

/// **Only the hover gate is open.** v1's condition was `hover && !disabled`, and
/// nothing else ever turned the layer on -- no press, and no focus (focus is the
/// ring's, below). Material opens all three and suppresses none.
#[test]
fn only_the_hover_gate_is_open_and_the_layer_is_suppressed_while_disabled() {
    let layer = style().layer;
    assert!(layer.on_hover, "v1 drew it while hovered");
    assert!(
        !layer.on_press,
        "v1 had no pressed state for the indicator or the layer"
    );
    assert!(
        !layer.on_focus,
        "focus is the ring's answer on this kit, and the two must not both fire"
    );
    assert!(
        layer.suppressed_when_disabled,
        "v1 wrote `!hover || disabled` into its `apply_if`"
    );
}

/// Hovering paints the layer. Unlike Material's, a press does not deepen it --
/// the press gate is closed and there is no second tint to deepen it with.
#[test]
fn hovering_opens_the_layer_and_pressing_keeps_the_resting_tint() {
    let style = style();
    let at = Point::new(8.0, 16.0);

    let mut hovered = FluentCheckbox::new("A").with_theme(theme()).view();
    let id = allocate(&hovered);
    hover(&mut hovered, id);
    let (ops, snapshot) = paint(&hovered, ROW);
    let layers = layer_fills(&ops);
    assert_eq!(
        layers.len(),
        1,
        "the layer is the only new fill:\n{snapshot}"
    );
    assert_eq!(layers[0].1, style.layer.color);
    assert_eq!(layers[0].0.width(), f64::from(style.layer.size));

    let mut pressed = FluentCheckbox::new("A").with_theme(theme()).view();
    let id = allocate(&pressed);
    dispatch(&mut pressed, id, ROW, PointerEvent::primary_down(at).into());
    let (ops, snapshot) = paint(&pressed, ROW);
    assert_eq!(
        layer_fills(&ops)[0].1,
        style.layer.color,
        "a press does not deepen this kit's layer:\n{snapshot}"
    );
    assert_eq!(style.layer.color_pressed, None);
}

/// **The other half of this kit's layer, pinned.** Material's is drawn while
/// disabled; this one is not.
///
/// Focus rather than hover, because the engine refuses to set `hovered` on a
/// disabled widget at all -- `event` returns before the match -- so focus is the
/// only gate a disabled checkbox can be put through, and it is closed here.
#[test]
fn the_layer_is_not_drawn_while_disabled() {
    let style = style();
    assert!(style.layer.suppressed_when_disabled);
    assert!(!style.layer.on_focus, "and focus is not an opening either");

    let mut disabled = FluentCheckbox::new("A")
        .with_theme(theme())
        .disabled(true)
        .view();
    disabled.set_focused(true);
    let (ops, snapshot) = paint(&disabled, ROW);
    assert_eq!(
        layer_fills(&ops).len(),
        0,
        "a disabled box paints a fill, a rim, a mark and a label and no layer:\n{snapshot}"
    );
}

/// And focus is not an answer on its own: an enabled, focused checkbox is still
/// bare of a layer, which is the other half of `on_focus: false`.
#[test]
fn focus_does_not_open_the_layer() {
    let mut focused = FluentCheckbox::new("A").with_theme(theme()).view();
    focused.set_focused(true);
    let (ops, snapshot) = paint(&focused, ROW);
    assert_eq!(
        layer_fills(&ops).len(),
        0,
        "focus opens no gate on this kit's layer:\n{snapshot}"
    );
}

// --- focus -----------------------------------------------------------------

/// **This kit answers focus with a ring, where Material answers with its layer.**
/// v1 ended its style closure with
/// `.focus(|s| s.outline(2.0).outline_color(colors().primary))`, so a focused
/// checkbox drew a 2px `primary` ring around the whole control -- the indicator
/// and the label together, because the closure was on the container holding both.
#[test]
fn the_focus_affordance_is_a_ring_around_the_whole_control() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    let ring = style
        .focus_ring
        .expect("v1 drew `outline(2.0)` in `primary` and this is that ring");
    assert_eq!(ring.width, 2.0, "v1 wrote `outline(2.0)`");
    assert_eq!(ring.color, tint(colors.primary, 255), "in `primary`");
    assert_eq!(
        ring.gap, 0.0,
        "flush against the control: Compose's `outline()` has no inset"
    );
    assert_eq!(
        ring.radius, None,
        "no pinned radius, so the ring follows whatever the control's own corners are"
    );

    assert!(
        !style.layer.on_focus,
        "and the layer does not also answer focus, or a focused checkbox would \
         carry two focus affordances at once"
    );
    // Fluent's layer is `on_surface`, so the two answers differ in rgb as well
    // as in alpha -- a stronger separation than Cupertino's, where both are
    // `primary` and only the alpha tells them apart.
    assert_ne!(
        ring.color.r, colors.on_surface.r,
        "control: this kit's *layer* is `on_surface`@{LAYER_ALPHA}, which is a different \
         answer and must not be confused with this one"
    );
    assert_ne!(
        ring.color, style.layer.color,
        "and not the same colour either"
    );
}

// --- the design system ----------------------------------------------------

/// v1's style struct carried `design_system: "fluent"` as a field; the engine's
/// `CheckboxStyle` has no such field, because the engine is not a design system,
/// so the string moved to a const. Asserted here because it is the one part of the
/// v1 struct that survived the port rather than being re-derived.
#[test]
fn the_design_system_const_is_what_v1_put_on_the_struct() {
    assert_eq!(floom_widget_fluent_checkbox::DESIGN_SYSTEM, "fluent");
}

/// **The tripwire, stated as Material's numbers.** This crate cannot depend on its
/// sibling -- three crates that named each other would have no place to put a
/// shared truth -- so the difference is asserted against the style Material
/// *would* resolve, written out here.
///
/// Both halves have to be true at once for the two to stay two: this kit's layer
/// is `size + 8` and it draws a ring. A "harmonise" edit that reached for the
/// flat 40 would fail the first; one that dropped the ring would fail the second.
#[test]
fn this_kit_does_not_resolve_to_materials_layer_or_focus() {
    let style = style();

    // What Material resolves: a flat 40 circle with a pressed tint, gated on all
    // three and never suppressed.
    let material_layer = StateLayer {
        color: style.layer.color,
        color_pressed: Some(style.layer.color),
        size: 40.0,
        shape: LayerShape::Circle,
        on_hover: true,
        on_press: true,
        on_focus: true,
        suppressed_when_disabled: false,
    };
    assert_ne!(
        style.layer, material_layer,
        "this kit's layer is a padded box with no pressed tint, gated on hover \
         and suppressed while disabled"
    );
    assert_ne!(style.layer.size, material_layer.size);
    assert_ne!(style.layer.shape, material_layer.shape);
    assert_ne!(style.layer.color_pressed, material_layer.color_pressed);
    assert_ne!(
        style.layer.suppressed_when_disabled,
        material_layer.suppressed_when_disabled
    );

    // And focus: a ring here, the layer there.
    assert!(
        style.focus_ring.is_some(),
        "this kit answers focus with a ring"
    );
    assert!(!style.layer.on_focus);
    assert_ne!(
        style.layer.on_focus, material_layer.on_focus,
        "Material answers focus with its layer and draws no ring"
    );
}
