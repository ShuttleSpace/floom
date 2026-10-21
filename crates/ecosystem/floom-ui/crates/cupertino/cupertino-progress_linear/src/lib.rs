//! Cupertino linear progress, ported to the Floom 2.0 widget API.
//!
//! ## Disagreements preserved in this adapter
//!
//! This kit prefers a **static indeterminate segment**, truncated percentages, and
//! **rounded fill** (both track and bar). The engine defaults give the majority
//! (two kits) rather than Material, so this kit must override in its style.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{FillPlacement, INDETERMINATE_WIDTH_PCT, Indeterminate, LinearProgress, LinearProgressStyle, PercentRounding};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ProgressLinearTokens};
use floom_core_theme::props::progress_linear::ProgressLinearProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_widget_runtime::theme_signal;

fn progress_linear_tokens(theme: &MultiTheme) -> ProgressLinearTokens {
    match theme.component(ComponentId::ProgressLinear) {
        ComponentTokens::ProgressLinear(t) => t,
        _ => unreachable!("ComponentId::ProgressLinear must resolve to ProgressLinearTokens"),
    }
}


/// Cupertino's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoProgressLinearStyle {
    pub design_system: &'static str,
    /// Height of the track. Cupertino calls this `track_height`, where Material
    /// calls the same field `height` — an API difference kept rather than tidied.
    pub track_height: f32,
    pub border_radius: f32,
    /// Cupertino rounds the bar as well as the track, so the fill radius is the
    /// track radius rather than `None`.
    pub fill_radius: f32,
    /// Track colour (`surface_variant`) and bar colour (`primary`).
    pub track_color: Color,
    pub fill_color: Color,
}

pub struct CupertinoProgressLinear {
    props: ProgressLinearProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoProgressLinear {
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

    pub fn resolved_style(&self) -> CupertinoProgressLinearStyle {
        let theme = self.theme.get();
        let tokens = progress_linear_tokens(&theme);
        let height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        CupertinoProgressLinearStyle {
            design_system: "cupertino",
            track_height: height,
            border_radius: tokens.border_radius,
            fill_radius: tokens.border_radius,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
        }
    }

    /// The engine's style, which is the whole translation.
    ///
    /// Cupertino differs from the engine defaults in two places: it rounds the
    /// fill (the majority) and it truncates fractional percentages (the only kit
    /// that does). Both are values, not branches.
    pub fn linear_progress_style(&self) -> LinearProgressStyle {
        let style = self.resolved_style();
        LinearProgressStyle {
            track_color: to_paint(style.track_color),
            fill_color: to_paint(style.fill_color),
            track_height: style.track_height,
            track_radius: style.border_radius,
            fill_radius: Some(style.fill_radius),
            fill_placement: FillPlacement::Absolute,
            percent_rounding: PercentRounding::Truncate,
        }
    }

    /// Cupertino's indeterminate mode: a static segment at the start of the
    /// track, unlike Material's sliding one.
    pub fn indeterminate_mode(&self) -> Indeterminate {
        if self.props.indeterminate {
            Indeterminate::StaticSegment
        } else {
            Indeterminate::None
        }
    }

    /// The bar as an engine widget.
    pub fn view(self) -> LinearProgress {
        LinearProgress::new(self.props.value)
            .indeterminate_mode(self.indeterminate_mode())
            .style(self.linear_progress_style())
    }
}

impl Default for CupertinoProgressLinear {
    fn default() -> Self {
        Self::new()
    }
}
