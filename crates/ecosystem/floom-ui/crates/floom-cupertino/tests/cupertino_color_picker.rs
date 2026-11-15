#![cfg(feature = "color_picker")]
//! The ported Cupertino colour picker -- a square of colour, a hue strip, an
//! alpha strip, a preview and a hex field.

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, Point, Rect, Widget, WidgetAny};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_cupertino::widgets::color_picker::{CupertinoColorPicker, CupertinoColorPickerStyle};

/// The picker's box, in the tests' coordinate space.
const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 264.0,
    y1: 320.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoColorPicker {
    CupertinoColorPicker::new().with_theme(theme())
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
///
/// The widget is behind a `WidgetAny`, because that is what a slot hands back,
/// and upcasting `&dyn WidgetAny` to `&dyn Widget` is not something Rust can
/// do yet -- so the methods are called through the `WidgetAny` handle, which
/// finds them on the supertrait.
fn ops_of(widget: &dyn WidgetAny) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The rounded fills of a widget's paint, in order.
fn fills(ops: &[PaintOp]) -> Vec<Color> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::RoundedRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

/// The text runs of a widget's paint, in order.
fn texts(ops: &[PaintOp]) -> Vec<(String, f32, Color)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .collect()
}

/// The picker's stack: the square, the strips and the preview row.
fn stack_of(surface: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    let mut children = surface.children();
    assert_eq!(children.len(), 1, "the surface holds one column");
    children.remove(0).build()
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoColorPickerStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::ColorPicker) {
            floom_core_theme::component_tokens::ComponentTokens::ColorPicker(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.sv_width, tokens.sv_width, "the token's square width");
    assert_eq!(
        style.sv_height, tokens.sv_height,
        "the token's square height"
    );
    assert_eq!(
        style.slider_height, tokens.slider_height,
        "the token's strip height"
    );
    assert_eq!(style.thumb_size, tokens.thumb_size, "the token's thumb");
    assert_eq!(style.radius, tokens.radius, "the token's radius");
    assert_eq!(style.surface, theme().colors().surface, "the picker's fill");
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the picker's border"
    );
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the field's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

/// The numbers v1 wrote as literals are reported, because v1's paint used them
/// and v1's style did not report them.
#[test]
fn the_literals_v1_wrote_are_reported() {
    let style = kit().resolved_style();
    assert_eq!(style.padding, 12.0, "v1's padding");
    assert_eq!(style.border_width, 1.0, "v1's border");
    assert_eq!(style.border_alpha, 80, "v1's border alpha");
    assert_eq!(style.margin_top, 12.0, "v1's margin above each strip");
    assert_eq!(style.row_gap, 12.0, "v1's gap beside the field");
    assert_eq!(style.preview_size, 40.0, "v1's preview");
    assert_eq!(style.hex_width, 120.0, "v1's field");
    assert_eq!(style.hex_font_size, 14.0, "v1's field text");
    assert_eq!(style.thumb_border_width, 2.0, "v1's thumb border");
}

// --- the pixels -----------------------------------------------------------

/// The picker is a surface in v1's colours.
#[test]
fn the_picker_is_painted_in_v1_s_colours() {
    let style = kit().resolved_style();
    let surface = kit().view();
    let ops = ops_of(&surface);
    // The picker's fill is the surface colour.
    assert_eq!(fills(&ops).first(), Some(&paint(style.surface)), "the fill");
    // And its border is v1's: `outline` at 80, one point thick. A border is
    // a stroke and not a fill, so it is not among the fills.
    let (width, colour) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Path {
                stroke: Some((width, colour)),
                ..
            } => Some((*width, *colour)),
            _ => None,
        })
        .expect("the picker is outlined");
    assert_eq!(
        width,
        f64::from(style.border_width),
        "v1's border is one point thick"
    );
    assert_eq!(
        colour,
        paint(floom_core_theme::tokens::color_ext::with_alpha(
            style.outline,
            style.border_alpha
        )),
        "in v1's border colour"
    );
}

