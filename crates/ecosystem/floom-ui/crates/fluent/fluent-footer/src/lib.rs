//! Fluent footer, ported to the Floom 2.0 widget API.
//!
//! The one thing this kit answers differently from Material is the design
//! system it resolves against: the bar is 48dp tall with 16dp of
//! padding, from the Fluent tokens, and every colour comes from the
//! Fluent palette.
//!
//! The theme is a `MultiTheme` snapshot rather than the app-wide signal that
//! Material's kit uses. That is v1's difference, kept: v1 held a snapshot here
//! and the live signal there.
//!
//! # What changed from v1
//!
//! v1 built a view by composing and styling:
//!
//! ```ignore
//! Container::new(child).style(move |s| {
//!     let mut s = s.width_pct(100.0).height(h).padding(pad).items_center()
//!                .background(surface).color(on_surface);
//!     if bordered { s = s.border_color(outline_variant).border_top(border); }
//!     if elevation > 0 { s = s.box_shadow_blur(6.0)..; }
//!     s
//! })
//! ```
//!
//! Each of those becomes a different thing in 2.0:
//!
//! * the box is a [`Surface`] — a decorated box that holds its child and pads it;
//! * `items_center` is [`Surface::request`]'s `main_align` / `cross_align`, not a
//!   property of the fill, because the child is what gets centred;
//! * `border_top` is [`Surface::border_edges`] with [`Edges::TOP`], the one thing
//!   v1's four-sided stroke could not say;
//! * `color(..)` on the box does not exist in 2.0 at all: colour belongs to the
//!   thing that draws, so the content colour is exposed as
//!   [`FluentFooter::content_color`] for the child to use.
//!
//! # The layout registry is kept
//!
//! v1 called `register_layout(LayoutSlot::Footer, height)` while building the
//! view, and `md-main` reads that process-wide signal to inset the content
//! region. Dropping it would leave every footer's content overlapped by the bar,
//! so [`FluentFooter::view`] still publishes — at build time, exactly as v1 did, not
//! reactively. A footer whose height changes after mount does not re-publish,
//! which was already true in v1 and is recorded in
//! [`FluentFooter::publish_layout`] rather than quietly improved.

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Align, Edges, Surface};
use floom_core::{LayoutRequest, WidgetAny};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, FooterTokens};
use floom_core_theme::props::footer::FooterProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::*;

/// Opaque black, the base the elevation shadow tints.
const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// The blur of the elevation shadow. v1 wrote `box_shadow_blur(6.0)
/// .box_shadow_spread(0.0).box_shadow_color(with_alpha(BLACK, 30))` with no
/// offset, so the shadow is a symmetric halo rather than a drop.
const ELEVATION_BLUR: f32 = 6.0;

/// The alpha of that shadow, matching v1's `with_alpha(BLACK, 30)`.
const ELEVATION_ALPHA: u8 = 30;

/// A builder for the footer's content, replayed each time the bar is built.
///
/// An [`std::sync::Arc`] for the reason `md-toolbar`'s is: a 2.0 slot is a
/// replayed `Fn`, so every rebuild needs its own copy to move into it.
pub type ChildBuilder =
    std::sync::Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;


