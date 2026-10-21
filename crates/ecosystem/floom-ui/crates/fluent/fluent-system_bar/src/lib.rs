//! Fluent system bar, ported to the Floom 2.0 widget API.
//!
//! A system bar is the thin status strip at the very top of the application
//! shell -- the window controls, the clock, the battery.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for the bar -- the decorated box
//! that holds the caller's content -- and the process-wide layout registry
//! `fluent-main` reads to inset the content region. There is nothing here
//! that a widget could own and the vocabulary does not already say.
//!
//! ## What this kit answers
//!
//! * the bar's height and its padding: `SystemBarTokens` (Fluent 28 tall
//!   and 8 of padding), unless the caller overrides them;
//! * the bar's fill: `surface_variant`; the dark variant's: `scrim`;
//! * the colour of the bar's content: `on_surface_variant`; the dark
//!   variant's: `on_surface`.
//!
//! ## The theme
//!
//! v1's Fluent system bar took a *snapshot* of its own design system rather
//! than reading the app-wide signal, so a change of theme does not reach it.
//! That difference is v1's, and it is kept.
//!
//! The snapshot names a design system it does not pin: `MultiTheme` reads
//! the *current* design system at read time, so a Fluent bar in an app
//! whose active design system is Material shows Material's colours. That is
//! v1's behaviour too -- the same `MultiTheme` v1 snapshotted -- and it is
//! kept rather than papered over.
//!
//! ## The padding v1 wrote on every side
//!
//! `SystemBarProps::pad` calls itself *horizontal* padding, but v1 wrote
//! `.padding(pad)` on the bar, which insets the content on all four sides.
//! The code is the specification, so the port pads on all four sides too, and
//! the mismatch with the prop's own doc is named here rather than resolved
//! silently in either direction.
//!
//! ## The colour v1 could not hand on
//!
//! v1 ended the bar's style with `.color(fg)`, and that was what the caller's
//! `Label` inherited. 2.0's `Text` carries its own colour and a decoration
//! carries no colour at all, so a bar cannot hand its content an ink. The
//! style still reports it, and [`FluentSystemBar::content_color`] gives it
//! to a caller that wants to paint with it.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The child is a slot, not a view.** v1's `child(..)` took an
//!   `impl IntoView`; 2.0's takes a [`Child`], because a 2.0 widget is
//!   mounted by its parent rather than returned.
//! * **The style does not name the design system.** v1's
//!   `FluentSystemBarStyle` had no `design_system` field, so the three
//!   kits' styles could not be told apart. The port's names it, because a kit
//!   that cannot say which design system it is is a kit that cannot be tested
//!   against one.
//! * **The accessors.** v1's system bar had `props()` and `theme()` but no
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.

use floom_core::Child;
use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;
use floom_widgets::{Align, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::system_bar::SystemBarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;

/// The system bar's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSystemBarStyle {
    /// The design system this kit is. v1's style did not name it.
    pub design_system: &'static str,
    /// The bar's fill.
    pub surface: Color,
    /// The colour the bar's content should be drawn in.
    pub content: Color,
    /// The bar's height: the props', or the token's when the props defer.
    pub height: f32,
    /// The bar's padding: the props', or the token's when the props defer.
    pub pad: f32,
    /// Whether the dark variant is in force.
    pub dark: bool,
}

/// Fluent's system bar: the thin status strip at the top of the shell.
pub struct FluentSystemBar {
    props: SystemBarProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl FluentSystemBar {
    pub fn new() -> Self {
        Self {
            props: SystemBarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    /// The bar's height in dp. `None` -- the default -- defers to the design
    /// system's `SystemBarTokens::height` (Fluent 28).
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// Dark variant: uses a darker, higher-contrast background.
    pub fn dark(mut self, dark: bool) -> Self {
        self.props.dark = dark;
        self
    }

    /// The bar's padding. `None` -- the default -- defers to the design
    /// system's `SystemBarTokens::pad` (Fluent 8).
    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = Some(pad);
        self
    }

    /// Supply the system bar's content.
    ///
    /// v1 took `impl IntoView`; 2.0 takes a mounted slot, because a 2.0 widget
    /// is mounted by its parent rather than returned.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn props(&self) -> &SystemBarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentSystemBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::SystemBar(tokens) = theme.component(ComponentId::SystemBar) else {
            unreachable!("ComponentId::SystemBar must resolve to SystemBarTokens")
        };
        // v1's own choice, in both places it appears: the dark bar is the
        // scrim with the ordinary ink on it, the light one the surface
        // variant with the variant ink.
        let (surface, content) = if self.props.dark {
            (theme.colors().scrim, theme.colors().on_surface)
        } else {
            (
                theme.colors().surface_variant,
                theme.colors().on_surface_variant,
            )
        };
        FluentSystemBarStyle {
            design_system: "fluent",
            surface,
            content,
            height: self.props.height.unwrap_or(tokens.height),
            pad: self.props.pad.unwrap_or(tokens.pad),
            dark: self.props.dark,
        }
    }

    /// The bar's extent -- its height and the full width of its parent -- plus
    /// the centring v1 wrote as `items_center()`.
    ///
    /// The alignment lives on the request rather than on the fill, because it
    /// is the *child* that gets centred.
    pub fn request(&self) -> LayoutRequest {
        let style = self.resolved_style();
        LayoutRequest::new()
            .width_pct(100.0)
            .height(style.height)
            .cross_align(Align::Center)
    }

    /// The inset between the bar's edge and its content.
    ///
    /// On every side, as v1 wrote it -- see the module note on the padding.
    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().pad))
    }

    /// The colour the bar's content should be drawn in.
    ///
    /// v1 put this on the box as `color(..)`, which in 2.0 has nowhere to
    /// live: a `Surface` paints a fill, and only the child knows it is text.
    pub fn content_color(&self) -> PaintColor {
        to_paint(self.resolved_style().content)
    }

    /// Report this bar's height to the process-wide layout registry.
    ///
    /// v1 did this inside `view()` and `fluent-main` reads the signal to
    /// inset the content region, so dropping it would overlap the content with
    /// the bar. Deliberately a build-time side effect and not reactive: that
    /// is what v1 did, and a bar whose height changes after mount does not
    /// re-publish.
    pub fn publish_layout(&self) {
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::SystemBar,
            self.resolved_style().height,
        );
    }

    /// The bar: the filled box, its padding, its extent, and the caller's
    /// content inside it.
    pub fn view(self) -> Surface {
        self.publish_layout();

        let request = self.request();
        let padding = self.padding();
        let fill = to_paint(self.resolved_style().surface);
        let child = self.child;

        let bar = Surface::new()
            .request(request)
            .padding(padding)
            .background(fill);
        match child {
            Some(child) => bar.child(child),
            // v1 substituted an empty view. A `Surface` with no children is the
            // same answer without a node.
            None => bar,
        }
    }
}

impl Default for FluentSystemBar {
    fn default() -> Self {
        Self::new()
    }
}

