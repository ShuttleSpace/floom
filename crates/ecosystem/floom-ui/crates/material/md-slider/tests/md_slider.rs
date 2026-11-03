
use std::cell::Cell;
use std::rc::Rc;

use floom_core::{ElementId, Key, NamedKey, Rect, Widget};
use floom_types::Color as PaintColor;
use floom_types::KeyEvent;
use floom_ui_composites::{Slider, SliderStyle};
use floom_widgets::testkit::{dispatch, AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_md_slider::MdSlider;

/// The box the slider is laid out in: wide enough to drag, tall enough for the
/// thumb to sit on the track.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

/// Every style is read through an explicit theme rather than the app-wide
/// signal, so these tests cannot be moved by another test's global state.
fn style() -> SliderStyle {
    MdSlider::new().with_theme(theme()).resolved_style()
}

fn disabled_style() -> SliderStyle {
    MdSlider::new()
        .with_theme(theme())
        .disabled(true)
        .resolved_style()
}

fn alpha_of(c: PaintColor) -> u8 {
    c.a
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Paint the slider into a recorder and hand back both the ops and the snapshot
/// text, so a failing assertion can say what was actually drawn.
fn paint(slider: &Slider, bounds: Rect) -> (Vec<PaintOp>, String) {
    let id = allocate(slider);
    let mut recorder = SceneRecorder::new(bounds);
    slider.paint(&mut recorder, id);
    recorder.finish();
    (recorder.ops().to_vec(), recorder.snapshot())
}

/// The colour every filled op carries, in the order `paint` drew them.
fn fills_of(ops: &[PaintOp]) -> Vec<PaintColor> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::Rect { color, .. } => Some(*color),
            PaintOp::RoundedRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}


#[test]
fn the_track_is_on_surface_variant_at_97() {
    let binding = theme();
    let colors = binding.colors();
    let track = style().track_color;
    assert_eq!(track.r, colors.on_surface_variant.r);
    assert_eq!(track.g, colors.on_surface_variant.g);
    assert_eq!(track.b, colors.on_surface_variant.b);
    assert_eq!(alpha_of(track), 97);
}

/// Material's fill and thumb are both `primary`, undimmed. Cupertino's thumb is
/// not, and that is the entire difference between the two kits' thumbs.
#[test]
fn the_fill_and_the_thumb_are_both_primary() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();
    assert_eq!(style.filled_color.r, colors.primary.r);
    assert_eq!(style.filled_color.g, colors.primary.g);
    assert_eq!(style.filled_color.b, colors.primary.b);
    assert_eq!(alpha_of(style.filled_color), 255);
    assert_eq!(style.thumb_color, style.filled_color);
    assert_ne!(
        style.thumb_color.r, colors.surface.r,
        "control: Cupertino's thumb is `surface`, so this is not a default"
    );
}

/// Material is the only kit of the three with a halo: 40px, `primary` at 20
/// percent. The other two write `None`, which paints no halo at all.
#[test]
fn the_thumb_carries_a_forty_pixel_halo_at_twenty_percent() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();
    let halo = style.halo_color.expect("Material draws a halo");
    assert_eq!(halo.r, colors.primary.r);
    assert_eq!(halo.g, colors.primary.g);
    assert_eq!(halo.b, colors.primary.b);
    assert_eq!(alpha_of(halo), 20);
    assert_eq!(style.halo_size, 40.0);
    assert_eq!(style.halo_size_dragging, 40.0);

    assert!(
        SliderStyle::default().halo_color.is_none(),
        "control: the engine default draws no halo, so `Some` is this kit's"
    );
}

#[test]
fn a_disabled_slider_dims_to_the_tokens() {
    let binding = theme();
    let colors = binding.colors();
    let on = style();
    let off = disabled_style();

    assert_eq!(off.track_color.r, colors.on_surface_variant.r);
    assert_eq!(alpha_of(off.track_color), 60);
    assert_ne!(off.track_color, on.track_color, "the track dims");

    assert_eq!(off.filled_color.r, colors.primary.r);
    assert_eq!(alpha_of(off.filled_color), 120);
    assert_ne!(off.filled_color, on.filled_color, "the fill dims");

    assert_eq!(
        off.thumb_color, off.filled_color,
        "fill and thumb dim alike"
    );
    assert_ne!(off.thumb_color, on.thumb_color, "the thumb dims");

    assert_eq!(off.track_height, on.track_height);
    assert_eq!(off.thumb_size, on.thumb_size);
    assert_eq!(off.thumb_radius, on.thumb_radius);
}

#[test]
fn a_disabled_slider_is_not_the_enabled_one() {
    assert_ne!(disabled_style(), style());
}

#[test]
fn none_of_the_interaction_variants_are_this_kits_own() {
    let style = style();
    assert!(style.track_hovered_color.is_none());
    assert!(style.filled_hovered_color.is_none());
    assert!(style.filled_pressed_color.is_none());

    let default = SliderStyle::default();
    assert!(
        default.track_hovered_color.is_some()
            && default.filled_hovered_color.is_some()
            && default.filled_pressed_color.is_some(),
        "control: the engine default carries a variant for each, so `None` here means this kit declined them rather than inheriting them"
    );
}

