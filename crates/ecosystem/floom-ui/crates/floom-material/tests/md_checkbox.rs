#![cfg(feature = "checkbox")]
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
use floom_material::widgets::checkbox::MdCheckbox;
use floom_material::widgets::icon::{MdIcon, MD_ICON_VIEW_BOX};

fn check_path() -> &'static str {
    MdIcon::new("check").path_data()
}
const CHECK_VIEW_BOX: Rect = MD_ICON_VIEW_BOX;

/// The box the checkbox is laid out in: wide enough for the indicator and its
/// label, tall enough for the indicator to be centred in it.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

const DISABLED_ALPHA: u8 = 38;
const DISABLED_CONTAINER_ALPHA: u8 = 12;

const V1_DEAD_ALPHA: u8 = 97;

const MARK_SCALE: f32 = 0.7;
const V1_LABEL_GAP: f32 = 8.0;
/// What the engine would have used, pinned so the two cannot be confused.
const ENGINE_LABEL_GAP: f32 = 6.0;

const LAYER_SIZE: f32 = 40.0;

/// The layer's alpha while hovered or focused, and while pressed. Material is the
/// only kit of the three with a pressed tint at all.
const LAYER_ALPHA: u8 = 20;
const LAYER_ALPHA_PRESSED: u8 = 30;

const OTHER_KITS_PADDING: f32 = 4.0;

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

/// Every style is read through an explicit theme rather than the app-wide signal,
/// so these tests cannot be moved by another test's global state.
fn style() -> CheckboxStyle {
    MdCheckbox::new("A").with_theme(theme()).resolved_style()
}

fn disabled_style() -> CheckboxStyle {
    MdCheckbox::new("A")
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

fn fills(ops: &[PaintOp]) -> Vec<(Rect, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::RoundedRect { bounds, color, .. } => Some((*bounds, *color)),
            _ => None,
        })
        .collect()
}

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

    let unchecked = MdCheckbox::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unchecked, ROW);
    let boxed = box_fills(&ops);
    assert_eq!(
        boxed.len(),
        1,
        "the engine fills the box in every state:\n{snapshot}"
    );
    assert_eq!(alpha_of(boxed[0].1), 0, "with nothing in it:\n{snapshot}");
    assert_eq!(boxed[0].0.width(), f64::from(style().size));

    let checked = MdCheckbox::new("A")
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

    let unchecked = MdCheckbox::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unchecked, ROW);
    let (width, ink) = rim(&ops).expect("a rim was stroked");
    assert_eq!(width, 2.0);
    assert_eq!(
        ink,
        tint(colors.outline, 255),
        "the resting rim:\n{snapshot}"
    );

    let checked = MdCheckbox::new("A")
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

    let checked = MdCheckbox::new("A")
        .with_theme(theme())
        .checked(true)
        .view();
    let (ops, snapshot) = paint(&checked, ROW);
    let (bounds, ink) = mark(&ops).expect("a mark was filled");
    assert_eq!(ink, tint(colors.on_primary, 255), "\n{snapshot}");

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

#[test]
fn an_unchecked_box_draws_no_mark() {
    let unchecked = MdCheckbox::new("A").with_theme(theme()).view();
    let (ops, snapshot) = paint(&unchecked, ROW);
    assert!(
        mark(&ops).is_none(),
        "an unchecked box draws a rim and a label and no mark:\n{snapshot}"
    );
}

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

    let checkbox = MdCheckbox::new("Ship it").with_theme(theme()).view();
    let (ops, snapshot) = paint(&checkbox, ROW);
    let (text, origin, size, ink) = label(&ops).expect("a label run");
    assert_eq!(text, "Ship it", "\n{snapshot}");
    assert_eq!(size, style.label_font_size);
    assert_eq!(ink, tint(colors.on_surface, 255), "\n{snapshot}");

    let indicator = checkbox.box_bounds(ROW);
    assert_eq!(
        origin.x - indicator.x1,
        f64::from(style.label_gap),
        "\n{snapshot}"
    );
}

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

