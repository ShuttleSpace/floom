#![cfg(feature = "radio")]
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
use floom_material::widgets::radio::MdRadio;

/// The box the radio is laid out in: wide enough for the ring and its label, tall
/// enough for the ring to be centred in it.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

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

    let ring = selected.dot_bounds(ROW);
    assert!((dot.x0 + dot.width() / 2.0 - ring.center().x).abs() < 1e-9);
}

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

#[test]
fn a_disabled_radio_is_not_the_enabled_one() {
    assert_ne!(disabled_style(), style());
}

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
        StateLayer::default(),
        "the default draws nothing at all"
    );
    assert_eq!(StateLayer::default().size, 0.0);
}

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
