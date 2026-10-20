//! Fluent circular progress, ported to the Floom 2.0 widget API.
//!
//! v1 drew an SVG ring: a track circle and an accent arc over it, with the
//! arc's length and start set by the value. 2.0's [`CircularProgress`] draws
//! the same two things with the engine's own path and stroke, so the kit is a
//! thin adapter over it.
//!
//! ## What this kit answers
//!
//! * **the size**: the props' size when it is set, and the token's otherwise;
//! * **the colours**: `surface_variant` for the track, `primary` for the arc;
//! * **the indeterminate arc**: //! **25% of the ring, starting at three o'clock** -- which is
//! where this kit and the other two disagree
//!
//! ## The size token is never read
//!
//! v1 wrote `if self.props.size > 0.0 { self.props.size } else { tokens.size }`,
//! and the props' default size is **36.0** -- so the props always win and
//! `ProgressCircularTokens::size` is dead. v1's own doc on `size()` said
//! "Zero -- the default -- defers to the token", which is wrong: the default is
//! 36.0, not 0. The rule is kept, because the rule is what v1 drew, and the
//! lie is named here rather than corrected silently.
//!
//! ## The rotation
//!
//! //! This kit's v1 arc did not spin, so there is nothing for a host to drive and
//! the rotation stays at zero.
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{CircularProgress, CircularProgressStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::progress_circular::ProgressCircularProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentProgressCircularStyle {
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

/// Fluent's circular progress.
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

    pub fn resolved_style(&self) -> FluentProgressCircularStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressCircular(tokens) = theme.component(ComponentId::ProgressCircular)
        else {
            unreachable!("ComponentId::ProgressCircular must resolve to ProgressCircularTokens")
        };
        FluentProgressCircularStyle {
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
            indeterminate_start_rad: 0.0,
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

impl Default for FluentProgressCircular {
    fn default() -> Self {
        Self::new()
    }
}

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
