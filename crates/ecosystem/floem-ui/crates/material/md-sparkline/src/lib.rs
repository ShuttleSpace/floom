use floem::prelude::*;
use floem::view::AnyView;
use floem::views::{Decorators, Empty, h_stack_from_iter, Stack};
use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::sparkline::SparklineProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

/// Material 3 sparkline: a compact bar chart rendering a `values` series using
/// theme primary color. With `fill`, bars use a translucent primary.
pub struct MdSparkline {
    props: SparklineProps,
    theme: RwSignal<MultiTheme>,
}

impl MdSparkline {
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

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let values = self.props.values.clone();
        let width0 = self.props.width;
        let height0 = self.props.height;
        let fill = self.props.fill;
        let gap = self.props.line_width.max(0.0);

        // Resolve default size from per-DS SparklineTokens when the caller
        // didn't override it, so the widget follows runtime design-system switch.
        let base = match theme.get().component(ComponentId::Sparkline) {
            ComponentTokens::Sparkline(tk) => tk,
            _ => unreachable!(),
        };
        let base_w = width0.unwrap_or(base.width);
        let base_h = height0.unwrap_or(base.height);

        let n = values.len();
        let (min, max) = if n == 0 {
            (0.0f32, 1.0f32)
        } else {
            let mn = values.iter().cloned().fold(f32::INFINITY, f32::min);
            let mx = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            (mn, mx)
        };
        let range = if (max - min) <= f32::EPSILON {
            1.0
        } else {
            max - min
        };
        let bar_w = if n > 0 {
            let total_gap = gap * (n as f32 - 1.0);
            ((base_w - total_gap) / n as f32).max(1.0)
        } else {
            0.0
        };
        let avail_h = base_h;

        let bars: Vec<AnyView> = values
            .iter()
            .map(|v| {
                let h = ((v - min) / range) * avail_h;
                let h = h.max(2.0);
                Container::new(Empty::new())
                    .style(move |s| {
                        let t = theme.get();
                        let tk = match t.component(ComponentId::Sparkline) {
                            ComponentTokens::Sparkline(tk) => tk,
                            _ => unreachable!(),
                        };
                        let c = if fill {
                            with_alpha(t.colors().primary, tk.fill_opacity)
                        } else {
                            t.colors().primary
                        };
                        s.width(bar_w)
                            .height(h)
                            .background(c.to_peniko())
                            .border_radius(2.0)
                    })
                    .into_any()
            })
            .collect();

        Stack::horizontal_from_iter(bars).style(move |s| {
            let tk = match theme.get().component(ComponentId::Sparkline) {
                ComponentTokens::Sparkline(tk) => tk,
                _ => unreachable!(),
            };
            let width = width0.unwrap_or(tk.width);
            let height = height0.unwrap_or(tk.height);
            s.gap(gap).width(width).height(height).items_center()
        })
    }
}

impl Default for MdSparkline {
    fn default() -> Self {
        Self::new()
    }
}
