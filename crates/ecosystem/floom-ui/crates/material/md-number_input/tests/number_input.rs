
use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, Rect, Widget};
use floom_types::{Color, Key, KeyEvent, Point, PointerEvent};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_number_input::{MaterialNumberInput, MaterialNumberInputStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialNumberInput {
    MaterialNumberInput::new().with_theme(theme())
}

/// Theme colour -> engine colour.
fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// One widget's paint ops.
fn ops_of<W: Widget + ?Sized>(widget: &W) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// A press and a release over the same spot: what a click is.
fn click<W: Widget + ?Sized>(widget: &mut W) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut down = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_down(Point::new(10.0, 10.0))),
    );
    widget.event(&mut down, id);
    let mut up = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    widget.event(&mut up, id);
}

/// A character typed into a field.
fn type_text<W: Widget + ?Sized>(widget: &mut W, ch: char) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut key = TestEventContext::new(id, BOX, Event::Key(KeyEvent::down(Key::Char(ch))));
    widget.event(&mut key, id);
}

/// The field: the last child of the column.
fn field_of(view: &floom_widgets::Stack) -> Box<dyn floom_core::WidgetAny> {
    let mut children = view.children();
    children.remove(children.len() - 1).build()
}

/// The row inside the field.
fn row_of(field: &dyn floom_core::WidgetAny) -> Box<dyn floom_core::WidgetAny> {
    field.children().remove(0).build()
}


/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialNumberInputStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::NumberInput) {
            floom_core_theme::component_tokens::ComponentTokens::NumberInput(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.field_height, tokens.height, "the token's height");
    assert_eq!(style.field_width, tokens.width, "the token's width");
    assert_eq!(
        style.button_size, tokens.button_size,
        "the token's button size"
    );
    assert_eq!(
        style.padding_horizontal, tokens.padding_horizontal,
        "the token's horizontal padding"
    );
    assert_eq!(style.border_width, tokens.border, "the token's rim");
    assert_eq!(style.border_radius, tokens.radius, "the token's radius");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(
        style.text_color,
        theme().colors().on_surface,
        "the text's ink"
    );
    assert_eq!(
        style.background,
        theme().colors().surface,
        "the field's fill"
    );
    assert_eq!(
        style.border_color,
        theme().colors().outline,
        "the field's rim"
    );
    assert_eq!(
        style.button_background,
        theme().colors().surface_variant,
        "the buttons' fill"
    );
    assert_eq!(
        style.button_color,
        theme().colors().on_surface,
        "the buttons' ink"
    );
    assert_eq!(
        style.label_color,
        theme().colors().on_surface_variant,
        "the label's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}


#[test]
fn the_field_is_painted_in_v1_s_colours() {
    let resolved = kit().resolved_style();
    let view = kit().view();
    let field = field_of(&view);
    let ops = ops_of(&*field);

    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.background)), "the field's fill");

    let rim = ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((_, color)),
            ..
        } => Some(*color),
        _ => None,
    });
    assert_eq!(rim, Some(paint(resolved.border_color)), "the field's rim");

    let request = field
        .layout_request()
        .expect("the field asks for its own space");
    assert_eq!(
        request.height,
        Some(floom_core::Length::Px(resolved.field_height)),
        "the field's height"
    );
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(resolved.field_width)),
        "the field's width"
    );
}

#[test]
fn the_text_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let view = kit().value(4).view();
    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let input = children.remove(1).build();

    let ops = ops_of(&*input);
    let (text, size, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the text is painted");
    assert_eq!(text, "4", "the number the field was given");
    assert_eq!(size, resolved.font_size, "v1's font size");
    assert_eq!(ink, paint(resolved.text_color), "v1's ink");
}

