//! Material's radio as a kit crate: the token translation, plus the two numbers
//! that are this kit's own rather than the tokens' -- a halo 40 points across
//! that does not scale with the ring, and a pressed tint for it.
//!
//! These three crates have never been compiled, which is how three kits came to
//! disagree with themselves about a five-layer control for this long. So the
//! point of this file is not to exercise the engine's `Radio` -- that is what
//! `floom-ui-composites`' own radio tests are for -- but to pin the numbers *this
//! kit wrote*, in the places where a plausible-looking edit would change a pixel.
//!
//! ## What this file deliberately does not assert
//!
//! v1 drew no interior behind the ring, so a `RadioStyle` whose `interior_color`
//! is `None` paints one layer fewer than the engine's default. That absence is
//! asserted, because a `Some(white)` here would be visible; the *engine's*
//! behaviour for a `None` interior belongs to the engine's tests.
//!
//! ## The tripwires
//!
//! Three of these tests would fail if the kit stopped being this kit:
//! `the_halo_is_forty_across_and_does_not_track_the_ring`,
//! `a_disabled_radio_is_not_the_enabled_one` and `the_halo_is_drawn_while_disabled`.
//! They are why the three kits stay three crates rather than becoming one.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{Color, ElementId, Point, Rect, Widget};
use floom_types::PointerEvent;
use floom_ui_composites::{Radio, RadioHalo, RadioStyle};
use floom_widgets::testkit::{dispatch, AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::text_width_at;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, RadioTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_md_radio::MdRadio;

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

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

/// Every style is read through an explicit theme rather than the app-wide signal,
/// so these tests cannot be moved by another test's global state.
fn style() -> RadioStyle {
    MdRadio::new("A").with_theme(theme()).resolved_style()
}

fn disabled_style() -> RadioStyle {
    MdRadio::new("A")
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

    let unselected = MdRadio::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unselected, ROW);
    let (_, ink) = rim(&ops).expect("a rim was stroked");
    assert_eq!(ink.r, colors.outline.r, "the resting rim:\n{snapshot}");
    assert_eq!(ink.g, colors.outline.g);
    assert_eq!(ink.b, colors.outline.b);
    assert_eq!(alpha_of(ink), 255, "undimmed");

    let selected = MdRadio::new("A").with_theme(theme()).checked(true).view();
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
    let selected = MdRadio::new("A").with_theme(theme()).checked(true).view();
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
/// middle of every Material radio showed whatever its parent was painted with.
///
/// The engine's default fills it white, which would be a white disc inside a
/// transparent ring.
#[test]
fn the_interior_is_transparent() {
    assert!(style().interior_color.is_none());

    let unselected = MdRadio::new("A").with_theme(theme()).view();
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
    let radio = MdRadio::new("A")
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

    let radio = MdRadio::new("Ship it").with_theme(theme()).view();
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

/// Material is the only kit of the three with a second alpha: `primary` at 20
/// while hovered or focused, and `primary` at 30 while pressed. The other two
/// write `None`, which means the halo keeps its resting colour.
#[test]
fn the_halo_is_primary_at_twenty_and_thirty_while_pressed() {
    let binding = theme();
    let colors = binding.colors();
    let halo = style().halo;

    assert_eq!(halo.color.r, colors.primary.r);
    assert_eq!(halo.color.g, colors.primary.g);
    assert_eq!(halo.color.b, colors.primary.b);
    assert_eq!(alpha_of(halo.color), 20);

    let pressed = halo
        .color_pressed
        .expect("Material is the only kit with one");
    assert_eq!(pressed.r, colors.primary.r);
    assert_eq!(alpha_of(pressed), 30);
}

/// Every gate Material opens, and the one it does not. v1's halo condition was
/// `hover || pressed || focused`, with no disabled check anywhere in the style --
/// see the module doc for why that flag is `false` when v1's event handlers
/// guarded themselves instead.
#[test]
fn material_opens_all_three_halo_gates_and_suppresses_none() {
    let halo = style().halo;
    assert!(halo.on_hover, "v1 drew it while hovered");
    assert!(halo.on_press, "v1 drew it while pressed");
    assert!(halo.on_focus, "v1 drew it while focused");
    assert!(
        !halo.suppressed_when_disabled,
        "Material's halo carries no disabled rule"
    );
    assert_ne!(
        halo,
        RadioHalo::default(),
        "the default draws nothing at all"
    );
    assert_eq!(RadioHalo::default().size, 0.0);
}

/// **The tripwire for the two kits converging.** v1 hard-coded
/// `.size(40.0, 40.0)`; Cupertino and Fluent computed `size + pad * 2.0`, which
/// is 28 at today's 20px token. A "harmonise the kits" edit that reached for the
/// relative form would land here at 28, and one that reached for the absolute one
/// would land in the other two crates at 40.
#[test]
fn the_halo_is_forty_across_and_does_not_track_the_ring() {
    let style = style();
    assert_eq!(style.halo.size, 40.0);
    assert_eq!(
        style.touch_target, 40.0,
        "in v1 the halo *was* the hit area, so they are one number"
    );

    assert_ne!(
        style.halo.size,
        style.size + 8.0,
        "Material's halo is absolute; the other two kits write `size + 8`"
    );
    assert_ne!(
        style.touch_target,
        style.size + 8.0,
        "and the touch target is the same number as the halo, not the padded ring"
    );
    assert_eq!(
        style.size + 8.0,
        28.0,
        "the number Cupertino and Fluent write, so this is not a coincidence of tokens"
    );
}

/// The three layers between the halo, the ring and the label, all from the style
/// the kit resolved.
#[test]
fn the_mounted_radio_paints_this_kits_three_layers() {
    let style = style();
    let radio = MdRadio::new("A").with_theme(theme()).checked(true).view();
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

/// Hovering paints the halo, and pressing swaps in the pressed tint -- the two
/// halves of Material's interaction answer, which is all it has.
#[test]
fn hovering_and_pressing_open_the_halo() {
    let style = style();
    let at = Point::new(8.0, 16.0);

    let mut hovered = MdRadio::new("A").with_theme(theme()).view();
    let hovered_id = allocate(&hovered);
    hover(&mut hovered, hovered_id);
    let (ops, snapshot) = paint(&hovered, ROW);
    assert_eq!(
        fills(&ops).len(),
        1,
        "the halo is the only new fill:\n{snapshot}"
    );
    assert_eq!(fills(&ops)[0].1, style.halo.color);
    assert_eq!(fills(&ops)[0].0.width(), f64::from(style.halo.size));

    let mut pressed = MdRadio::new("A").with_theme(theme()).view();
    let pressed_id = allocate(&pressed);
    dispatch(
        &mut pressed,
        pressed_id,
        ROW,
        PointerEvent::primary_down(at).into(),
    );
    let (ops, snapshot) = paint(&pressed, ROW);
    assert_eq!(
        fills(&ops)[0].1,
        style.halo.color_pressed.expect("a pressed tint"),
        "a press swaps the tint, not the size:\n{snapshot}"
    );
}

/// **The claim the module doc makes, pinned.** Material's halo is drawn while the
/// radio is disabled, because the style carries no disabled rule.
///
/// Focus rather than hover, because the engine refuses to set `hovered` on a
/// disabled widget at all -- `event` returns before the match -- so a hover-based
/// version of this test would pass for the wrong reason and would stop being a
/// claim about the flag the moment the engine taught itself to hover disabled
/// controls.
#[test]
fn the_halo_is_drawn_while_disabled() {
    let style = style();
    assert!(!style.halo.suppressed_when_disabled);

    let mut radio = MdRadio::new("A").with_theme(theme()).disabled(true).view();
    radio.set_focused(true);
    let (ops, snapshot) = paint(&radio, ROW);
    let fills = fills(&ops);
    assert_eq!(
        fills.len(),
        1,
        "the halo is the only fill a disabled radio has -- no dot, no interior:\n{snapshot}"
    );
    assert_eq!(fills[0].0.width(), f64::from(style.halo.size));
    assert_eq!(fills[0].1, style.halo.color, "\n{snapshot}");
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
/// the ring's diameter instead would reserve 20 and lay this radio out a fifth
/// smaller than it paints; one that read the engine's constant would reserve 16.
/// Both fail here.
#[test]
fn the_touch_target_is_reserved_by_the_layout() {
    let style = style();
    let radio = MdRadio::new("Ship it").with_theme(theme()).view();
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
    let radio = MdRadio::new("Ship it")
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
    let mut radio = MdRadio::new("A")
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
    let mut already = MdRadio::new("A")
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
    let material = theme();
    let cupertino = MultiTheme::new_for(DesignSystem::Cupertino);
    let fluent = MultiTheme::new_for(DesignSystem::Fluent);
    let material_colors = material.colors();
    assert_eq!(cupertino.colors().outline, material_colors.outline);
    assert_eq!(fluent.colors().primary, material_colors.primary);

    let fluent_tokens = match fluent.component(ComponentId::Radio) {
        ComponentTokens::Radio(tokens) => tokens,
        _ => unreachable!("ComponentId::Radio must resolve to RadioTokens"),
    };
    assert_eq!(fluent_tokens.size, radio_tokens().size);
    assert_eq!(fluent_tokens.size, style().size);
}

/// Material answers focus with its halo, not with a ring.
///
/// This is the control for the other two kits' tests: a `FocusRing` here would
/// be a second focus affordance on the same control, and Material's v1 drew
/// exactly one -- the 40px halo, gated on focus as well as hover and press.
#[test]
fn material_answers_focus_with_the_halo_and_draws_no_ring() {
    let style = style();

    assert_eq!(
        style.focus_ring, None,
        "v1 drew no outline on this kit's radio; `None` is that answer, not an \
         absence of one"
    );
    assert_eq!(
        style.halo.on_focus, true,
        "the halo is what carries focus here, so its focus gate is open"
    );
}
