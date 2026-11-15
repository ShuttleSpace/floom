//! Cupertino circular progress, ported to the Floom 2.0 widget API.
//!
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{CircularProgress, CircularProgressStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::progress_circular::ProgressCircularProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoProgressCircularStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub fill_color: Color,
    pub size: f32,
    pub stroke_width: f32,
    /// The fraction of the ring the indeterminate arc covers.
    pub indeterminate_fraction: f64,
    /// Where the indeterminate arc starts, in radians.
    pub indeterminate_start_rad: f64,
}

/// Cupertino's circular progress.
pub struct CupertinoProgressCircular {
    props: ProgressCircularProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoProgressCircular {
    pub fn new() -> Self {
        Self {
            props: ProgressCircularProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// Progress percentage, in `0.0..=100.0`.
    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    /// Ignore the value and draw the indeterminate arc.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.props.indeterminate = indeterminate;
        self
    }

    /// The ring's diameter in dp. Zero -- the default -- defers to the token.
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

    pub fn resolved_style(&self) -> CupertinoProgressCircularStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressCircular(tokens) =
            theme.component(ComponentId::ProgressCircular)
        else {
            unreachable!("ComponentId::ProgressCircular must resolve to ProgressCircularTokens")
        };
        CupertinoProgressCircularStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
            // v1: the props' size when it is set, the token's otherwise.
            size: if self.props.size > 0.0 {
                self.props.size
            } else {
                tokens.size
            },
            stroke_width: tokens.stroke_width,
            indeterminate_fraction: 0.25,
            indeterminate_start_rad: -1.5707963267948966,
        }
    }

    /// The ring, ready to mount.
    pub fn view(self) -> CircularProgress {
        let style = self.resolved_style();
        let mut progress = if self.props.indeterminate {
            CircularProgress::indeterminate()
        } else {
            // v1's value is a percentage; the engine's is a fraction.
            CircularProgress::determinate((self.props.value as f64 / 100.0).clamp(0.0, 1.0))
        };
        progress = progress
            .size(f64::from(style.size))
            .stroke_width(f64::from(style.stroke_width))
            .track_color(to_paint(style.track_color))
            .fill_color(to_paint(style.fill_color))
            .style(CircularProgressStyle {
                indeterminate_fraction: style.indeterminate_fraction,
                // v1 rotated the determinate arc to twelve o'clock in all three.
                start_rad: -std::f64::consts::FRAC_PI_2,
                indeterminate_start_rad: style.indeterminate_start_rad,
            });
        progress
    }
}

impl Default for CupertinoProgressCircular {
    fn default() -> Self {
        Self::new()
    }
}
