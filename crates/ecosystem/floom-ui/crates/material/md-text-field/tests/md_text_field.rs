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
use floom_widget_md_text_field::MdTextField;

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 240.0,
    y1: 48.0,
};

const DISABLED_ALPHA: u8 = 38;

const DISABLED_FILL_ALPHA: u8 = 4;

const V1_DEAD_ALPHA: u8 = 97;

fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
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
    MdTextField::new().with_theme(theme()).resolved_style()
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

fn type_text(field: &mut TextField, id: ElementId, text: &str) {
    for ch in text.chars() {
        dispatch(field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
}

#[test]
fn the_geometry_is_this_kits_text_field_tokens() {
    let tokens = field_tokens();
    let style = style();

    assert_eq!(style.height, Some(tokens.height), "the pinned row height");
    assert_eq!(style.height, Some(48.0), "v1 read `tokens.height`");
    assert_eq!(
        style.padding_horizontal, tokens.padding_horizontal,
        "the text inset, either side"
    );
    assert_eq!(style.padding_horizontal, 16.0);
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the rim's corners"
    );
    assert_eq!(style.border_radius, 4.0);

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

#[test]
fn the_placeholder_and_the_preedit_are_inherited_not_themed() {
    let binding = theme();
    let colors = binding.colors();
    let default = TextFieldStyle::default();
    let style = style();

    assert_eq!(style.placeholder_color, default.placeholder_color);
    assert_eq!(style.preedit_color, default.preedit_color);

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

#[test]
fn the_field_measures_its_own_content_rather_than_filling_its_parent() {
    let style = style();
    let field = MdTextField::new().with_theme(theme()).value("hi").view();
    let request = field.layout_request().expect("an intrinsic size");

    assert_eq!(
        px(request.height),
        style.height.expect("this kit pins its height"),
        "the row is still 48pt tall; only the width is in question"
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

    let empty = MdTextField::new()
        .with_theme(theme())
        .placeholder("Name")
        .view();
    let request = empty.layout_request().expect("an intrinsic size");
    assert!(
        px(request.width) >= style.min_width,
        "an empty field is usable: the placeholder sets the floor"
    );
}

#[test]
fn max_length_reaches_the_engine_and_clamps_typing_too() {
    let capped = MdTextField::new()
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

    let mut typed = MdTextField::new().with_theme(theme()).max_length(8).view();
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

#[test]
fn on_change_reports_typing_and_not_the_builders_value() {
    let seen = Rc::new(Cell::new(String::new()));
    let sink = seen.clone();
    let mut field = MdTextField::new()
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

#[test]
fn password_is_carried_and_still_renders_in_the_clear() {
    let kit = MdTextField::new()
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

#[test]
fn a_disabled_field_is_not_the_enabled_one() {
    let binding = theme();
    let colors = binding.colors();

    let enabled = MdTextField::new().with_theme(theme()).value("hi").view();
    let disabled = MdTextField::new()
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

#[test]
fn a_disabled_field_does_not_accept_typing() {
    let mut disabled = MdTextField::new().with_theme(theme()).disabled(true).view();
    let id = allocate(&disabled);
    disabled.set_focused(true);
    type_text(&mut disabled, id, "nope");
    assert_eq!(disabled.text(), "", "the engine's own rule, not this kit's");
}

#[test]
fn the_props_cross_into_the_widget() {
    let field = MdTextField::new()
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

    let empty = MdTextField::new()
        .with_theme(theme())
        .placeholder("Name")
        .view();
    assert_eq!(
        empty.semantics_hint().label.as_deref(),
        Some("Name"),
        "so the placeholder does reach assistive technology while empty"
    );
}

#[test]
fn the_focus_affordance_is_a_thicker_rim() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();

    assert_eq!(style.border_width, 1.0, "v1's resting rim");
    assert_eq!(
        style.border_width_focused, 2.0,
        "v1 wrote `.border(if focused {{ 2.0 }} else {{ 1.0 }})`, and that \
         conditional is this kit's entire focus affordance"
    );
    assert_eq!(
        style.focus_ring, None,
        "no ring outside the box: Material and Cupertino both draw none, and \
         Fluent's `2.0` in `primary` is the third answer"
    );
    assert!(
        style.border_color_hovered.is_none(),
        "and no hovered rim: Cupertino's `primary` at 120 is the other answer"
    );
    assert_eq!(style.border_color_focused, tint(colors.primary, 255));

    assert_ne!(
        style.border_width_focused, 1.0,
        "control: Cupertino's and Fluent's rims never thicken"
    );
    assert_ne!(
        style.focus_ring.as_ref().map(|r| r.width),
        Some(2.0),
        "control: Fluent's ring"
    );
    assert_ne!(
        style.border_color_hovered,
        Some(tint(colors.primary, 120)),
        "control: Cupertino's hovered rim"
    );
}

#[test]
fn focusing_strokes_a_thicker_rim_and_no_ring() {
    let binding = theme();
    let colors = binding.colors();

    let mut focused = MdTextField::new().with_theme(theme()).value("hi").view();
    focused.set_focused(true);
    let (ops, snapshot) = paint(&focused);

    let paths = strokes(&ops);
    assert_eq!(
        paths.len(),
        1,
        "one rim and no ring outside the box:\n{snapshot}"
    );
    assert_eq!(paths[0].0, 2.0, "the thicker rim:\n{snapshot}");
    assert_eq!(paths[0].1, tint(colors.primary, 255));
    assert!(
        !snapshot.contains("Path (-1.0"),
        "a ring would be stroked half a width outside every edge; there is none:\n{snapshot}"
    );

    let idle = MdTextField::new().with_theme(theme()).value("hi").view();
    let (ops, snapshot) = paint(&idle);
    assert_eq!(
        strokes(&ops).len(),
        1,
        "still one stroke at rest:\n{snapshot}"
    );
    assert_eq!(
        strokes(&ops)[0].0,
        1.0,
        "and it is one point wide until focused:\n{snapshot}"
    );
    assert_eq!(strokes(&ops)[0].1, tint(colors.outline, 255));
}

#[test]
fn hovering_leaves_this_kits_rim_alone() {
    let binding = theme();
    let colors = binding.colors();
    let mut hovered = MdTextField::new().with_theme(theme()).value("hi").view();
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
    assert_eq!(strokes(&ops)[0].0, 1.0);

    leave(&mut hovered, id);
    let (ops, snapshot) = paint(&hovered);
    assert_eq!(
        strokes(&ops)[0].1,
        tint(colors.outline, 255),
        "and leaving puts `outline` back:\n{snapshot}"
    );
}

#[test]
fn the_theme_binding_is_reachable_on_both_constructors() {
    let kit = MdTextField::new();
    let _ = kit.theme();
    assert_eq!(kit.props().placeholder, "", "and the props start empty");
}
