//! Fluent sparkline, ported to the Floom 2.0 widget API.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::sparkline::SparklineProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const BAR_RADIUS: f32 = 2.0;
const MIN_BAR_HEIGHT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSparklineStyle {
    pub design_system: &'static str,
    pub width: f32,
    pub height: f32,
    pub gap: f32,
    pub bar_width: f32,
    pub min: f32,
    pub max: f32,
    pub range: f32,
    pub bar_color: Color,
    pub fill_opacity: u8,
    /// The token's own bar width. v1 never read it -- see the module note.
    pub token_line_width: f32,
    pub fill: bool,
}

pub struct FluentSparkline {
    props: SparklineProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentSparkline {
    pub fn new() -> Self {
        Self {
            props: SparklineProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn values(mut self, values: Vec<f32>) -> Self {
        self.props.values = values;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.props.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    pub fn fill(mut self, fill: bool) -> Self {
        self.props.fill = fill;
        self
    }

    pub fn line_width(mut self, lw: f32) -> Self {
        self.props.line_width = lw;
        self
    }

    pub fn props(&self) -> &SparklineProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentSparklineStyle {
        let theme = self.theme.get();
        let ComponentTokens::Sparkline(tokens) = theme.component(ComponentId::Sparkline) else {
            unreachable!("ComponentId::Sparkline must resolve to SparklineTokens")
        };
        let values = &self.props.values;
        let n = values.len();
        let (min, max) = if n == 0 {
            (0.0f32, 1.0f32)
        } else {
            let mn = values.iter().copied().fold(f32::INFINITY, f32::min);
            let mx = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            (mn, mx)
        };
        let range = if (max - min) <= f32::EPSILON {
            1.0
        } else {
            max - min
        };
        let width = self.props.width.unwrap_or(tokens.width);
        let height = self.props.height.unwrap_or(tokens.height);
        let gap = self.props.line_width.max(0.0);
        let bar_width = if n > 0 {
            let total_gap = gap * (n as f32 - 1.0);
            ((width - total_gap) / n as f32).max(1.0)
        } else {
            0.0
        };
        let bar_color = if self.props.fill {
            with_alpha(theme.colors().primary, tokens.fill_opacity)
        } else {
            theme.colors().primary
        };
        FluentSparklineStyle {
            design_system: "fluent",
            width,
            height,
            gap,
            bar_width,
            min,
            max,
            range,
            bar_color,
            fill_opacity: tokens.fill_opacity,
            token_line_width: tokens.line_width,
            fill: self.props.fill,
        }
    }

    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let values = self.props.values.clone();

        let mut bars = Stack::new(Axis::Horizontal)
            .gap(style.gap)
            .align(Align::Start, Align::Center)
            .request(LayoutRequest::new().width(style.width).height(style.height));

        for value in values {
            let h = (((value - style.min) / style.range) * style.height).max(MIN_BAR_HEIGHT);
            let bar_width = style.bar_width;
            let bar_color = to_paint(style.bar_color);
            bars = bars.push(move || {
                Box::new(
                    Surface::new()
                        .request(LayoutRequest::new().width(bar_width).height(h))
                        .background(bar_color)
                        .radius(BAR_RADIUS),
                )
            });
        }

        bars
    }
}

impl Default for FluentSparkline {
    fn default() -> Self {
        Self::new()
    }
}
