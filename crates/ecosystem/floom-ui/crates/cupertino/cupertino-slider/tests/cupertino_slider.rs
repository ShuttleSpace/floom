//! Cupertino's slider as a kit crate: the token translation, and the two numbers
//! this kit decides differently from Material -- a `surface` thumb and a drop
//! shadow on it.
//!
//! The behaviour tests are not repeated here on purpose. Quantisation, the drag
//! and the keyboard walk are the engine's, in
//! `floom-ui-composites/tests/slider.rs`; what is here is what is *this kit's*,
//! plus a control for each claim so a "just take the default" edit cannot
//! quietly pass.

use std::cell::Cell;
use std::rc::Rc;

use floom_core::{ElementId, Key, NamedKey, Rect, Widget};
use floom_types::Color as PaintColor;
use floom_types::KeyEvent;
use floom_ui_composites::{Slider, SliderStyle};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, dispatch};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_cupertino_slider::CupertinoSlider;

/// The box the slider is laid out in.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Cupertino)
}

/// Read through an explicit theme rather than a shared helper, because
/// `MultiTheme::colors()` borrows the theme and a test that reads several
/// colours has to hold the binding itself.
fn style() -> SliderStyle {
    CupertinoSlider::new().with_theme(theme()).resolved_style()
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

// --- the numbers that differ ----------------------------------------------

/// **The thumb is `surface`, not `primary`.**
///
/// This is the one place where v1's `resolved_style` and v1's `view()`
/// disagreed, and following the struct would have changed the kit's
/// appearance: v1's `CupertinoSliderStyle` wrote `thumb_color: primary`, and
/// nothing ever read it -- `view()` drew the thumb from `colors().surface`. So
/// the white disc is what v1 painted, and it is what this port carries.
#[test]
fn the_thumb_is_surface_and_not_primary() {
    let binding = theme();
    let colors = binding.colors();
    let style = style();
    assert_eq!(style.thumb_color.r, colors.surface.r);
    assert_eq!(style.thumb_color.g, colors.surface.g);
    assert_eq!(style.thumb_color.b, colors.surface.b);
    assert_ne!(
        style.thumb_color, style.filled_color,
        "control: the fill is `primary`, so if the thumb were `primary` too this  would be equal and the white disc would be gone"
    );
    assert_ne!(
        SliderStyle::default().thumb_color,
        style.thumb_color,
        "control: the engine default's thumb is white, but its palette is not this kit's"
    );
}

/// The track is an undimmed `surface_variant`, where Material's is
/// `on_surface_variant` at 97 percent. The alpha is the half that differs, so
/// it is asserted rather than left implicit.
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
        alpha_of(PaintColor {
            r: colors.surface_variant.r,
            g: colors.surface_variant.g,
            b: colors.surface_variant.b,
            a: colors.surface_variant.a,
        }),
        "carried through undimmed"
    );
    assert_eq!(
        alpha_of(style.track_color),
        255,
        "control: Material's same field is 97, so the alpha is what tells the  two kits' tracks apart"
    );
}

/// Cupertino is the only kit of the three that separates its thumb from the
/// track with a shadow: blur 3, spread 0, black at 60 percent.
#[test]
fn the_thumb_shadows_black_at_sixty_blurred_three() {
    let style = style();
    assert_eq!(style.shadow_blur, 3.0);
    assert_eq!(style.shadow_spread, 0.0);
    assert_eq!(alpha_of(style.shadow_color), 60);
    assert_eq!(style.shadow_color.r, 0, "black");
    assert_eq!(style.shadow_color.g, 0);
    assert_eq!(style.shadow_color.b, 0);

    assert_eq!(
        SliderStyle::default().shadow_blur,
        0.0,
        "control: the engine default draws no shadow, so 3.0 is this kit's"
    );
}

/// v1 drew no halo here, and did not draw one on the track either -- the shadow
/// is this kit's whole answer to "how is the thumb separated".
#[test]
fn there_is_no_halo() {
    let style = style();
    assert!(style.halo_color.is_none());
    assert_eq!(style.halo_size, 0.0, "no halo means no halo size to speak of");
    assert_eq!(style.halo_size_dragging, 0.0);
}

/// v1 had no hover or press colour of its own for any of the three boxes, so
/// all three variants are `None`. The engine's default is the opposite on all
/// three, which is what makes this a real answer rather than an omission.
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