#[test]
fn the_two_steppers_are_tiles_of_the_token_s_size() {
    let resolved = kit().resolved_style();
    let view = kit().view();
    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    assert_eq!(
        children.len(),
        3,
        "the two steppers and the text between them"
    );
    let dec = children.remove(0).build();
    let inc = children.remove(1).build();

    for (pressable, glyph) in [(dec, "-"), (inc, "+")] {
        let tile = pressable.children().remove(0).build();
        let ops = ops_of(&*tile);
        let fill = ops.iter().find_map(|op| match op {
            PaintOp::RoundedRect { color, .. } => Some(*color),
            PaintOp::Rect { color, .. } => Some(*color),
            _ => None,
        });
        assert_eq!(
            fill,
            Some(paint(resolved.button_background)),
            "the tile's fill"
        );
        let mark = tile.children().remove(0).build();
        let text = ops_of(&*mark)
            .iter()
            .find_map(|op| match op {
                PaintOp::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .expect("the glyph is painted");
        assert_eq!(text, glyph, "the glyph on the tile");
    }
}


#[test]
fn the_label_sits_above_the_field() {
    let view = kit().label("Quantity").view();
    let mut children = view.children();
    assert_eq!(children.len(), 2, "the label and the field");

    let label = children.remove(0).build();
    let ops = ops_of(&*label);
    let (text, size, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the label is painted");
    assert_eq!(text, "Quantity");
    assert_eq!(size, 12.0, "v1's label size");
    assert_eq!(
        ink,
        paint(kit().resolved_style().label_color),
        "v1's label ink"
    );
}

/// v1 left the label out when there was none, and so does this.
#[test]
fn no_label_leaves_only_the_field() {
    let view = kit().view();
    assert_eq!(view.children().len(), 1, "only the field");
}

/// The stepper moves the number and fires the callback.
#[test]
fn the_stepper_moves_the_number() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().value(4).step(2).on_change(move |value| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(value);
    });
    let number = kit.number();
    let view = kit.view();

    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let mut inc = children.remove(2).build();
    click(&mut *inc);

    assert_eq!(
        *number.lock().expect("the number is not poisoned"),
        6,
        "the number moved by the step"
    );
    assert_eq!(
        *seen.lock().expect("the recorder is not poisoned"),
        vec![6],
        "and the callback heard the new value"
    );
}

/// The `-` button steps down.
#[test]
fn the_decrement_button_steps_down() {
    let kit = kit().value(4).step(2);
    let number = kit.number();
    let view = kit.view();
    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let mut dec = children.remove(0).build();
    click(&mut *dec);
    assert_eq!(
        *number.lock().expect("the number is not poisoned"),
        2,
        "the number moved down by the step"
    );
}

/// The number is clamped to the bounds, and the text follows it.
#[test]
fn the_number_is_clamped_to_the_bounds() {
    let kit = kit().value(20).min(0).max(10);
    assert_eq!(kit.current_value(), 10, "the value was clamped");
    let view = kit.view();
    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let input = children.remove(1).build();
    let ops = ops_of(&*input);
    let text = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("the text is painted");
    assert_eq!(text, "10", "and the field shows the clamped number");
}

/// Typing a number moves the value and fires the callback.
#[test]
fn typing_a_number_moves_the_value() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().on_change(move |value| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(value);
    });
    let number = kit.number();
    let view = kit.view();

    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let mut input = children.remove(1).build();
    type_text(&mut *input, '7');

    assert_eq!(
        *number.lock().expect("the number is not poisoned"),
        7,
        "the typed number is the value"
    );
    assert_eq!(
        *seen.lock().expect("the recorder is not poisoned"),
        vec![7],
        "and the callback heard it"
    );
}

/// A disabled field does not answer: the value does not move.
#[test]
fn a_disabled_field_does_not_answer() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().disabled(true).on_change(move |value| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(value);
    });
    let number = kit.number();
    let view = kit.view();

    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let mut input = children.remove(1).build();
    type_text(&mut *input, '7');

    assert_eq!(
        *number.lock().expect("the number is not poisoned"),
        0,
        "a disabled field does not move"
    );
    assert!(
        seen.lock()
            .expect("the recorder is not poisoned")
            .is_empty(),
        "and does not answer"
    );
}

/// A disabled stepper does not answer either.
#[test]
fn a_disabled_stepper_does_not_answer() {
    let kit = kit().value(4).disabled(true);
    let number = kit.number();
    let view = kit.view();
    let field = field_of(&view);
    let row = row_of(&*field);
    let mut children = row.children();
    let mut inc = children.remove(2).build();
    click(&mut *inc);
    assert_eq!(
        *number.lock().expect("the number is not poisoned"),
        4,
        "a disabled stepper does not move the number"
    );
}

#[test]
fn the_disabled_palette_is_v1_s() {
    let style = kit().disabled(true).resolved_style();
    let palette = theme();
    let colors = palette.colors();
    assert_eq!(
        style.text_color,
        floom_core_theme::tokens::color::Color {
            a: 38,
            ..colors.on_surface
        },
        "v1's disabled ink"
    );
    assert_eq!(
        style.background,
        floom_core_theme::tokens::color::Color {
            a: 4,
            ..colors.on_surface
        },
        "v1's disabled fill"
    );
    assert_eq!(
        style.border_color,
        floom_core_theme::tokens::color::Color {
            a: 38,
            ..colors.on_surface
        },
        "v1's disabled rim"
    );
}

/// The props report the number, and the shared props' own callback slot stays
/// empty: the kit keeps a shareable copy of its own.
#[test]
fn the_props_report_the_number() {
    let kit = kit().value(7).min(1).max(9).step(3).label("Quantity");
    let props = kit.props();
    assert_eq!(props.value, 7);
    assert_eq!(props.min, Some(1));
    assert_eq!(props.max, Some(9));
    assert_eq!(props.step, 3);
    assert_eq!(props.label.as_deref(), Some("Quantity"));
    assert!(
        props.on_change.is_none(),
        "the shared props' callback is not shareable, so the kit keeps its own"
    );
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialNumberInput::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material number input read the app-wide signal, not a snapshot"
    );
}
