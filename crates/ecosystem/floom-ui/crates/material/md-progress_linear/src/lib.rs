
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{
    FillPlacement, Indeterminate, LinearProgress, LinearProgressStyle, PercentRounding,
};

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ProgressLinearTokens};
use floom_core_theme::props::progress_linear::ProgressLinearProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "material";

fn progress_linear_tokens(theme: &MultiTheme) -> ProgressLinearTokens {
    match theme.component(ComponentId::ProgressLinear) {
        ComponentTokens::ProgressLinear(t) => t,
        _ => unreachable!("ComponentId::ProgressLinear must resolve to ProgressLinearTokens"),
    }
}

/// Material's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdProgressLinearStyle {
    pub design_system: &'static str,
    /// Fill of the track behind the bar.
    pub track_color: Color,
    /// Fill of the bar.
    pub fill_color: Color,
    /// Height of the track. **Named `height`,** where Cupertino and Fluent say
    /// `track_height` — an API difference kept rather than tidied away.
    pub height: f32,
    pub border_radius: f32,
    /// Material rounds the track and not the bar, so there is no fill radius.
    /// Not a field: its absence *is* the disagreement.
    pub track_only_radius: bool,
}

pub struct MdProgressLinear {
    props: ProgressLinearProps,
    theme: RwSignal<MultiTheme>,
}

impl MdProgressLinear {
    pub fn new() -> Self {
        Self {
            props: ProgressLinearProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    /// Show a sliding segment instead of a value.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.props.indeterminate = indeterminate;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    pub fn props(&self) -> &ProgressLinearProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdProgressLinearStyle {
        let theme = self.theme.get();
        let tokens = progress_linear_tokens(&theme);
        let height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        MdProgressLinearStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
            height,
            border_radius: tokens.border_radius,
            track_only_radius: true,
        }
    }

    pub fn linear_progress_style(&self) -> LinearProgressStyle {
        let style = self.resolved_style();
        LinearProgressStyle {
            track_color: to_paint(style.track_color),
            fill_color: to_paint(style.fill_color),
            track_height: style.height,
            track_radius: style.border_radius,
            fill_radius: None,
            fill_placement: FillPlacement::Absolute,
            percent_rounding: PercentRounding::Exact,
        }
    }

    pub fn indeterminate_mode(&self) -> Indeterminate {
        if self.props.indeterminate {
            Indeterminate::SlidingSegment
        } else {
            Indeterminate::None
        }
    }

    pub fn view(self) -> LinearProgress {
        LinearProgress::new(self.props.value)
            .indeterminate_mode(self.indeterminate_mode())
            .style(self.linear_progress_style())
    }
}

impl Default for MdProgressLinear {
    fn default() -> Self {
        Self::new()
    }
}
