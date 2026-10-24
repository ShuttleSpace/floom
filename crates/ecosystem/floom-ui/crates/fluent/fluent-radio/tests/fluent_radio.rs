//! Fluent's radio as a kit crate: the token translation, plus the two numbers
//! that are this kit's own rather than the tokens' -- a halo four points of
//! padding wider than the ring, and the fact that it is hover-only.
//!
//! These three crates have never been compiled, which is how three kits came to
//! disagree with themselves about a five-layer control for this long. So the
//! point of this file is not to exercise the engine's `Radio` -- that is what
//! `floom-ui-composites`' own radio tests are for -- but to pin the numbers *this
//! kit wrote*, in the places where a plausible-looking edit would change a pixel.
//!
//! ## What this file deliberately does not assert
//!
//! v1 drew a focus outline around the whole control and drew no halo on focus at
//! all. `RadioStyle` cannot express the outline, so there is nothing here to pin
//! for it; the loss is recorded in the module doc rather than asserted, because a
//! kit test asserting an absent field would really be a test of the engine.
//!
//! ## Fluent and Cupertino differ in exactly one number
//!
//! Measured rather than assumed, because "the Fluent radio" is not a thing this
//! crate can check on its own. With the crate name and the design system enum
//! removed, this file and the Cupertino one are identical but for the halo's
//! tint: `on_surface` at 8 here against `primary` at 10 there. Same four points
//! of padding, same `size + 8.0`, same hover-only gate, same suppression while
//! disabled. So the two stay two crates, because the tint is the whole difference
//! and a shared implementation would be a place for it to hide.
//!
//! ## The tripwires
//!
//! Three of these tests would fail if the kit stopped being this kit:
//! `the_halo_is_the_ring_plus_four_points_of_padding`,
//! `a_disabled_radio_is_not_the_enabled_one` and
//! `the_halo_is_not_drawn_while_disabled`. They are why the three kits stay three
//! crates rather than becoming one.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{Color, ElementId, Point, Rect, Widget};
use floom_types::PointerEvent;
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{Radio, RadioStyle};
use floom_widgets::testkit::{dispatch, AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::text_width_at;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, RadioTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_radio::FluentRadio;

/// The box the radio is laid out in: wide enough for the ring and its label, tall
/// enough for the ring to be centred in it.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

/// The gap the engine puts between the ring and the label. It belongs to the
/// engine rather than to this kit, but the width assertion below has to name it,
/// so it is written out rather than left as an unexplained constant.
/// v1 drew `gap(8.0)` here; the engine's shared `INDICATOR_LABEL_GAP` is 6.0.
/// A kit that does not say so silently loses two points, which no screenshot
/// comparison catches because the label is still there, just closer.
const V1_LABEL_GAP: f32 = 8.0;
/// What the engine would have used, pinned so the two cannot be confused.
const ENGINE_LABEL_GAP: f32 = 6.0;

/// v1's `let pad = 4.0`. Written out here so the tripwire can say what the halo
/// is padded by rather than only that it is not 40.
const HALO_PADDING: f32 = 4.0;

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Fluent)
}

/// Every style is read through an explicit theme rather than the app-wide signal,
/// so these tests cannot be moved by another test's global state.
fn style() -> RadioStyle {
    FluentRadio::new("A").with_theme(theme()).resolved_style()
}

fn disabled_style() -> RadioStyle {
    FluentRadio::new("A")
        .with_theme(theme())
        .disabled(true)
        .resolved_style()
}

fn radio_tokens() -> RadioTokens {
    match theme().component(ComponentId::Radio) {
        ComponentTokens::Radio(tokens) => tokens,
        _ => unreachable!("ComponentId::Radio must resolve to RadioTokens"),
    }
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Paint into a recorder and hand back both the ops and the snapshot text, so a
/// failing assertion can say what was actually drawn.
fn paint(radio: &Radio, bounds: Rect) -> (Vec<PaintOp>, String) {
    let id = allocate(radio);
    let mut recorder = SceneRecorder::new(bounds);
    radio.paint(&mut recorder, id);
    recorder.finish();
    (recorder.ops().to_vec(), recorder.snapshot())
}

/// Every filled circle in the recorded scene -- the halo, the interior and the
/// dot, in the order `paint` drew them.
///
/// The rim is not among them: the engine draws it as a stroked path, which is
/// what makes `the_interior_is_transparent` below an exact statement about how
/// many layers a radio has rather than a search for an absence.
fn fills(ops: &[PaintOp]) -> Vec<(Rect, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::RoundedRect { bounds, color, .. } => Some((*bounds, *color)),
            _ => None,
        })
        .collect()
}