/// The square is painted with v1's three layers: the hue underneath, the white
/// ramp across it and the black ramp down it.
#[test]
fn the_square_is_painted_with_v1_s_three_layers() {
    let surface = kit().view();
    let stack = stack_of(&surface);
    let mut children = stack.children();
    let square = children.remove(0).build();
    let ops = ops_of(&*square);

    assert!(
        ops.iter().any(|op| matches!(op, PaintOp::Rect { .. })),
        "the hue underneath: {ops:?}"
    );
    assert_eq!(
        ops.iter()
            .filter(|op| matches!(op, PaintOp::BrushRect { .. }))
            .count(),
        2,
        "the white ramp and the black ramp: {ops:?}"
    );
}

/// The preview is painted in the chosen colour.
#[test]
fn the_preview_is_painted_in_the_chosen_colour() {
    let style = kit().resolved_style();
    let chosen = floom_core_theme::tokens::color::Color::rgb(12, 34, 56);
    let surface = kit().value(chosen).view();
    let stack = stack_of(&surface);
    let mut children = stack.children();
    // The square, the hue strip, the alpha strip, then the preview row.
    let bottom = children.remove(3).build();
    let mut bottom_children = bottom.children();
    let swatch = bottom_children.remove(0).build();
    let ops = ops_of(&*swatch);
    let fills = fills(&ops);
    assert_eq!(fills.first(), Some(&paint(style.outline)), "the border");
    assert_eq!(fills.get(1), Some(&paint(chosen)), "the chosen colour");
}

/// The hex field spells the colour out.
#[test]
fn the_hex_field_spells_the_colour_out() {
    let style = kit().resolved_style();
    let surface = kit()
        .value(floom_core_theme::tokens::color::Color::rgb(
            0x21, 0x96, 0xf3,
        ))
        .view();
    let stack = stack_of(&surface);
    let mut children = stack.children();
    let bottom = children.remove(3).build();
    let mut bottom_children = bottom.children();
    let field = bottom_children.remove(1).build();
    let ops = ops_of(&*field);
    let texts = texts(&ops);
    assert_eq!(texts.len(), 1, "the field's text: {texts:?}");
    assert_eq!(texts[0].0, "#2196f3", "v1's hex spelling");
    assert_eq!(texts[0].1, style.hex_font_size);
}

// --- the structure --------------------------------------------------------

/// The alpha strip is only there when the picker was asked for one.
#[test]
fn the_alpha_strip_is_only_there_when_it_was_asked_for() {
    let with = kit().show_alpha(true).view();
    let stack = stack_of(&with);
    assert_eq!(
        stack.children().len(),
        4,
        "the square, the hue strip, the alpha strip and the preview row"
    );

    let without = kit().show_alpha(false).view();
    let stack = stack_of(&without);
    assert_eq!(
        stack.children().len(),
        3,
        "the square, the hue strip and the preview row"
    );
}

/// A drag in the square reports the colour it landed on.
#[test]
fn a_drag_in_the_square_reports_the_colour_it_landed_on() {
    let reported: Arc<Mutex<Vec<floom_core_theme::tokens::color::Color>>> =
        Arc::new(Mutex::new(Vec::new()));
    let sink = reported.clone();
    let surface = kit()
        .on_change(move |color| {
            sink.lock().expect("the sink").push(color);
        })
        .view();
    let stack = stack_of(&surface);
    let mut children = stack.children();
    let mut square = children.remove(0).build();

    let mut create = AllocatingCreateContext::new();
    let id: ElementId = square.create(&mut create);
    let mut event = TestEventContext::new(
        id,
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 240.0,
            y1: 160.0,
        },
        Event::Pointer(PointerEvent::primary_down(Point::new(120.0, 80.0))),
    );
    square.event(&mut event, id);

    let reported = reported.lock().expect("the sink");
    assert_eq!(reported.len(), 1, "the colour is reported once");
    // The middle of the square is half saturated and half bright.
    let color = reported[0];
    assert!(
        color.r < 33 || color.g < 150 || color.b < 243,
        "the middle of the square is not the pure hue: {color:?}"
    );
}

/// The props report what v1 reported.
#[test]
fn the_props_report_the_alpha_flag() {
    let picker = kit().show_alpha(false);
    assert!(!picker.props().show_alpha);
    assert!(kit().show_alpha(true).props().show_alpha);
}

/// The theme: v1's Cupertino colour picker took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    assert_ne!(
        CupertinoColorPicker::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino colour picker took a snapshot, not the app-wide signal"
    );
}
