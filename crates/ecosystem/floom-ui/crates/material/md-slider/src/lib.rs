
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::{Slider, SliderStyle};
use floom_widget_runtime::theme_signal;

/// v1 drew the halo 40px across in both states, so the two sizes are one number.
const HALO_SIZE: f32 = 40.0;
const HALO_ALPHA: u8 = 20;

/// Material's slider.
pub struct MdSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
}

impl MdSlider {
    pub fn new() -> Self {
        Self {
            props: SliderProps::default(),
            theme: theme_signal(),
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> SliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        let colors = theme.colors();
        let primary = colors.primary;

        let (track, filled, thumb) = if self.props.disabled {
            (
                to_paint(with_alpha(colors.on_surface_variant, 60)),
                to_paint(with_alpha(primary, 120)),
                to_paint(with_alpha(primary, 120)),
            )
        } else {
            (
                to_paint(with_alpha(colors.on_surface_variant, 97)),
                to_paint(primary),
                to_paint(primary),
            )
        };

        SliderStyle {
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,

            track_color: track,
            filled_color: filled,
            thumb_color: thumb,

            track_hovered_color: None,
            filled_hovered_color: None,
            filled_pressed_color: None,

            halo_color: Some(to_paint(with_alpha(primary, HALO_ALPHA))),
            halo_size: HALO_SIZE,
            halo_size_dragging: HALO_SIZE,

            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_color: PaintColor::TRANSPARENT,
        }
    }

    /// The slider, ready to mount.
    pub fn view(self) -> Slider {
        let style = self.resolved_style();

        let mut slider = Slider::new(
            f64::from(self.props.value),
            f64::from(self.props.min),
            f64::from(self.props.max),
        )
        .step(f64::from(self.props.step))
        .disabled(self.props.disabled)
        .style(style);

        let on_change = self.props.on_change;
        if let Some(on_change) = on_change {
            slider = slider.on_change(move |value| on_change(value as f32));
        }

        slider
    }
}

impl Default for MdSlider {
    fn default() -> Self {
        Self::new()
    }
}

pub type MdSliderStyle = SliderStyle;
