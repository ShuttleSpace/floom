//! Cupertino tooltip, ported to the Floom 2.0 widget API.
//!
//! A tooltip is a hint that appears over a control while the pointer rests on
//! it: a rounded dark box with one line of text, under the control.
//!
//! ## What the engine grew
//!
//! **A `Tooltip` composite.** v1's tooltip was a box in an overlay, placed by
//! hand from the window origin and the trigger's height, and shown or hidden by
//! a signal. 2.0's host owns the overlay layers, so the bubble is asked of the
//! host and placed against the control's own box -- and the bubble's *width* is
//! the text's width, which is what makes it a bubble and not a bar. Both of
//! those are the engine's business, so the widget is the engine's.
//!
//! ## What this kit answers
//!
//! * the bubble's fill: `on_surface` at 230;
//! * the text's ink: `surface`;
//! * the padding, the corner radius and the text size: the tokens;
//! * the gap below the control: none. v1's Cupertino bubble had no offset,
//!   and neither does this one.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **Where the bubble is.** v1's Cupertino bubble was in an overlay with *no
//!   position at all*, so it landed at the overlay's origin rather than under
//!   the control. The port places it under the control, like the other two: a
//!   bubble that appears in the corner is a bug, not a style.
//! * **The trigger is a slot, not a view.** v1's `child(..)` took an
//!   `impl IntoView`; 2.0's takes a [`Child`], because a 2.0 widget is mounted
//!   by its parent rather than returned.
//! * **The trigger's centring.** v1 centred the control and the bubble
//!   together in one box; 2.0's tooltip leaves the control's own box alone.
//!
//! ## The one disagreement inside v1
//!
//! v1's style reported the bubble's fill as the *opaque* `on_surface`, while
//! the paint used `on_surface` at 230. The port reports the colour it paints,
//! because a style that disagrees with the pixels is a trap -- and the pixels
//! are v1's.

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Tooltip, TooltipStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tooltip::TooltipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const DESIGN_SYSTEM: &str = "cupertino";

/// The tooltip's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTooltipStyle {
    pub design_system: &'static str,
    /// The bubble's fill.
    pub bubble_color: Color,
    /// The text's ink.
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
    /// The gap between the control and the bubble. A v1 literal, and this kit's
    /// is zero.
    pub gap: f32,
}

/// Cupertino's tooltip.
pub struct CupertinoTooltip {
    props: TooltipProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// The control the pointer rests on.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// What the tooltip says.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.props.text = text.into();
        self
    }

    pub fn props(&self) -> &TooltipProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoTooltipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tooltip(tokens) = theme.component(ComponentId::Tooltip) else {
            unreachable!("ComponentId::Tooltip must resolve to TooltipTokens")
        };
        CupertinoTooltipStyle {
            design_system: DESIGN_SYSTEM,
            // v1's paint: `with_alpha(t.colors().on_surface, 230)`.
            bubble_color: with_alpha(theme.colors().on_surface, 230),
            text_color: theme.colors().surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            // v1's Cupertino bubble had no offset.
            gap: 0.0,
        }
    }

    /// The tooltip: the control, the text, and the bubble that follows them.
    pub fn view(self) -> Tooltip {
        let style = self.resolved_style();
        let mut tooltip = Tooltip::new()
            .text(self.props.text.clone())
            .style(TooltipStyle {
                background: to_paint(style.bubble_color),
                foreground: to_paint(style.text_color),
                padding: f64::from(style.padding),
                radius: style.border_radius,
                font_size: style.font_size,
                gap: f64::from(style.gap),
            });
        if let Some(child) = self.child {
            tooltip = tooltip.trigger(child);
        }
        tooltip
    }
}

impl Default for CupertinoTooltip {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
