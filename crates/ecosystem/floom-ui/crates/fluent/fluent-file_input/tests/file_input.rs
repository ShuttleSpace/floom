//! The ported Fluent file input -- a bordered trigger box, a trigger label and

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_file_input::{FluentFileInput, FluentFileInputStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 80.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentFileInput {
    FluentFileInput::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn box_of(kit: FluentFileInput) -> Box<dyn floom_core::WidgetAny> {
    let pressable = kit.view();
    pressable.children().remove(0).build()
}

fn row_of(kit: FluentFileInput) -> Box<dyn floom_core::WidgetAny> {
    let surface = box_of(kit);
    surface.children().remove(0).build()
}

fn text_of(kit: FluentFileInput, index: usize) -> (String, f32, Color) {
    let row = row_of(kit);
    let text = row.children().remove(index).build();
    let ops = ops_of(&*text);
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a file input paints its texts")
}

fn box_painted(kit: FluentFileInput) -> (Option<Color>, Option<(f64, Color)>) {
    let surface = box_of(kit);
    let ops = ops_of(&*surface);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    let rim = ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((width, color)),
            ..
        } => Some((*width, *color)),
        _ => None,
    });
    (fill, rim)
}

fn tap(pressable: &mut floom_widgets::Pressable) {
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        pressable.event(&mut cx, id);
    }
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentFileInputStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::FileInput)
    {
        floom_core_theme::component_tokens::ComponentTokens::FileInput(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(
        style.padding_horizontal, tokens.padding_horizontal,
        "the token's padding"
    );
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(
        style.trigger_color,
        theme().colors().primary,
        "the label's ink"
    );
    assert_eq!(
        style.path_color,
        theme().colors().on_surface,
        "the readout's ink"
    );
    assert_eq!(
        style.empty_path_color,
        theme().colors().on_surface_variant,
        "the empty readout's ink"
    );
    assert_eq!(
        style.border_color,
        theme().colors().outline,
        "the box's rim"
    );
    assert_eq!(style.background, theme().colors().surface, "the box's fill");
}

#[test]
fn the_numbers_v1_wrote_as_literals_are_literals_here() {
    let style = kit().resolved_style();
    assert_eq!(style.gap, 12.0, "v1's `.gap(12.0)`");
    assert_eq!(style.border_width, 1.0, "v1's `.border(1.0)`");
    assert_eq!(
        style.empty_path_label, "No file chosen",
        "v1's empty readout"
    );
}

#[test]
fn the_trigger_label_takes_the_theme_s_label_weight() {
    let style = kit().resolved_style();
    assert_eq!(
        style.trigger_font_weight,
        floom_types::FontWeight::Medium,
        "the theme's `label.large` weight, in the engine's vocabulary"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_box_is_painted_with_v1_s_fill_and_rim() {
    let style = kit().resolved_style();
    let (fill, rim) = box_painted(kit());
    assert_eq!(fill, Some(paint(style.background)), "the box's fill");
    assert_eq!(
        rim,
        Some((f64::from(style.border_width), paint(style.border_color))),
        "the box's rim"
    );
}

#[test]
fn the_disabled_box_is_painted_with_v1_s_disabled_fill_and_rim() {
    let style = kit().resolved_style();
    let (fill, rim) = box_painted(kit().disabled(true));
    assert_eq!(
        fill,
        Some(paint(style.disabled_background)),
        "the disabled fill"
    );
    assert_eq!(
        rim,
        Some((
            f64::from(style.border_width),
            paint(style.disabled_border_color)
        )),
        "the disabled rim"
    );
    assert_eq!(
        style.disabled_background.a, 4,
        "v1's disabled fill was `on_surface` at 4"
    );
    assert_eq!(
        style.disabled_border_color.a, 38,
        "v1's disabled rim was `on_surface` at 38"
    );
}

#[test]
fn the_trigger_label_is_painted_in_primary_at_the_token_s_size() {
    let style = kit().resolved_style();
    let (text, size, ink) = text_of(kit(), 0);
    assert_eq!(text, "Upload", "v1's default label");
    assert_eq!(size, style.font_size, "the token's font size");
    assert_eq!(ink, paint(style.trigger_color), "v1's `primary`");
}

#[test]
fn the_readout_says_no_file_chosen_when_there_is_no_path() {
    let style = kit().resolved_style();
    let (text, size, ink) = text_of(kit(), 1);
    assert_eq!(text, "No file chosen", "v1's empty readout");
    assert_eq!(size, style.font_size, "the token's font size");
    assert_eq!(
        ink,
        paint(style.empty_path_color),
        "v1's `on_surface_variant` while there is no path"
    );
}

#[test]
fn the_readout_says_the_path_when_there_is_one() {
    let style = kit().resolved_style();
    let kit = kit();
    kit.set_path("/tmp/report.pdf");
    let (text, size, ink) = text_of(kit, 1);
    assert_eq!(text, "/tmp/report.pdf", "the path the host wrote");
    assert_eq!(size, style.font_size, "the token's font size");
    assert_eq!(ink, paint(style.path_color), "v1's `on_surface`");
}

#[test]
fn the_box_is_as_tall_as_the_token_says() {
    let style = kit().resolved_style();
    let surface = box_of(kit());
    let request = surface
        .layout_request()
        .expect("the box asks for its own space");
    assert_eq!(
        request.height,
        Some(floom_core::Length::Px(style.height)),
        "the token's height"
    );
}

#[test]
fn the_callers_content_is_mounted_after_the_readout() {
    let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("12 KB"))
    });
    let row = row_of(kit().child(child));
    assert_eq!(
        row.children().len(),
        3,
        "the label, the readout and the caller's own content"
    );
    let last = row.children().remove(2).build();
    let ops = ops_of(&*last);
    let text = ops.iter().find_map(|op| match op {
        PaintOp::Text { text, .. } => Some(text.clone()),
        _ => None,
    });
    assert_eq!(text.as_deref(), Some("12 KB"), "the caller's content");
}

#[test]
fn without_the_slot_the_row_is_v1_s() {
    let row = row_of(kit());
    assert_eq!(row.children().len(), 2, "v1's label and readout");
}

#[test]
fn the_trigger_reports_the_current_path_when_it_is_clicked() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let mut pressable = kit()
        .on_change(move |path| s.lock().unwrap().push(path))
        .view();

    tap(&mut pressable);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[None],
        "v1's `cb(path.get())` with no path chosen"
    );
}

#[test]
fn the_trigger_reports_the_path_the_host_wrote() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let mut kit = kit();
    kit.set_path("/tmp/report.pdf");
    let mut pressable = kit
        .on_change(move |path| s.lock().unwrap().push(path))
        .view();
    tap(&mut pressable);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[Some("/tmp/report.pdf".to_string())],
        "the path as it is at the moment of the click"
    );
}

#[test]
fn a_disabled_trigger_does_not_answer() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let mut pressable = kit()
        .disabled(true)
        .on_change(move |path| s.lock().unwrap().push(path))
        .view();

    tap(&mut pressable);
    assert!(
        seen.lock().unwrap().is_empty(),
        "v1's action was guarded by `if !disabled`"
    );
}

#[test]
fn the_props_report_the_shape_and_not_the_callback() {
    let kit = kit().label("Pick").accept(".pdf").multiple(true);
    assert_eq!(kit.props().label, "Pick");
    assert_eq!(kit.props().accept, ".pdf");
    assert!(kit.props().multiple);
    assert!(kit.props().on_change.is_none());
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentFileInput::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent file input took a snapshot, not the app-wide signal"
    );
}
