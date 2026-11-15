#![cfg(feature = "confirm_edit")]
use floom_core::{
    Child, ChildKey, ElementId, Event, Key, KeyEvent, LayoutRequest, Rect, Widget, WidgetAny,
};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::confirm_edit::{MaterialConfirmEdit, MaterialConfirmEditStyle};

/// The box the confirm-edit is laid out in.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 80.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialConfirmEdit {
    MaterialConfirmEdit::new().with_theme(theme())
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

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// One widget's paint ops.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(ROW);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// Dispatch an event to a widget held behind a trait object.
fn dispatch_any(widget: &mut dyn WidgetAny, id: ElementId, bounds: Rect, event: Event) -> bool {
    let mut cx = TestEventContext::new(id, bounds, event);
    widget.event(&mut cx, id)
}

/// The row the confirm-edit holds, in the mode it is in.
fn row_of(widget: &dyn Widget) -> Box<dyn WidgetAny> {
    widget.children().remove(0).build()
}

/// Click a widget, the way a pointer does: a press and a release.
fn click(widget: &mut dyn WidgetAny) {
    let id = allocate(&*widget);
    dispatch_any(
        &mut *widget,
        id,
        ROW,
        Event::Pointer(PointerEvent::primary_down(floom_core::Point::new(4.0, 4.0))),
    );
    dispatch_any(
        &mut *widget,
        id,
        ROW,
        Event::Pointer(PointerEvent::primary_up(floom_core::Point::new(4.0, 4.0))),
    );
}

/// The tokens reach the style.
#[test]
fn the_tokens_reach_the_style() {
    let style: MaterialConfirmEditStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::ConfirmEdit) {
            floom_core_theme::component_tokens::ComponentTokens::ConfirmEdit(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.gap, tokens.gap, "the gap between the parts of a row");
    assert_eq!(style.radius, tokens.radius, "the field's radius");
    assert_eq!(
        style.icon_size, tokens.icon_size,
        "the token's icon size, which v1 never used"
    );
}

/// The colours reach the style.
#[test]
fn the_colours_reach_the_style() {
    let style = kit().resolved_style();
    let palette = theme();
    let colors = palette.colors();
    assert_eq!(style.on_surface, colors.on_surface, "the field's ink");
    assert_eq!(
        style.on_surface_variant, colors.on_surface_variant,
        "the cross's and the edit affordance's ink"
    );
    assert_eq!(style.primary, colors.primary, "the tick's ink");
    assert_eq!(style.outline, colors.outline, "the field's outline");
}

#[test]
fn the_literals_are_in_the_style() {
    let style = kit().resolved_style();
    assert_eq!(style.glyph_size, 16.0, "the glyphs' size");
    assert_eq!(style.button_padding, 8.0, "the padding around a glyph");
    assert_eq!(style.input_width, 160.0, "the field's width");
    assert_eq!(style.input_font_size, 14.0, "the field's font size");
    assert_eq!(style.input_padding_horiz, 8.0, "the field's padding");
    assert_eq!(style.input_border, 1.0, "the field's outline");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

/// The props report the value.
#[test]
fn the_props_report_the_value() {
    let kit = kit().value("Hello");
    assert_eq!(kit.props().value, "Hello");
}

/// The display row: the value and the edit affordance.
#[test]
fn the_display_row_holds_the_value_and_the_edit_affordance() {
    let widget = kit().value("Hello").view();
    assert!(!widget.is_editing(), "the field starts closed");
    let row = row_of(&widget);
    assert_eq!(row.children().len(), 2, "the value and the edit affordance");
}

/// The caller's child replaces the value in the display row.
#[test]
fn the_callers_child_replaces_the_value() {
    let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("Custom")) as Box<dyn WidgetAny>
    });
    let widget = kit().value("Hello").child(child).view();
    let row = row_of(&widget);
    let content = row.children().remove(0).build();
    let ops = ops_of(&*content);
    let text = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("the child is painted");
    assert_eq!(text, "Custom", "the caller's child is what is shown");
}

