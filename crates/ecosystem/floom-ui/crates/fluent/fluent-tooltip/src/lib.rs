//! Fluent tooltip, ported to the Floom 2.0 widget API.
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
//! * the gap below the control: none. v1's Fluent bubble had no offset, and
//!   neither does this one.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The bubble's own position.** v1 placed the bubble by hand, from the
//!   window origin and the trigger's height; 2.0's host places it against the
//!   control's box. The same place, reached differently.
//! * **The trigger is a slot, not a view.** v1's `child(..)` took an
//!   `impl IntoView`; 2.0's takes a [`Child`], because a 2.0 widget is mounted
//!   by its parent rather than returned.
//! * **The trigger's centring.** v1's Fluent outer box had no alignment at
//!   all; 2.0's tooltip leaves the control's own box alone.
//!
//! ## The one thing this kit already agreed with
//!
//! v1's Fluent style reported the bubble's fill as `on_surface` at 230 -- the
//! same colour the paint used. The other two kits reported the opaque
//! `on_surface` and painted the translucent one; see their notes.

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Tooltip, TooltipStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tooltip::TooltipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;

const DESIGN_SYSTEM: &str = "fluent";

/// The tooltip's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentTooltipStyle {
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

/// Fluent's tooltip.
pub struct FluentTooltip {
    props: TooltipProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl FluentTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
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

    pub fn resolved_style(&self) -> FluentTooltipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tooltip(tokens) = theme.component(ComponentId::Tooltip) else {
            unreachable!("ComponentId::Tooltip must resolve to TooltipTokens")
        };
        FluentTooltipStyle {
            design_system: DESIGN_SYSTEM,
            // v1's style and v1's paint agreed here, and this is that colour.
            bubble_color: with_alpha(theme.colors().on_surface, 230),
            text_color: theme.colors().surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            // v1's Fluent bubble had no offset.
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

impl Default for FluentTooltip {
    fn default() -> Self {
        Self::new()
    }
}

