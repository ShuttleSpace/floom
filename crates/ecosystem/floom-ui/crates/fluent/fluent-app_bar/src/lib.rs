//! Fluent app bar, ported to the Floom 2.0 widget API.
//!
//! An app bar is the strip at the top of a screen: an optional leading slot, a
//! title, and optional trailing actions.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a `Surface` for the bar, a horizontal `Stack` for
//! the row inside it, and `Text` for the title. v1's three kits were the same
//! code under three names, and there is nothing here that a widget could own
//! and the vocabulary does not already say.
//!
//! ## What this kit answers
//!
//! * the bar's fill: `surface`; the title's ink: `on_surface`;
//! * the bar's height, its padding and the title's size: the tokens -- unless
//!   the caller gave a height, which wins, as it did in v1;
//! * the bar's elevation: the caller's, and the shadow it draws.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The leading and the actions are slots, not views.** v1's `leading(..)`
//!   took an `impl IntoView` and its `actions(..)` took a `Vec<AnyView>`;
//!   2.0's take [`Child`] slots, because a 2.0 widget is mounted by its
//!   parent rather than returned.
//! * **The title's `flex_grow(1)`.** v1 let the title's *box* take the room
//!   between the leading slot and the actions. 2.0's `Text` is a leaf and
//!   cannot grow, so the room is a flexible spacer instead: the title sits at
//!   the left and the actions at the right, which is what the growing box
//!   looked like.

use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Shadow;
use floom_widgets::{Align, Axis, Spacer, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::app_bar::AppBarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;


/// The gap between the trailing actions. A v1 literal.
const ACTION_GAP: f32 = 8.0;

/// The app bar's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentAppBarStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub bar_color: Color,
    /// The title's ink.
    pub title_color: Color,
    /// The actions' ink.
    pub action_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub title_font_size: f32,
    pub elevation: u8,
}

/// Fluent's app bar.
pub struct FluentAppBar {
    props: AppBarProps,
    /// The slot at the left, if any.
    leading: Option<Child>,
    /// The slots at the right.
    actions: Vec<Child>,
    theme: RwSignal<MultiTheme>,
}

impl FluentAppBar {
    pub fn new() -> Self {
        Self {
            props: AppBarProps::default(),
            leading: None,
            actions: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    /// The bar's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    /// The bar's elevation: the depth of the shadow it draws.
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    /// The bar's height, in place of the token's.
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    /// The slot at the left, if any.
    pub fn leading(mut self, leading: Child) -> Self {
        self.leading = Some(leading);
        self
    }

    /// The slots at the right, laid out left to right.
    pub fn actions(mut self, actions: Vec<Child>) -> Self {
        self.actions = actions;
        self
    }

    pub fn props(&self) -> &AppBarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentAppBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::AppBar(tokens) = theme.component(ComponentId::AppBar) else {
            unreachable!("ComponentId::AppBar must resolve to AppBarTokens")
        };
        FluentAppBarStyle {
            design_system: "fluent",
            bar_color: theme.colors().surface,
            title_color: theme.colors().on_surface,
            action_color: theme.colors().on_surface_variant,
            // v1: the caller's height wins, and the token's is the fallback.
            height: if self.props.height > 0.0 {
                self.props.height
            } else {
                tokens.height
            },
            padding_horizontal: tokens.padding_horizontal,
            title_font_size: tokens.title_font_size,
            elevation: self.props.elevation,
        }
    }

    /// The bar: the row of leading, title and actions, on the bar's own fill.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let title = self.props.title.clone();
        let title_size = style.title_font_size;
        let title_color = to_paint(style.title_color);
        let action_color = to_paint(style.action_color);
        let padding = style.padding_horizontal;

        let mut row = Stack::new(Axis::Horizontal)
            // v1's row: `items_center()`.
            .align(Align::Center, Align::Center)
            // v1's row gap was the padding.
            .gap(padding)
            .request(
                LayoutRequest::new().padding_horiz(f64::from(padding)),
            );

        if let Some(leading) = self.leading {
            row = row.child(leading);
        }

        row = row.push({
            let title = title.clone();
            let title_color = title_color;
            move || {
                Box::new(Text::new(title.clone()).size(title_size).color(title_color))
            }
        });

        if !self.actions.is_empty() {
            // The room between the title and the actions: v1 let the title's box
            // grow, and 2.0's `Text` cannot.
            row = row.push(|| Box::new(Spacer::flexible()));
            let mut actions = Stack::new(Axis::Horizontal)
                // v1's actions: `gap(8.0).items_center().justify_end()`.
                .gap(ACTION_GAP)
                .align(Align::End, Align::Center);
            for action in self.actions {
                actions = actions.child(action);
            }
            row = row.push(move || Box::new(actions.clone()));
        }

        let bar_color = to_paint(style.bar_color);
        let elevation = style.elevation;
        let height = style.height;

        // v1 registered the bar's height with the theme's layout slots, so an
        // app could ask for it.
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::AppBar,
            height,
        );

        let mut bar = Surface::new()
            .request(LayoutRequest::new().height(height))
            .background(bar_color);
        // v1's elevation: a shadow whose blur grows with the elevation.
        if elevation > 0 {
            bar = bar.shadow(Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 30)),
                blur: 2.0 + 2.0 * f64::from(elevation),
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            });
        }
        bar.push(move || Box::new(row.clone()))
    }
}

impl Default for FluentAppBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
