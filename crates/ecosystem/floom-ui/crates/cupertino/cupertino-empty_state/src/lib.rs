//! Cupertino empty state, ported to the Floom 2.0 widget API.
//!
//! An empty state is what a list shows when it has nothing to show: an icon, a
//! title, a description, and whatever the caller wants under them, all
//! centred in a surface.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for the box, a vertical `Stack` for
//! the column, and `Text` for the three lines. v1's three kits were the same
//! code under three names, so the shape stays in the kits -- there is nothing
//! here that a widget could own and the vocabulary does not already say.
//!
//! ## What this kit answers
//!
//! * the icon's size, the title's and the description's sizes, the gap and the
//!   column's maximum width: the tokens;
//! * the icon's and the description's ink: `on_surface_variant`; the title's:
//!   `on_surface`, in medium weight;
//! * the padding around the column: 24, a v1 literal.
//!
//! ## The one field the port's style adds
//!
//! v1's style reported the ink and the sizes but not the surface the column
//! sits on, though the paint used it. The port's style reports it, because a
//! style that does not say what the widget draws is not the widget's output.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The column's ink.** v1 set `.color(on_surface)` on the column, which is
//!   what the caller's own child inherited. 2.0's `Text` carries its own
//!   colour, so a caller's child keeps the colour it was given.
//! * **The child is a slot, not a view.** v1's `child(..)` took an
//!   `impl IntoView`; 2.0's takes a [`Child`], because a 2.0 widget is mounted
//!   by its parent rather than returned.
//! * **The accessors.** v1's empty states had no `props()`, `theme()` or
//!   `with_theme()`. The port gives them all three, because a kit that cannot
//!   be given a theme cannot be tested against one.

use floom_core::Child;
use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{FontWeight, Insets};
use floom_widgets::{Align, Axis, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::empty_state::EmptyStateProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;


/// The padding around the column, at every side. A v1 literal.
const PADDING: f64 = 24.0;

/// The empty state's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoEmptyStateStyle {
    pub design_system: &'static str,
    /// The title's ink.
    pub on_surface: Color,
    /// The icon's and the description's ink.
    pub on_surface_variant: Color,
    /// The surface the column sits on.
    pub surface: Color,
    pub icon_size: f32,
    pub title_font_size: f32,
    pub description_font_size: f32,
    pub gap: f32,
    pub max_width: f32,
}

/// Cupertino's empty state.
pub struct CupertinoEmptyState {
    props: EmptyStateProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoEmptyState {
    pub fn new() -> Self {
        Self {
            props: EmptyStateProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// The glyph above the title. Empty means no icon.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.props.icon = icon.into();
        self
    }

    /// The headline.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    /// The supporting description. Empty means no description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.props.description = description.into();
        self
    }

    /// The action or illustration under the text.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn props(&self) -> &EmptyStateProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoEmptyStateStyle {
        let theme = self.theme.get();
        let ComponentTokens::EmptyState(tokens) = theme.component(ComponentId::EmptyState) else {
            unreachable!("ComponentId::EmptyState must resolve to EmptyStateTokens")
        };
        CupertinoEmptyStateStyle {
            design_system: "cupertino",
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            surface: theme.colors().surface,
            icon_size: tokens.icon_size,
            title_font_size: tokens.title_font_size,
            description_font_size: tokens.description_font_size,
            gap: tokens.gap,
            max_width: tokens.max_width,
        }
    }

    /// The empty state: the surface, and the column inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let mut column = Stack::new(Axis::Vertical)
            .gap(style.gap)
            // v1's `.items_center().justify_center()`.
            .align(Align::Center, Align::Center)
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .max_width(style.max_width),
            );

        if !self.props.icon.is_empty() {
            let icon = self.props.icon.clone();
            let size = style.icon_size;
            let color = to_paint(style.on_surface_variant);
            column = column.push(move || {
                Box::new(Text::new(icon.clone()).size(size).color(color))
            });
        }

        let title = self.props.title.clone();
        let title_size = style.title_font_size;
        let title_color = to_paint(style.on_surface);
        column = column.push(move || {
            Box::new(
                Text::new(title.clone())
                    .size(title_size)
                    .color(title_color)
                    // v1's title was medium weight.
                    .weight(FontWeight::Medium),
            )
        });

        if !self.props.description.is_empty() {
            let description = self.props.description.clone();
            let size = style.description_font_size;
            let color = to_paint(style.on_surface_variant);
            column = column.push(move || {
                Box::new(Text::new(description.clone()).size(size).color(color))
            });
        }

        if let Some(child) = self.child {
            column = column.child(child);
        }

        let surface = to_paint(style.surface);
        Surface::new()
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .padding(Insets {
                left: PADDING,
                right: PADDING,
                top: PADDING,
                bottom: PADDING,
            })
            .background(surface)
            .push(move || Box::new(column.clone()))
    }
}

impl Default for CupertinoEmptyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
