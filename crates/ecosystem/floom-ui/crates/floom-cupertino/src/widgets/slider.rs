//! Cupertino's slider, ported to the 2.0 widget API.
//!

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::{Slider, SliderStyle};

/// v1's three shadow numbers, kept as constants because none of them is a
/// token and all three are this kit's own.
const THUMB_SHADOW_BLUR: f32 = 3.0;
const THUMB_SHADOW_SPREAD: f32 = 0.0;
const THUMB_SHADOW_ALPHA: u8 = 60;

/// Cupertino's slider.
pub struct CupertinoSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSlider {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            props: SliderProps::default(),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.props.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.props.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.props.step = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &SliderProps {
        &self.props
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Cupertino's appearance, as the engine's `SliderStyle`.
    ///
    /// Two of v1's numbers are this kit's own: the `surface` thumb and the
    /// drop shadow. The track is an undimmed `surface_variant` here, where
    /// Material's is `on_surface_variant` at 97 percent.
    ///
    /// There is no disabled branch here, and that is a decision rather than an
    /// oversight. v1's disabled look came from the style layer's generic dim --
    /// `set_disabled(true)`, which halved a colour's alpha -- not from this
    /// kit's tokens, and the engine deliberately dropped that rule: a hard-coded
    /// dim a kit cannot override is the problem `SliderStyle` exists to fix.
    /// Material shows what a kit's *own* disabled answer looks like (token
    /// alphas, 120 and 60), and this kit had none. Inventing one here would be
    /// a design decision this port has no licence to make, so a disabled
    /// Cupertino slider paints the enabled palette.
    pub fn resolved_style(&self) -> SliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        let colors = theme.colors();

        SliderStyle {
            // --- from `SliderTokens`, as v1 read them ---
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,

            track_color: to_paint(colors.surface_variant),
            filled_color: to_paint(colors.primary),
            thumb_color: to_paint(colors.surface),

            // v1 had no hover or press colour of its own for any of the three
            // boxes. `None` means "no variant of my own", so the base colour is
            // painted -- the faithful answer, and a different answer from the
            // engine's default, which carries its own hover and press blues.
            track_hovered_color: None,
            filled_hovered_color: None,
            filled_pressed_color: None,

            // No halo: v1 drew none, and the shadow below is what this kit uses
            halo_color: None,
            halo_size: 0.0,
            halo_size_dragging: 0.0,

            shadow_blur: THUMB_SHADOW_BLUR,
            shadow_spread: THUMB_SHADOW_SPREAD,
            shadow_color: to_paint(with_alpha(Color::BLACK, THUMB_SHADOW_ALPHA)),
        }
    }

    /// The slider, ready to mount.
    pub fn view(self) -> Slider {
        let style = self.resolved_style();

        // `SliderProps` speaks `f32` because it is the theme crate's shared
        // struct, still shared with the kits that have not been ported, while
        // the engine's slider is `f64`. So the widening happens here, once.
        let mut slider = Slider::new(
            f64::from(self.props.value),
            f64::from(self.props.min),
            f64::from(self.props.max),
        )
        .step(f64::from(self.props.step))
        .disabled(self.props.disabled)
        .style(style);

        // And the narrowing back, on the way out.
        let on_change = self.props.on_change;
        if let Some(on_change) = on_change {
            slider = slider.on_change(move |value| on_change(value as f32));
        }

        slider
    }
}

impl Default for CupertinoSlider {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine's `SliderStyle`, under this kit's old name.
///
/// v1's own `CupertinoSliderStyle` carried `design_system`, `track_color`,
/// `thumb_color` and three geometry numbers, and nothing ever read it -- v1's
/// `view()` resolved the colours from the theme directly, which is how the
/// `primary` in that dead struct came to disagree with the `surface` it
/// painted. The alias keeps the name resolvable for callers without
/// pretending a struct still exists. The `design_system` field has no
/// successor: the engine's style names no design system, because it is not one.
pub type CupertinoSliderStyle = SliderStyle;