/// The editing row: the field, the tick and the cross.
#[test]
fn the_editing_row_holds_the_field_and_the_two_buttons() {
    let widget = kit().value("Hello").view();
    widget.edit();
    let row = row_of(&widget);
    assert_eq!(row.children().len(), 3, "the field, the tick and the cross");
}

/// The display row shows the value.
#[test]
fn the_display_row_shows_the_value() {
    let widget = kit().value("Hello").view();
    let row = row_of(&widget);
    let content = row.children().remove(0).build();
    let ops = ops_of(&*content);
    let (text, size, _) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the value is painted");
    assert_eq!(text, "Hello");
    assert_eq!(size, 14.0, "v1's display label carried no size of its own");
}

/// The edit affordance is a glyph, painted in `on_surface_variant`.
#[test]
fn the_edit_affordance_is_a_glyph_in_on_surface_variant() {
    let widget = kit().value("Hello").view();
    let row = row_of(&widget);
    let button = row.children().remove(1).build();
    assert!(
        button.as_any().downcast_ref::<Pressable>().is_some(),
        "the edit affordance is a pressable"
    );
    let surface = button.children().remove(0).build();
    let glyph = surface.children().remove(0).build();
    let ops = ops_of(&*glyph);
    let fill = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Path {
                fill: Some(color), ..
            } => Some(*color),
            _ => None,
        })
        .expect("the glyph is painted");
    assert_eq!(
        fill,
        paint(kit().resolved_style().on_surface_variant),
        "v1 drew the edit affordance in `on_surface_variant`"
    );
}

/// The edit affordance opens the field, and the draft takes the value.
#[test]
fn the_edit_affordance_opens_the_field() {
    let widget = kit().value("Hello").view();
    let row = row_of(&widget);
    let mut button = row.children().remove(1).build();

    click(&mut *button);

    assert!(widget.is_editing(), "the field is open");
    assert_eq!(widget.draft(), "Hello", "the draft took the value");
}

/// The tick saves: the value takes the draft and the callback fires.
#[test]
fn the_tick_saves_and_the_cross_cancels() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let sink = seen.clone();
    let widget = kit()
        .value("Hello")
        .on_save(move |value| {
            if let Ok(mut guard) = sink.lock() {
                *guard = value;
            }
        })
        .view();

    widget.edit();
    let row = row_of(&widget);
    let mut input = row.children().remove(0).build();
    let id = allocate(&*input);
    dispatch_any(
        &mut *input,
        id,
        ROW,
        Event::Key(KeyEvent::down(Key::Char('!'))),
    );
    assert_eq!(widget.draft(), "Hello!", "the field holds the edit");

    let row = row_of(&widget);
    let mut save = row.children().remove(1).build();
    click(&mut *save);
    assert_eq!(widget.value(), "Hello!", "the value took the draft");
    assert_eq!(
        *seen.lock().expect("the lock is not poisoned"),
        "Hello!",
        "and the callback fired"
    );
    assert!(!widget.is_editing(), "and the field closed");

    widget.edit();
    {
        let draft = widget.draft_buffer();
        *draft.lock().expect("the lock is not poisoned") = "Throwaway".to_string();
    }
    let row = row_of(&widget);
    let mut cancel = row.children().remove(2).build();
    click(&mut *cancel);
    assert_eq!(widget.draft(), "Hello!", "the draft took the value back");
    assert_eq!(widget.value(), "Hello!", "and the value did not move");
    assert!(!widget.is_editing(), "and the field closed");
}

/// A change of mode asks the host for the other row: the two rows are
/// different subtrees, and 2.0's tree is built once.
#[test]
fn a_change_of_mode_asks_for_the_other_row() {
    let mut widget = kit().value("Hello").view();
    let id = allocate(&widget);

    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    assert!(
        update.children_dirty().is_empty(),
        "nothing moved, so nothing is rebuilt"
    );

    widget.edit();

    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    assert!(
        !update.children_dirty().is_empty(),
        "the mode moved, so the other row is mounted"
    );
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialConfirmEdit::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material confirm-edit read the app-wide signal, not a snapshot"
    );
}