#[test]
fn the_disabled_colour_reaches_every_layer() {
    let binding = theme();
    let colors = binding.colors();
    let dimmed = tint(colors.on_surface, DISABLED_ALPHA);
    let dim_fill = tint(colors.on_surface, DISABLED_CONTAINER_ALPHA);

    let checkbox = MdCheckbox::new("A")
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

/// The props cross into the widget, and both of the two places `disabled` lives
/// are set from the same builder flag.
#[test]
fn mounting_carries_the_props_across() {
    let checkbox = MdCheckbox::new("Ship it")
        .with_theme(theme())
        .checked(true)
        .disabled(true)
        .view();
    assert!(checkbox.is_checked(), "the checked prop reached the widget");
    assert!(checkbox.interaction().disabled, "and so did disabled");
}

#[test]
fn the_callback_reports_both_directions() {
    let seen = Rc::new(Cell::new(Vec::<bool>::new()));
    let sink = seen.clone();
    let mut checkbox = MdCheckbox::new("A")
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

#[test]
fn the_layout_reserves_the_indicator_the_gap_and_the_label() {
    let style = style();
    let checkbox = MdCheckbox::new("Ship it").with_theme(theme()).view();
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

#[test]
fn the_disabled_flag_does_not_move_the_style() {
    assert_eq!(
        disabled_style(),
        style(),
        "`CheckboxStyle` names a colour per state; `Checkbox` picks"
    );
}

#[test]
fn the_layer_is_a_flat_forty_circle() {
    let style = style();
    assert_eq!(style.layer.size, LAYER_SIZE);
    assert_eq!(style.layer.size, 40.0);
    assert_eq!(style.layer.shape, LayerShape::Circle, "v1 wrote `9999.0`");
    assert_ne!(
        style.layer.shape,
        LayerShape::Box {
            padding: OTHER_KITS_PADDING
        },
        "the other two kits' layer is a padded box"
    );
    assert_ne!(
        style.layer.size,
        style.size + OTHER_KITS_PADDING * 2.0,
        "Material's is absolute, not `size + 8`"
    );
    assert!(
        style.size + OTHER_KITS_PADDING * 2.0 < LAYER_SIZE,
        "control: the two forms really do differ at today's token, or this \
         assertion proves nothing"
    );
}

/// The layer is `primary` at 20 while hovered or focused, and `primary` at 30
/// while pressed. Material is the only kit of the three with a second tint.
#[test]
fn the_layer_is_primary_at_twenty_and_thirty_while_pressed() {
    let binding = theme();
    let colors = binding.colors();
    let layer = style().layer;

    assert_eq!(layer.color, tint(colors.primary, LAYER_ALPHA));
    assert_eq!(alpha_of(layer.color), 20);

    let pressed = layer
        .color_pressed
        .expect("Material is the only kit with one");
    assert_eq!(pressed, tint(colors.primary, LAYER_ALPHA_PRESSED));
    assert_eq!(alpha_of(pressed), 30);
    assert_ne!(
        layer.color, pressed,
        "control: a pressed tint that equals the resting one is not a second answer"
    );
}

#[test]
fn material_opens_all_three_layer_gates_and_suppresses_none() {
    let layer = style().layer;
    assert!(layer.on_hover, "v1 drew it while hovered");
    assert!(layer.on_press, "v1 drew it while pressed");
    assert!(layer.on_focus, "v1 drew it while focused");
    assert!(
        !layer.suppressed_when_disabled,
        "Material's layer carries no disabled rule"
    );
    assert_ne!(layer, StateLayer::default(), "the default draws nothing");
    assert_eq!(StateLayer::default().size, 0.0);
}

/// Hovering paints the layer and pressing swaps in the pressed tint -- the two
/// halves of Material's interaction answer, which is all it has.
#[test]
fn hovering_and_pressing_open_the_layer() {
    let style = style();
    let at = Point::new(8.0, 16.0);

    let mut hovered = MdCheckbox::new("A").with_theme(theme()).view();
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

    let mut pressed = MdCheckbox::new("A").with_theme(theme()).view();
    let id = allocate(&pressed);
    dispatch(&mut pressed, id, ROW, PointerEvent::primary_down(at).into());
    let (ops, snapshot) = paint(&pressed, ROW);
    assert_eq!(
        layer_fills(&ops)[0].1,
        style.layer.color_pressed.expect("a pressed tint"),
        "a press swaps the tint, not the size:\n{snapshot}"
    );
}

#[test]
fn the_layer_is_drawn_while_disabled() {
    let style = style();
    assert!(!style.layer.suppressed_when_disabled);

    let mut disabled = MdCheckbox::new("A")
        .with_theme(theme())
        .disabled(true)
        .view();
    disabled.set_focused(true);
    let (ops, snapshot) = paint(&disabled, ROW);
    let layers = layer_fills(&ops);
    assert_eq!(
        layers.len(),
        1,
        "the layer is still there while disabled:\n{snapshot}"
    );
    assert_eq!(layers[0].1, style.layer.color, "\n{snapshot}");
}

#[test]
fn material_answers_focus_with_the_layer_and_draws_no_ring() {
    let style = style();

    assert_eq!(
        style.focus_ring, None,
        "v1 drew no outline on this kit's checkbox; `None` is that answer, not \
         an absence of one"
    );
    assert!(
        style.layer.on_focus,
        "the layer is what carries focus here, so its focus gate is open"
    );
}

/// And focus really does paint the layer, which is the other half of that claim:
/// a focused, enabled, idle checkbox shows one that no other state opened.
#[test]
fn focus_paints_the_layer() {
    let style = style();
    let mut focused = MdCheckbox::new("A").with_theme(theme()).view();
    focused.set_focused(true);
    let (ops, snapshot) = paint(&focused, ROW);
    let layers = layer_fills(&ops);
    assert_eq!(
        layers.len(),
        1,
        "focus opens the layer and nothing else:\n{snapshot}"
    );
    assert_eq!(layers[0].1, style.layer.color);
    assert_eq!(layers[0].0.width(), f64::from(style.layer.size));
}

#[test]
fn material_never_carried_a_design_system_field() {
    let style = style();
    assert_eq!(
        std::mem::size_of_val(&style),
        std::mem::size_of::<CheckboxStyle>(),
        "`MdCheckboxStyle` *is* the engine's style"
    );
}

#[test]
fn material_does_not_resolve_to_the_other_two_kits_layer_or_focus() {
    let style = style();

    let other_layer = StateLayer {
        color: style.layer.color,
        color_pressed: None,
        size: style.size + 8.0,
        shape: LayerShape::Box { padding: 4.0 },
        on_hover: true,
        on_press: false,
        on_focus: false,
        suppressed_when_disabled: true,
    };
    assert_ne!(
        style.layer, other_layer,
        "Material's layer is a flat 40 circle with a pressed tint, gated on all \
         three and never suppressed"
    );
    assert_ne!(style.layer.size, other_layer.size);
    assert_ne!(style.layer.shape, other_layer.shape);
    assert_ne!(style.layer.color_pressed, other_layer.color_pressed);
    assert_ne!(
        style.layer.suppressed_when_disabled,
        other_layer.suppressed_when_disabled
    );

    assert_eq!(style.focus_ring, None);
    assert!(style.layer.on_focus);
    assert_ne!(
        style.layer.on_focus, other_layer.on_focus,
        "Material's focus gate is open; the other two answer focus with a ring"
    );
}