fn footer_tokens(theme: &MultiTheme) -> FooterTokens {
    match theme.component(ComponentId::Footer) {
        ComponentTokens::Footer(t) => t,
        _ => unreachable!("ComponentId::Footer must resolve to FooterTokens"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFooterStyle {
    pub surface: Color,
    /// The colour the *content* should be drawn in.
    ///
    /// A field rather than a property of the box: 2.0 has no `color(..)` on a
    /// decoration, because a fill and a glyph are different objects and only one
    /// of them is text.
    pub content: Color,
    /// The separator colour, present only when `bordered`.
    pub outline: Color,
    /// The props' height, or the token's when the props defer.
    pub height: f32,
    /// The props' padding, or the token's when the props defer.
    pub padding: f32,
    /// The top separator's thickness, from the token.
    pub border: f32,
    pub bordered: bool,
    pub elevated: bool,
}

pub struct FluentFooter {
    props: FooterProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentFooter {
    pub fn new() -> Self {
        Self {
            props: FooterProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    /// Bar height in dp. `None` — the default — defers to the design system's
    /// `FooterTokens::height` (48.0 for Fluent).
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// Horizontal padding. `None` — the default — defers to `FooterTokens::pad`.
    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = Some(pad);
        self
    }

    /// Draw the top separator. On by default: v1's `FooterProps::default()` sets
    /// `bordered: true`.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.props.bordered = bordered;
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    /// Supply the footer's content.
    ///
    /// v1 took `impl IntoView`; 2.0 takes a replayed builder. Callers holding a
    /// widget already write `move || Box::new(..)`.
    pub fn child(
        mut self,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(std::sync::Arc::new(build));
        self
    }

    pub fn props(&self) -> &FooterProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentFooterStyle {
        let theme = self.theme.get();
        let tokens = footer_tokens(&theme);
        FluentFooterStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            outline: theme.colors().outline_variant,
            height: self.props.height.unwrap_or(tokens.height),
            padding: self.props.pad.unwrap_or(tokens.pad),
            border: tokens.border,
            bordered: self.props.bordered,
            elevated: self.props.elevation > 0,
        }
    }

    /// The bar's extent — its height and the full width of its parent — plus the
    /// centring v1 wrote as `items_center()`.
    ///
    /// The alignment lives here rather than on a builder because the *child* is
    /// what gets centred: 2.0's decoration boxes carry a fill and a border, and
    /// the layout of what they hold is declared by the box's own request.
    pub fn request(&self) -> LayoutRequest {
        let style = self.resolved_style();
        LayoutRequest::new()
            .height(style.height)
            .width_pct(100.0)
            .main_align(Align::Center)
            .cross_align(Align::Center)
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    /// The top separator as `(thickness, colour)`, or `None` when unbordered.
    ///
    /// [`Edges::TOP`] specifically: v1 wrote `border_top`, and a footer's rule
    /// along its top edge is not the same drawing as a box outlined all round.
    pub fn border(&self) -> Option<(f32, PaintColor)> {
        let style = self.resolved_style();
        style
            .bordered
            .then_some((style.border, to_paint(style.outline)))
    }

    /// The edges the separator is drawn on, for a caller drawing it itself.
    pub fn border_edges(&self) -> Edges {
        if self.resolved_style().bordered {
            Edges::TOP
        } else {
            Edges::NONE
        }
    }

    pub fn shadow(&self) -> Option<Shadow> {
        self.resolved_style().elevated.then(|| Shadow {
            color: to_paint(with_alpha(BLACK, ELEVATION_ALPHA)),
            blur: f64::from(ELEVATION_BLUR),
            offset_x: 0.0,
            offset_y: 0.0,
            spread: 0.0,
        })
    }

    /// The colour the content should be drawn in.
    ///
    /// v1 put this on the box as `color(..)`, which in 2.0 has nowhere to live:
    /// a `Surface` paints a fill, and only the child knows it is text.
    pub fn content_color(&self) -> PaintColor {
        to_paint(self.resolved_style().content)
    }

    /// Report this footer's height to the process-wide layout registry.
    ///
    /// v1 did this inside `view()` and `md-main` reads it to inset the content
    /// region, so dropping it overlaps the content with the bar. Deliberately a
    /// build-time side effect and not reactive: that is what v1 did, and a footer
    /// whose height changes after mount does not re-publish. Making it reactive
    /// would mean holding the theme signal inside a `Send + Sync` slot closure,
    /// which 2.0's slots cannot do.
    pub fn publish_layout(&self) {
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::Footer,
            self.resolved_style().height,
        );
    }

    /// The whole bar: the filled box, its padding, its extent, its separator, its
    /// elevation, and the caller's content inside it.
    pub fn view(self) -> Surface {
        self.publish_layout();

        let style = self.resolved_style();
        let request = self.request();
        let padding = self.padding();
        let border = self.border();
        let edges = self.border_edges();
        let shadow = self.shadow();
        let surface_colour = to_paint(style.surface);
        // Last, because it moves.
        let child = self.child;

        let mut surface = Surface::new()
            .request(request)
            .padding(padding)
            .background(surface_colour);
        if let Some((width, colour)) = border {
            surface = surface.border_edges(width, colour, edges);
        }
        if let Some(shadow) = shadow {
            surface = surface.shadow(shadow);
        }
        match child {
            Some(child) => surface.push(move || child()),
            // v1 substituted an empty view. A `Surface` with no children is the
            // same answer without a node, and leaves the bar's centring
            // uncontested.
            None => surface,
        }
    }
}

impl Default for FluentFooter {
    fn default() -> Self {
        Self::new()
    }
}
