//! Fluent circular progress, ported to the Floom 2.0 widget API.
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{CircularProgress, CircularProgressStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::progress_circular::ProgressCircularProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentProgressCircularStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub fill_color: Color,
    pub size: f32,
    pub stroke_width: f32,
    pub indeterminate_fraction: f64,
    pub indeterminate_start_rad: f64,
}

pub struct FluentProgressCircular {
    props: ProgressCircularProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentProgressCircular {
    pub fn new() -> Self {
        Self {
            props: ProgressCircularProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.props.indeterminate = indeterminate;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = size;
        self
    }

    pub fn props(&self) -> &ProgressCircularProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentProgressCircularStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressCircular(tokens) =
            theme.component(ComponentId::ProgressCircular)
        else {
            unreachable!("ComponentId::ProgressCircular must resolve to ProgressCircularTokens")
        };
        FluentProgressCircularStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
            size: if self.props.size > 0.0 {
                self.props.size
            } else {
                tokens.size
            },
            stroke_width: tokens.stroke_width,
            indeterminate_fraction: 0.25,
            indeterminate_start_rad: 0.0,
        }
    }

    pub fn view(self) -> CircularProgress {
        let style = self.resolved_style();
        let mut progress = if self.props.indeterminate {
            CircularProgress::indeterminate()
        } else {
            CircularProgress::determinate((self.props.value as f64 / 100.0).clamp(0.0, 1.0))
        };
        progress = progress
            .size(f64::from(style.size))
            .stroke_width(f64::from(style.stroke_width))
            .track_color(to_paint(style.track_color))
            .fill_color(to_paint(style.fill_color))
            .style(CircularProgressStyle {
                indeterminate_fraction: style.indeterminate_fraction,
                start_rad: -std::f64::consts::FRAC_PI_2,
                indeterminate_start_rad: style.indeterminate_start_rad,
            });
        progress
    }
}

impl Default for FluentProgressCircular {
    fn default() -> Self {
        Self::new()
    }
}
