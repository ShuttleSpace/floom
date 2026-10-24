//! Material container, ported to the Floom 2.0 widget API.
//!
//! A container is a content wrapper: a full-width box that centres a
//! max-width-capped, padded, surface-coloured column inside it.
//!
//! ## The two boxes
//!
//! v1 nested two `Container`s: an outer one that was full width and centred its
//! child, and an inner one that carried the padding, the fill and the max-width
//! cap. 2.0's `Container` is a *leaf* -- a decorated box with nothing inside it
//! -- so the roles are spelled with the engine's own vocabulary: the outer box is
//! a [`Stack`] that centres on the main axis, and the inner one is a
//! [`Surface`] that fills, pads and caps.
//!
//! ## `fluid`
//!
//! A fluid container drops the max-width cap and spans its parent. v1 wrote
//! `if !fluid { s = s.max_width(max_width) }`, and so does this.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::props::container::ContainerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// A builder for the container's content, replayed each time the box is built.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialContainerStyle {
    pub surface: Color,
    pub content: Color,
    pub max_width: f32,
    pub padding: f32,
    pub fluid: bool,
}

/// Material's container.
pub struct MaterialContainer {
    props: ContainerProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl MaterialContainer {
    pub fn new() -> Self {
        Self {
            props: ContainerProps::default(),
            // The app-wide signal, as v1's was.
            theme: theme_signal(),
            child: None,
        }
    }

    /// Maximum content width in dp.
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.props.max_width = max_width;
        self
    }

    /// Horizontal padding inside the container.
    pub fn padding(mut self, padding: f32) -> Self {
        self.props.padding = padding;
        self
    }

    /// When true the container spans the full width (no max-width cap).
    pub fn fluid(mut self, fluid: bool) -> Self {
        self.props.fluid = fluid;
        self
    }

    /// Supply the container's content.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &ContainerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialContainerStyle {
        let theme = self.theme.get();
        MaterialContainerStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            max_width: self.props.max_width,
            padding: self.props.padding,
            fluid: self.props.fluid,
        }
    }

    /// The centred, max-width-capped content wrapper.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let surface = to_paint(style.surface);
        let padding = style.padding;
        let max_width = style.max_width;
        let fluid = style.fluid;
        let child = self.child.clone();

        Stack::new(Axis::Horizontal)
            // v1's outer box: `width_pct(100.0).justify_center()`.
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .main_align(Align::Center),
            )
            .push(move || {
                // v1's inner box: `width_pct(100.0).padding(pad).background(surface)`,
                // and the max-width cap unless the container is fluid.
                let mut request = LayoutRequest::new()
                    .width_pct(100.0)
                    .padding(Insets::uniform(f64::from(padding)));
                if !fluid {
                    request = request.max_width(max_width);
                }
                let mut surface_box = Surface::new().request(request).background(surface);
                match child.clone() {
                    Some(build) => surface_box = surface_box.push(move || build()),
                    None => {}
                }
                Box::new(surface_box)
            })
    }
}

impl Default for MaterialContainer {
    fn default() -> Self {
        Self::new()
    }
}
