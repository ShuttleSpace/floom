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
use floom_widget_runtime::theme_signal;

const BAR_RADIUS: f32 = 2.0;
const MIN_BAR_HEIGHT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSparklineStyle {
    pub design_system: &'static str,
    /// The rendered width: the props', or the token's.
    pub width: f32,
    /// The rendered height: the props', or the token's.
    pub height: f32,
    /// The gap between the bars: the props' `line_width`, not the token's.
    pub gap: f32,
    /// The width of each bar, computed from the width and the gap.
    pub bar_width: f32,
    /// The smallest value in the series. An empty series reports 0.
    pub min: f32,
    /// The largest value in the series. An empty series reports 1.
    pub max: f32,
    /// The scale the values are drawn against.
    pub range: f32,
    /// The bars' colour: `primary`, translucent when `fill` is set.
    pub bar_color: Color,
    /// The alpha of the translucent fill, from the token.
    pub fill_opacity: u8,
    pub token_line_width: f32,
    /// Whether the bars are translucent.
    pub fill: bool,
}

/// Material's sparkline: a compact bar chart of a series of values.
pub struct MaterialSparkline {
    props: SparklineProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialSparkline {
    pub fn new() -> Self {
        Self {
            props: SparklineProps::default(),
            theme: theme_signal(),
        }
    }

    /// Series of values rendered as bars.
    pub fn values(mut self, values: Vec<f32>) -> Self {
        self.props.values = values;
        self
    }

    /// Rendered width in px.
    pub fn width(mut self, width: f32) -> Self {
        self.props.width = Some(width);
        self
    }

    /// Rendered height in px.
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// When true, bars use a translucent fill; otherwise solid.
    pub fn fill(mut self, fill: bool) -> Self {
        self.props.fill = fill;
        self
    }

    /// Gap between bars in px (also the default bar width hint).
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

    pub fn resolved_style(&self) -> MaterialSparklineStyle {
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
        MaterialSparklineStyle {
            design_system: "material",
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

    /// The sparkline: a row of bars, each as tall as its value is of the way
    /// to the largest.
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

impl Default for MaterialSparkline {
    fn default() -> Self {
        Self::new()
    }
}