/// The ring's rim as `(stroke width, ink)`.
fn rim(ops: &[PaintOp]) -> Option<(f64, Color)> {
    ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((width, color)),
            ..
        } => Some((*width, *color)),
        _ => None,
    })
}

/// The label run as `(text, font size, ink)`.
fn label(ops: &[PaintOp]) -> Option<(String, f32, Color)> {
    ops.iter().find_map(|op| match op {
        PaintOp::Text {
            text, size, color, ..
        } => Some((text.clone(), *size, *color)),
        _ => None,
    })
}

fn alpha_of(c: Color) -> u8 {
    c.a
}

fn hover(radio: &mut Radio, id: ElementId) {
    dispatch(
        radio,
        id,
        ROW,
        PointerEvent::hover(Point::new(8.0, 16.0)).into(),
    );
}

/// A full press-and-release, which is how the engine sees an activation.
fn click(radio: &mut Radio, id: ElementId) {
    let at = Point::new(8.0, 16.0);
    dispatch(radio, id, ROW, PointerEvent::primary_down(at).into());
    dispatch(radio, id, ROW, PointerEvent::primary_up(at).into());
}

// --- the ring and the dot ---------------------------------------------------

/// The ring is `RadioTokens::size` and its rim `RadioTokens::border_width`.
///
/// Both are pinned to literals as well, because the border width is the number
/// that says *ring*: the engine's default is `size / 2.0`, which fills the whole
/// ring and turns it back into the disc the widget painted before the style
/// existed.
#[test]
fn the_ring_and_its_rim_are_the_radio_tokens() {
    let tokens = radio_tokens();
    let style = style();
    assert_eq!(style.size, tokens.size);
    assert_eq!(style.size, 20.0);
    assert_eq!(style.border_width, tokens.border_width);
    assert_eq!(style.border_width, 2.0, "a rim, not a filled disc");
    assert_ne!(
        style.border_width,
        style.size / 2.0,
        "control: the engine's default rim fills the ring"
    );
}

/// The rim is `outline` while unselected and `primary` while selected, and the dot
/// is `primary` either way.
///
/// v1 branched on `checked` inside the ring's own style closure, so these were one
/// colour in one place; the style has them as two fields, which is what lets a
/// test say which is which.
#[test]
fn the_unselected_ring_is_outline_and_the_selected_ring_is_primary() {
    let binding = theme();
    let colors = binding.colors();

    let unselected = FluentRadio::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unselected, ROW);
    let (_, ink) = rim(&ops).expect("a rim was stroked");
    assert_eq!(ink.r, colors.outline.r, "the resting rim:\n{snapshot}");
    assert_eq!(ink.g, colors.outline.g);
    assert_eq!(ink.b, colors.outline.b);
    assert_eq!(alpha_of(ink), 255, "undimmed");

    let selected = FluentRadio::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&selected, ROW);
    let (_, ink) = rim(&ops).expect("a rim was stroked");
    assert_eq!(ink.r, colors.primary.r, "the selected rim:\n{snapshot}");
    assert_eq!(alpha_of(ink), 255);
}

/// The dot is `size * 0.5` across, which is the number all three v1 kits wrote.
///
/// `RadioStyle` has no dot field -- the engine derives it -- so this is asserted
/// from the painted scene, where the dot is the only filled circle a selected,
/// idle radio has.
#[test]
fn the_dot_is_half_the_ring() {
    let style = style();
    let selected = FluentRadio::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&selected, ROW);
    let fills = fills(&ops);
    assert_eq!(
        fills.len(),
        1,
        "the halo is gated and the interior is transparent, so the dot is the only fill:\n{snapshot}"
    );
    let (dot, _) = fills[0];
    assert_eq!(dot.width(), f64::from(style.size) * 0.5, "\n{snapshot}");
    assert_eq!(dot.width(), 10.0);
    assert_eq!(dot.height(), dot.width(), "the dot is a circle");

    // And it is centred on the ring rather than sitting at its corner.
    let ring = selected.dot_bounds(ROW);
    assert!((dot.x0 + dot.width() / 2.0 - ring.center().x).abs() < 1e-9);
}