/// **There is no disabled palette in this kit, and that is pinned on purpose.**
///
/// `SliderStyle` has no disabled field, and v1's disabled look came from the
/// style layer's generic dim -- `set_disabled(true)`, which halved a colour's
/// alpha -- rather than from this kit's tokens. The engine deliberately dropped
/// that rule, because a hard-coded dim a kit cannot override is the problem
/// `SliderStyle` exists to fix. Material shows what a kit's *own* disabled
/// answer looks like (120 and 60); this kit had none. So a disabled Cupertino
/// slider paints the enabled palette, and this test records that as the
/// decision it is rather than leaving it to be discovered.
#[test]
fn a_disabled_slider_paints_the_enabled_palette() {
    let disabled = CupertinoSlider::new()
        .with_theme(theme())
        .disabled(true)
        .resolved_style();
    assert_eq!(
        disabled, style(),
        "no disabled colour of this kit's own, so nothing changes but the flag"
    );
}

// --- what is shared -------------------------------------------------------

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

/// The three design systems resolve the same tokens and the same colours today,
/// so a kit's identity lives in its constants -- see the same test in the
/// Material file for why this one has to exist.
#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let cupertino_theme = theme();
    let material_theme = MultiTheme::new_for(DesignSystem::Material);
    let cupertino = cupertino_theme.colors();
    let material = material_theme.colors();
    assert_eq!(cupertino.surface, material.surface);
    assert_eq!(cupertino.surface_variant, material.surface_variant);
    assert_eq!(cupertino.primary, material.primary);

    let cupertino_tokens = match cupertino_theme.component(ComponentId::Slider) {
        ComponentTokens::Slider(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(cupertino_tokens.thumb_size, style().thumb_size);
}

// --- mounting -------------------------------------------------------------

/// The style the kit resolved is the style the engine paints: track, fill, a
/// shadow, then the white thumb.
#[test]
fn the_mounted_slider_paints_the_shadow_under_the_white_thumb() {
    let style = style();
    let slider = CupertinoSlider::new()
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
        vec![
            style.track_color,
            style.filled_color,
            style.thumb_color,
        ],
        "three filled shapes -- track, fill, thumb -- and no halo:\n{snapshot}"
    );

    let shadows: Vec<&PaintOp> = ops
        .iter()
        .filter(|op| matches!(op, PaintOp::Shadow { .. }))
        .collect();
    assert_eq!(shadows.len(), 1, "exactly one shadow, for the thumb");
    match shadows[0] {
        PaintOp::Shadow { blur, spread, .. } => {
            assert_eq!(*blur, 3.0);
            assert_eq!(*spread, 0.0);
        }
        _ => unreachable!(),
    }
}

/// v1's props were `f32`; the engine's widget is `f64`. The widening happens in
/// `view()`, and this is where it is checked, including the clamp that stops a
/// caller handing the engine a value outside its own range.
#[test]
fn mounting_carries_the_props_as_f64() {
    let slider = CupertinoSlider::new()
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

    let clamped = CupertinoSlider::new()
        .with_theme(theme())
        .min(0.0)
        .max(100.0)
        .value(500.0)
        .view();
    assert_eq!(clamped.value(), 100.0, "the engine clamps on construction");
}

/// The callback narrows the engine's `f64` back to the `f32` that
/// `SliderProps` declares. The tolerance is 1e-6 rather than 1e-9 because the
/// `f32` step widens: 0.1 becomes 0.10000000149011612, and the engine's value is
/// exactly that.
#[test]
fn the_callback_reports_the_engines_value_narrowed_to_f32() {
    let seen = Rc::new(Cell::new(f32::NAN));
    let sink = seen.clone();
    let mut slider: Slider = CupertinoSlider::new()
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

/// The disabled *flag* still reaches the widget even though no colour changes:
/// the engine's `event` returns early for a disabled slider, so this is what
/// stops a disabled one being draggable.
#[test]
fn a_disabled_slider_reaches_the_engine_as_disabled() {
    let enabled = CupertinoSlider::new().with_theme(theme()).view();
    assert!(!enabled.interaction().disabled);

    let disabled = CupertinoSlider::new()
        .with_theme(theme())
        .disabled(true)
        .view();
    assert!(disabled.interaction().disabled);
}

/// A slider's intrinsic height is the thumb, because the thumb is the tallest
/// thing on it.
#[test]
fn mounting_reports_a_height_of_this_kits_thumb() {
    let style = style();
    let slider = CupertinoSlider::new().with_theme(theme()).view();
    let request = slider.layout_request().expect("a slider asks for a size");
    assert_eq!(
        request.height.expect("a height").px().expect("in px"),
        style.thumb_size.max(style.track_height)
    );
}
