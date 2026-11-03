//! Fluent's slider as a kit crate: the token translation, and the one thing

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
use floom_widget_fluent_slider::FluentSlider;

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn style() -> SliderStyle {
    FluentSlider::new().with_theme(theme()).resolved_style()
}

fn alpha_of(c: PaintColor) -> u8 {
    c.a
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn paint(slider: &Slider, bounds: Rect) -> (Vec<PaintOp>, String) {
    let id = allocate(slider);
    let mut recorder = SceneRecorder::new(bounds);
    slider.paint(&mut recorder, id);
    recorder.finish();
    (recorder.ops().to_vec(), recorder.snapshot())
}


#[test]
fn the_track_is_an_undimmed_surface_variant() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();
    assert_eq!(style.track_color.r, colors.surface_variant.r);
    assert_eq!(style.track_color.g, colors.surface_variant.g);
    assert_eq!(style.track_color.b, colors.surface_variant.b);
    assert_eq!(
        alpha_of(style.track_color),
        255,
        "control: Material's same field is 97, so the alpha is what tells the two kits' tracks apart"
    );
}

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
fn neither_a_halo_nor_a_drop_shadow() {
    let style = style();
    assert!(style.halo_color.is_none(), "no halo");
    assert_eq!(style.halo_size, 0.0, "no halo means no halo size");
    assert_eq!(style.halo_size_dragging, 0.0);
    assert_eq!(
        style.shadow_blur, 0.0,
        "control: Cupertino's is 3.0, so 0.0 is a decision rather than an unset field"
    );
    assert_eq!(style.shadow_spread, 0.0);
    assert_eq!(alpha_of(style.shadow_color), 0);
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
        "control: the engine default carries a variant for each, so `None` here means this kit declined them"
    );
}

#[test]
fn a_disabled_slider_paints_the_enabled_palette() {
    let disabled = FluentSlider::new()
        .with_theme(theme())
        .disabled(true)
        .resolved_style();
    assert_eq!(
        disabled,
        style(),
        "no disabled colour of this kit's own, so nothing changes but the flag"
    );
}

#[test]
fn the_track_corner_radius_token_is_readable_but_unexpressible() {
    let binding = theme();
    let style = style();
    let radius_sm = binding.radius().sm;

    assert!(
        radius_sm > 0.0,
        "v1 read `radius().sm` for the track, and the token is still there"
    );
    assert_eq!(style.track_height / 2.0, 2.0, "the pills convention");
    assert_ne!(
        radius_sm,
        style.track_height / 2.0,
        "v1's radius token and the pills convention differ, so the track\'s corner radius is a real difference that `SliderStyle` has nowhere to put -- and substituting one for the other would have been visible"
    );
    assert_eq!(
        style.thumb_radius,
        match binding.component(ComponentId::Slider) {
            ComponentTokens::Slider(t) => t.thumb_radius,
            _ => unreachable!(),
        },
        "the style's only radius field is the thumb's, and it carries the token"
    );
}


#[test]
fn the_mounted_slider_paints_exactly_three_shapes() {
    let style = style();
    let slider = FluentSlider::new()
        .with_theme(theme())
        .min(0.0)
        .max(100.0)
        .value(50.0)
        .view();

    let (ops, snapshot) = paint(&slider, ROW);
    let fills: Vec<PaintColor> = ops
        .iter()
        .filter_map(|op| match op {
            PaintOp::Rect { color, .. } => Some(*color),
            PaintOp::RoundedRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        vec![style.track_color, style.filled_color, style.thumb_color],
        "track, fill, thumb -- no halo, no shadow:\n{snapshot}"
    );
    assert!(
        !ops.iter().any(|op| matches!(op, PaintOp::Shadow { .. })),
        "no shadow op at all"
    );
}

#[test]
fn mounting_carries_the_props_as_f64() {
    let slider = FluentSlider::new()
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

    let clamped = FluentSlider::new()
        .with_theme(theme())
        .min(0.0)
        .max(100.0)
        .value(500.0)
        .view();
    assert_eq!(clamped.value(), 100.0, "the engine clamps on construction");
}

#[test]
fn the_callback_reports_the_engines_value_narrowed_to_f32() {
    let seen = Rc::new(Cell::new(f32::NAN));
    let sink = seen.clone();
    let mut slider: Slider = FluentSlider::new()
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
        "the kit reported the engine's f64 narrowed straight back to f32"
    );
}

#[test]
fn a_disabled_slider_reaches_the_engine_as_disabled() {
    let enabled = FluentSlider::new().with_theme(theme()).view();
    assert!(!enabled.interaction().disabled);

    let disabled = FluentSlider::new()
        .with_theme(theme())
        .disabled(true)
        .view();
    assert!(disabled.interaction().disabled);
}

#[test]
fn mounting_reports_a_height_of_this_kits_thumb() {
    let style = style();
    let slider = FluentSlider::new().with_theme(theme()).view();
    let request = slider.layout_request().expect("a slider asks for a size");
    assert_eq!(
        request.height.expect("a height").px().expect("in px"),
        style.thumb_size.max(style.track_height)
    );
}

#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let fluent_theme = theme();
    let material_theme = MultiTheme::new_for(DesignSystem::Material);
    let fluent = fluent_theme.colors();
    let material = material_theme.colors();
    assert_eq!(fluent.surface_variant, material.surface_variant);
    assert_eq!(fluent.primary, material.primary);

    let fluent_tokens = match fluent_theme.component(ComponentId::Slider) {
        ComponentTokens::Slider(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(fluent_tokens.track_height, style().track_height);
}