/// v1's `circle` was `.border(bw).border_color(..)` with no background, so the
/// middle of every Fluent radio showed whatever its parent was painted with.
///
/// The engine's default fills it white, which would be a white disc inside a
/// transparent ring.
#[test]
fn the_interior_is_transparent() {
    assert!(style().interior_color.is_none());

    let unselected = FluentRadio::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unselected, ROW);
    assert_eq!(
        fills(&ops).len(),
        0,
        "an unpainted interior is the absence of a layer, not a white one:\n{snapshot}"
    );

    assert!(
        RadioStyle::default().interior_color.is_some(),
        "control: the engine's default fills the middle, so `None` here is this kit declining it"
    );
}

// --- the disabled palette ---------------------------------------------------

/// **One `disabled_color`, because all three layers dim to the same colour.**
/// v1 wrote `with_alpha(on_surface, 38)` twice -- for the ring and the label --
/// and the engine's single field covers rim, dot and label at once, which is why
/// `resolved_style` composes instead of branching the way `floom-widget-md-slider`
/// does.
#[test]
fn a_disabled_radio_is_on_surface_at_38() {
    let binding = theme();
    let colors = binding.colors();
    let dimmed = Color {
        r: colors.on_surface.r,
        g: colors.on_surface.g,
        b: colors.on_surface.b,
        a: 38,
    };
    assert_eq!(disabled_style().disabled_color, Some(dimmed));
    assert_eq!(
        style().disabled_color,
        None,
        "an enabled radio names no disabled colour -- `None` is the engine's own half-alpha dim, not `Some(undimmed)`"
    );
}

/// And the one colour really does reach all three layers when mounted, which is
/// the claim the single field makes.
#[test]
fn the_disabled_colour_reaches_the_rim_the_dot_and_the_label() {
    let disabled = disabled_style().disabled_color.expect("a dim");
    let radio = FluentRadio::new("A")
        .with_theme(theme())
        .checked(true)
        .disabled(true)
        .view();
    let (ops, snapshot) = paint(&radio, ROW);
    assert_eq!(
        rim(&ops).expect("a rim").1,
        disabled,
        "the rim:\n{snapshot}"
    );
    assert_eq!(fills(&ops)[0].1, disabled, "the dot:\n{snapshot}");
    assert_eq!(
        label(&ops).expect("a label").2,
        disabled,
        "the label:\n{snapshot}"
    );
}

/// **This is why the kit composes the disabled colour.** `RadioStyle`'s
/// `disabled_color` is the only place a kit can answer "what does disabled look
/// like", so if the composition were dropped the disabled radio would resolve to
/// the enabled one and only this would notice.
#[test]
fn a_disabled_radio_is_not_the_enabled_one() {
    assert_ne!(disabled_style(), style());
}

// --- the label --------------------------------------------------------------

#[test]
fn the_label_is_on_surface_at_the_token_size() {
    let tokens = radio_tokens();
    let style = style();
    let binding = theme();
    let colors = binding.colors();

    assert_eq!(style.label_font_size, tokens.label_font_size);

    let radio = FluentRadio::new("Ship it").with_theme(theme()).view();
    let (ops, snapshot) = paint(&radio, ROW);
    let (text, size, ink) = label(&ops).expect("a label run");
    assert_eq!(text, "Ship it", "\n{snapshot}");
    assert_eq!(size, style.label_font_size);
    assert_eq!(size, tokens.label_font_size);
    assert_eq!(ink.r, colors.on_surface.r, "\n{snapshot}");
    assert_eq!(ink.g, colors.on_surface.g);
    assert_eq!(ink.b, colors.on_surface.b);
    assert_eq!(alpha_of(ink), 255, "undimmed while enabled");
}

// --- the halo ---------------------------------------------------------------

