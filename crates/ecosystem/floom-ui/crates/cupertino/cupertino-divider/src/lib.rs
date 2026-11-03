//! cupertino divider, ported to the Floom 2.0 widget API.
//!

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Divider, RuleAlignment};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, DividerTokens};
use floom_core_theme::props::divider::{DividerOrientation, DividerProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "cupertino";

fn divider_tokens(theme: &MultiTheme) -> DividerTokens {
    match theme.component(ComponentId::Divider) {
        ComponentTokens::Divider(t) => t,
        _ => unreachable!("ComponentId::Divider must resolve to DividerTokens"),
    }
}

/// Cupertino's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoDividerStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub thickness: f32,
    pub inset: f32,
    pub orientation: DividerOrientation,
}

pub struct CupertinoDivider {
    props: DividerProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoDivider {
    pub fn new() -> Self {
        Self {
            props: DividerProps::default(),
            // A **snapshot**, unlike Material's live `theme_signal()`. Built
            // from the process-global design system at construction, so a later
            // global theme switch does not repaint this divider.
            // That difference is v1's and is preserved rather than corrected:
            // two of the three kits behaved this way, so "correcting" it would
            // change two systems' behaviour on the strength of one.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    pub fn orientation(mut self, orientation: DividerOrientation) -> Self {
        self.props.orientation = orientation;
        self
    }

    pub fn inset(mut self, inset: f32) -> Self {
        self.props.inset = inset;
        self
    }

    pub fn props(&self) -> &DividerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoDividerStyle {
        let theme = self.theme.get();
        let tokens = divider_tokens(&theme);
        CupertinoDividerStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().outline_variant,
            thickness: tokens.thickness,
            inset: self.props.inset,
            orientation: self.props.orientation,
        }
    }

    /// The engine's style, which is the whole translation.
    ///
    /// Four disagreements in seven lines, and only one of them is a value:
    /// `RuleAlignment::Start` is this kit's entire styling opinion. The colour,
    /// the thickness and the inset are what all three kits already agreed on.
    pub fn divider(&self) -> Divider {
        let style = self.resolved_style();
        Divider::new()
            .color(to_paint(style.color))
            .thickness(style.thickness as f64)
            .axis(match style.orientation {
                DividerOrientation::Horizontal => floom_core::Axis::Horizontal,
                DividerOrientation::Vertical => floom_core::Axis::Vertical,
            })
            .indent(style.inset as f64)
            .alignment(RuleAlignment::Start)
    }
}

impl Default for CupertinoDivider {
    fn default() -> Self {
        Self::new()
    }
}
