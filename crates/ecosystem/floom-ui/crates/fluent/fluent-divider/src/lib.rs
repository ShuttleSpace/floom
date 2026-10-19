//! fluent divider, ported to the Floom 2.0 widget API.
//!
//! ## What this kit uniquely wants
//!
//! **A rule flush with the top of its box.** `RuleAlignment::Start`, not the
//! engine's default `Center`.
//!
//! v1 wrote `s.height(thickness).width_full().background(color)` for a
//! horizontal divider and said nothing about the vertical axis. In a container
//! taller than the rule, "nothing said" means the rule lands on the container's
//! top edge. The engine centres instead — it was written before this crate was
//! ported, and centring is the better default for a rule between two columns.
//!
//! So this kit asks for `Start` explicitly rather than inheriting a default that
//! would silently move its rule. `start_reproduces_what_v1_wrote` in
//! `floom-widgets/tests/primitives.rs` is the assertion that makes that a port
//! rather than a preference.
//!
//! ## The other difference, which is not about the divider
//!
//! This kit captures a theme **snapshot** at construction
//! (`MultiTheme::new_for(..)`); Material reads the **live** theme signal
//! (`theme_signal()`). So a Material divider follows a global theme switch and
//! this one does not. That difference lives in `new()`, not in the widget,
//! because it is a property of where the theme came from rather than of how a
//! rule is drawn.
//!
//! ## What the three kits agree on
//!
//! Everything else, exactly. Measured: with naming and the `design_system`
//! field removed, the three v1 files' `resolved_style()` and `view()` bodies are
//! **byte-identical**. One token (`thickness: 1.0`), one colour role
//! (`outline_variant`), one inset rule. There is no second style parameter to
//! set, and inventing one would be inventing a disagreement.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Divider, RuleAlignment};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, DividerTokens};
use floom_core_theme::props::divider::{DividerOrientation, DividerProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "fluent";

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn divider_tokens(theme: &MultiTheme) -> DividerTokens {
    match theme.component(ComponentId::Divider) {
        ComponentTokens::Divider(t) => t,
        _ => unreachable!("ComponentId::Divider must resolve to DividerTokens"),
    }
}

/// Fluent's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentDividerStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub thickness: f32,
    pub inset: f32,
    pub orientation: DividerOrientation,
}

pub struct FluentDivider {
    props: DividerProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentDivider {
    pub fn new() -> Self {
        Self {
            props: DividerProps::default(),
            // A **snapshot**, unlike Material's live `theme_signal()`. Built
            // from the process-global design system at construction, so a later
            // global theme switch does not repaint this divider.
            //
            // That difference is v1's and is preserved rather than corrected:
            // two of the three kits behaved this way, so "correcting" it would
            // change two systems' behaviour on the strength of one.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
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

    pub fn resolved_style(&self) -> FluentDividerStyle {
        let theme = self.theme.get();
        let tokens = divider_tokens(&theme);
        FluentDividerStyle {
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

impl Default for FluentDivider {
    fn default() -> Self {
        Self::new()
    }
}