/// The halo is `on_surface` at 8 and has no second tint: v1 read one alpha, from
/// `hover` alone.
///
/// `on_surface` rather than `primary` is the whole difference from Cupertino's
/// `primary` at 10 -- same gates, same padding, same suppression, same everything
/// else -- and it is asserted against the token rather than as a bare number so a
/// rename cannot make the two crates agree by accident.
#[test]
fn the_halo_is_on_surface_at_eight_and_has_no_pressed_tint() {
    let binding = theme();
    let colors = binding.colors();
    let halo = style().halo;

    assert_eq!(halo.color.r, colors.on_surface.r);
    assert_eq!(halo.color.g, colors.on_surface.g);
    assert_eq!(halo.color.b, colors.on_surface.b);
    assert_eq!(alpha_of(halo.color), 8);
    assert_ne!(
        halo.color.r, colors.primary.r,
        "control: this kit tints the halo with `on_surface`, where Cupertino uses `primary`"
    );

    assert_eq!(
        halo.color_pressed, None,
        "`None` means the halo keeps `color` while pressed; Material's `Some` is the other answer"
    );
}

/// Only the hover gate is open. v1's condition was `hover && !disabled`, and
/// nothing else ever turned the halo on -- no press, and no focus (the focus
/// outline v1 drew is recorded in the module doc).
#[test]
fn fluent_opens_only_the_hover_gate_and_suppresses_while_disabled() {
    let halo = style().halo;
    assert!(halo.on_hover, "v1 drew it while hovered");
    assert!(
        !halo.on_press,
        "v1 had no pressed state for the ring or the halo"
    );
    assert!(
        !halo.on_focus,
        "v1 drew a focus outline instead, and that is a thing the style cannot carry"
    );
    assert!(
        halo.suppressed_when_disabled,
        "v1 hid the halo outright when disabled"
    );
    assert_ne!(
        halo,
        StateLayer::default(),
        "the default draws nothing at all"
    );
    assert_eq!(StateLayer::default().size, 0.0);
}

/// **The tripwire for the kits converging.** v1 computed `size + pad * 2.0` with
/// `pad = 4.0`, which is 28 at today's 20px token; Material's is a flat 40. A
/// "harmonise the kits" edit that reached for the absolute form would land here
/// at 40, and one that dropped the padding would land at 20.
#[test]
fn the_halo_is_the_ring_plus_four_points_of_padding() {
    let style = style();
    assert_eq!(style.halo.size, style.size + HALO_PADDING * 2.0);
    assert_eq!(style.halo.size, 28.0);
    assert_eq!(
        style.touch_target, style.halo.size,
        "in v1 the halo *was* the hit area, so they are one number"
    );

    assert_ne!(
        style.halo.size, 40.0,
        "Material's halo is a flat 40 that does not track the ring; this one does"
    );
    assert_ne!(style.halo.size, style.size, "and it is padded, not bare");
}

/// The three layers between the halo, the ring and the label, all from the style
/// the kit resolved.
#[test]
fn the_mounted_radio_paints_this_kits_three_layers() {
    let style = style();
    let radio = FluentRadio::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&radio, ROW);

    assert_eq!(
        ops.len(),
        3,
        "rim, dot, label -- no halo while idle and no interior ever:\n{snapshot}"
    );
    assert_eq!(
        rim(&ops),
        Some((f64::from(style.border_width), style.selected_color))
    );
    assert_eq!(fills(&ops)[0].1, style.selected_dot_color);
    assert_eq!(label(&ops).map(|run| run.2), Some(style.label_color));
}

/// Hovering paints the halo -- the whole of this kit's interaction answer.
#[test]
fn hovering_opens_the_halo() {
    let style = style();
    let mut hovered = FluentRadio::new("A").with_theme(theme()).view();
    let id = allocate(&mut hovered);
    hover(&mut hovered, id);
    let (ops, snapshot) = paint(&hovered, ROW);
    assert_eq!(
        fills(&ops).len(),
        1,
        "the halo is the only new fill:\n{snapshot}"
    );
    assert_eq!(fills(&ops)[0].1, style.halo.color);
    assert_eq!(fills(&ops)[0].0.width(), f64::from(style.halo.size));
}

/// And pressing keeps the resting tint rather than deepening it. That is a
/// different behaviour from Material's, and `color_pressed: None` is what makes
/// it one: the press gate is closed, so the halo is still the hover halo.
#[test]
fn pressing_keeps_the_resting_halo_tint() {
    let style = style();
    let mut pressed = FluentRadio::new("A").with_theme(theme()).view();
    let id = allocate(&mut pressed);
    dispatch(
        &mut pressed,
        id,
        ROW,
        PointerEvent::primary_down(Point::new(8.0, 16.0)).into(),
    );
    let (ops, snapshot) = paint(&pressed, ROW);
    assert_eq!(
        fills(&ops)[0].1,
        style.halo.color,
        "a press does not deepen this kit's halo:\n{snapshot}"
    );
    assert_eq!(style.halo.color_pressed, None);
}

