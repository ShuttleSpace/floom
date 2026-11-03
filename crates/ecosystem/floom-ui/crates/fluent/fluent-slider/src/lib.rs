//! Fluent's slider, ported to the 2.0 widget API.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_ui_composites::{Slider, SliderStyle};

pub struct FluentSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentSlider {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
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

    pub fn resolved_style(&self) -> SliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        let colors = theme.colors();

        SliderStyle {
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,

            track_color: to_paint(colors.surface_variant),
            filled_color: to_paint(colors.primary),
            thumb_color: to_paint(colors.primary),

            track_hovered_color: None,
            filled_hovered_color: None,
            filled_pressed_color: None,

            halo_color: None,
            halo_size: 0.0,
            halo_size_dragging: 0.0,

            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_color: PaintColor::TRANSPARENT,
        }
    }

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

impl Default for FluentSlider {
    fn default() -> Self {
        Self::new()
    }
}

pub type FluentSliderStyle = SliderStyle;