#[test]
fn the_thumb_carries_no_drop_shadow() {
    let style = style();
    assert_eq!(
        style.shadow_blur, 0.0,
        "0 blur is how the style says 'none'"
    );
    assert_eq!(alpha_of(style.shadow_color), 0);
}


#[test]
fn the_geometry_is_the_slider_tokens() {
    let tokens = match theme().component(ComponentId::Slider) {
        ComponentTokens::Slider(t) => t,
        _ => unreachable!(),
    };
    let style = style();
    assert_eq!(style.track_height, tokens.track_height);
    assert_eq!(style.thumb_size, tokens.thumb_size);
    assert_eq!(style.thumb_radius, tokens.thumb_radius);
}

#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let material_theme = theme();
    let cupertino_theme = MultiTheme::new_for(DesignSystem::Cupertino);
    let fluent_theme = MultiTheme::new_for(DesignSystem::Fluent);
    let material = material_theme.colors();
    let cupertino = cupertino_theme.colors();
    let fluent = fluent_theme.colors();
    assert_eq!(cupertino.surface_variant, material.surface_variant);
    assert_eq!(fluent.surface_variant, material.surface_variant);
    assert_eq!(cupertino.primary, material.primary);

    let tokens = match fluent_theme.component(ComponentId::Slider) {
        ComponentTokens::Slider(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(tokens.track_height, style().track_height);
}


/// The style the kit resolved is the style the engine paints: the recorded
/// track, fill, halo and thumb carry this kit's colours, in `paint`'s order.
#[test]
fn the_mounted_slider_paints_this_kits_four_shapes() {
    let style = style();
    let slider = MdSlider::new()
        .with_theme(theme())
        .min(0.0)
        .max(100.0)
        .value(50.0)
        .view();

    let (ops, snapshot) = paint(&slider, ROW);
    assert_eq!(
        fills_of(&ops),
        vec![
            style.track_color,
            style.filled_color,
            style.halo_color.expect("a halo"),
            style.thumb_color,
        ],
        "track, fill, halo, thumb:\n{snapshot}"
    );
}

#[test]
fn mounting_carries_the_props_as_f64() {
    let slider = MdSlider::new()
        .with_theme(theme())
        .min(10.0)
        .max(20.0)
        .step(0.5)
        .value(15.25)
        .view();
    assert_eq!(slider.value(), 15.25);
    assert!(
        (slider.fraction() - 0.525).abs() < 1e-9,
        "15.25 is 52.5 percent of 10..20, got {}",
        slider.fraction()
    );

    let clamped = MdSlider::new()
        .with_theme(theme())
        .min(0.0)
        .max(100.0)
        .value(500.0)
        .view();
    assert_eq!(clamped.value(), 100.0, "the engine clamps on construction");
}

/// The callback narrows the engine's `f64` back to the `f32` that
/// `SliderProps` declares, and the value it reports is the engine's, stepped.
#[test]
fn the_callback_reports_the_engines_value_narrowed_to_f32() {
    let seen = Rc::new(Cell::new(f32::NAN));
    let sink = seen.clone();
    let mut slider: Slider = MdSlider::new()
        .with_theme(theme())
        .min(0.0)
        .max(1.0)
        .step(0.1)
        .on_change(move |v| sink.set(v))
        .view();

    slider.set_focused(true);
    let id = allocate(&slider);
    assert!(dispatch(
        &mut slider,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::ArrowRight)).into()
    ));

    assert!(
        (slider.value() - 0.1).abs() < 1e-6,
        "engine value is the widened f32 step, got {:?}",
        slider.value()
    );
    assert_eq!(
        seen.get(),
        slider.value() as f32,
        "the kit reported the engine's f64 narrowed straight back to f32, so the round trip is lossless against what the props declared"
    );
}

#[test]
fn a_disabled_slider_is_disabled_in_both_places() {
    let enabled = MdSlider::new().with_theme(theme()).view();
    assert!(!enabled.interaction().disabled);

    let disabled = MdSlider::new().with_theme(theme()).disabled(true).view();
    assert!(disabled.interaction().disabled);

    let (ops, _) = paint(&disabled, ROW);
    let rectangles: Vec<PaintColor> = ops
        .iter()
        .filter_map(|op| match op {
            PaintOp::Rect { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    assert_eq!(
        rectangles,
        vec![disabled_style().track_color, disabled_style().filled_color],
        "the disabled track and fill, not the enabled ones"
    );
}

#[test]
fn mounting_reports_a_height_of_this_kits_thumb() {
    let style = style();
    let slider = MdSlider::new().with_theme(theme()).view();
    let request = slider.layout_request().expect("a slider asks for a size");
    assert_eq!(
        request.height.expect("a height").px().expect("in px"),
        style.thumb_size.max(style.track_height)
    );
}