/// **The other half of this kit's halo, pinned.** Material's is drawn while
/// disabled; this one is not.
///
/// Focus rather than hover, because the engine refuses to set `hovered` on a
/// disabled widget at all -- `event` returns before the match -- so focus is the
/// only gate a disabled radio can be put through, and it is closed here. The
/// `suppressed_when_disabled` flag is asserted separately because it is a
/// second, independent reason, and only one of the two is reachable in a paint.
#[test]
fn the_halo_is_not_drawn_while_disabled() {
    let style = style();
    assert!(style.halo.suppressed_when_disabled);
    assert!(!style.halo.on_focus, "and focus is not an opening either");

    let mut disabled = FluentRadio::new("A")
        .with_theme(theme())
        .disabled(true)
        .view();
    disabled.set_focused(true);
    let (ops, snapshot) = paint(&disabled, ROW);
    assert_eq!(
        fills(&ops).len(),
        0,
        "a disabled radio paints a rim, a dot and a label and no halo:\n{snapshot}"
    );

    // And focus is not an answer on its own: an enabled, focused radio is still
    // bare, which is the other half of `on_focus: false`.
    let mut focused = FluentRadio::new("A").with_theme(theme()).view();
    focused.set_focused(true);
    let (ops, snapshot) = paint(&focused, ROW);
    assert_eq!(
        fills(&ops).len(),
        0,
        "focus opens no gate on this kit's halo:\n{snapshot}"
    );
}

/// v1 gave the ring no hover or press colour of its own -- the halo was the whole
/// answer -- so both are `None`. The engine's default is the opposite on both,
/// which is what makes this a decision rather than an omission.
#[test]
fn neither_interaction_variant_is_this_kits_own() {
    let style = style();
    assert!(style.unselected_hovered_color.is_none());
    assert!(style.unselected_pressed_color.is_none());

    let default = RadioStyle::default();
    assert!(
        default.unselected_hovered_color.is_some() && default.unselected_pressed_color.is_some(),
        "control: the engine carries a variant for each, so `None` here means this kit declined them rather than inheriting them"
    );
}

// --- the touch target -------------------------------------------------------

/// **The reason `RadioStyle` has a `touch_target` at all.** A layout that read
/// the ring's diameter instead would reserve 20 and lay this radio out a third
/// smaller than it paints; one that read the engine's constant would reserve 16.
/// Both fail here.
#[test]
fn the_touch_target_is_reserved_by_the_layout() {
    let style = style();
    let radio = FluentRadio::new("Ship it").with_theme(theme()).view();
    let request = radio
        .layout_request()
        .expect("a labelled radio asks for a size");

    assert_eq!(
        style.label_gap, V1_LABEL_GAP,
        "the label sits where v1 put it, not at the engine's shared gap"
    );
    assert_ne!(
        style.label_gap, ENGINE_LABEL_GAP,
        "if these are equal the two constants have been conflated and this \
         test no longer proves anything about the gap"
    );
    let expected_width =
        style.touch_target + style.label_gap + text_width_at("Ship it", style.label_font_size);
    assert_eq!(
        request.width.expect("a width").px().expect("in px"),
        expected_width,
        "the ring's touch target, this kit's gap, then the label at this kit's size"
    );
    assert!(
        request.height.expect("a height").px().expect("in px") >= style.touch_target,
        "the target is at least the height; a two-word label is shorter"
    );
    assert!(
        style.touch_target > 16.0,
        "control: the engine's default reserves 16, so this assertion is not vacuous"
    );
}

// --- mounting ---------------------------------------------------------------

/// The props cross into the widget, and both of the two places `disabled` lives
/// are set from the same builder flag.
#[test]
fn mounting_carries_the_props_across() {
    let radio = FluentRadio::new("Ship it")
        .with_theme(theme())
        .value("ship-it")
        .checked(true)
        .disabled(true)
        .view();
    assert!(radio.is_selected(), "the checked prop reached the widget");
    assert!(radio.interaction().disabled, "and so did disabled");
    assert!(
        disabled_style().disabled_color.is_some(),
        "the same flag also reached the style, which is a separate place"
    );
}

/// **`on_select` is `Fn()` and this kit still owes its caller a `Fn(bool)`.** The
/// new value is reconstructed from the `checked` prop the widget was built with,
/// so an activation reports `true` -- which is the whole of what a radio can
/// report.
#[test]
fn the_callback_reports_the_value_the_engine_activated_to() {
    let seen = Rc::new(Cell::new(None::<bool>));
    let sink = seen.clone();
    let mut radio = FluentRadio::new("A")
        .with_theme(theme())
        .on_change(move |value| sink.set(Some(value)))
        .view();
    let id = allocate(&radio);

    click(&mut radio, id);
    assert!(radio.is_selected(), "the engine owns the value now");
    assert_eq!(seen.get(), Some(true), "so the kit reports the new value");

    // And the edge: a radio built already selected never re-notifies, so the
    // adapter's `!was` is always `true`. v1's toggle could also report `false` --
    // clicking a selected radio cleared it -- and that is not reachable from here
    // because the engine has no unselect.
    let seen = Rc::new(Cell::new(None::<bool>));
    let sink = seen.clone();
    let mut already = FluentRadio::new("A")
        .with_theme(theme())
        .checked(true)
        .on_change(move |value| sink.set(Some(value)))
        .view();
    let id = allocate(&already);
    click(&mut already, id);
    assert_eq!(
        seen.get(),
        None,
        "the engine fires only on the false -> true edge, so a selected radio stays quiet"
    );
}

/// **The three design systems still resolve identical tokens, so a kit's identity
/// lives in its constants.** `MultiTheme::new_for` calls `activate`, which the
/// theme crate documents as a no-op placeholder, so Material, Cupertino and Fluent
/// hand back one identical set of colours and one `RadioTokens`. Every colour
/// assertion above is therefore on a number *this kit wrote*, never on a token
/// value -- a token assertion would pass for all three kits whatever any of them
/// did. This is the tripwire for the day per-DS tokens land.
#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let fluent = theme();
    let material = MultiTheme::new_for(DesignSystem::Material);
    let cupertino = MultiTheme::new_for(DesignSystem::Cupertino);
    let fluent_colors = fluent.colors();
    assert_eq!(material.colors().outline, fluent_colors.outline);
    assert_eq!(cupertino.colors().primary, fluent_colors.primary);

    let fluent_tokens = match fluent.component(ComponentId::Radio) {
        ComponentTokens::Radio(tokens) => tokens,
        _ => unreachable!("ComponentId::Radio must resolve to RadioTokens"),
    };
    assert_eq!(fluent_tokens.size, radio_tokens().size);
    assert_eq!(fluent_tokens.size, style().size);
}

/// The focus affordance v1 drew and the engine had no field for until now.
///
/// v1 ended its style closure with
/// `.focus(|s| s.outline(2.0).outline_color(colors().primary))`, so a focused
/// Fluent radio drew a 2px `primary` ring around the whole control -- the
/// indicator and the label together, because the closure was on the container
/// holding both. That is a `FocusRing` with no gap and no pinned radius.
#[test]
fn the_focus_affordance_is_a_ring_around_the_whole_control() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    let ring = style
        .focus_ring
        .expect("v1 drew `outline(2.0)` in `primary` and this is that ring");
    assert_eq!(ring.width, 2.0, "v1 wrote `outline(2.0)`");
    assert_eq!(ring.color.r, colors.primary.r, "in `primary`");
    assert_eq!(ring.color.g, colors.primary.g);
    assert_eq!(ring.color.b, colors.primary.b);
    assert_eq!(
        ring.gap, 0.0,
        "flush against the control: Compose's `outline()` has no inset"
    );
    assert_eq!(
        ring.radius, None,
        "no pinned radius, so the ring follows whatever the control's own \
         corners are"
    );

    assert_eq!(
        style.halo.on_focus, false,
        "and the halo does not also answer focus, or a focused radio would \
         carry two focus affordances at once"
    );
    assert_ne!(
        ring.color.r, colors.on_surface.r,
        "control: Fluent's *halo* is `on_surface`@8, which is a different answer \
         and must not be confused with this one"
    );
}
