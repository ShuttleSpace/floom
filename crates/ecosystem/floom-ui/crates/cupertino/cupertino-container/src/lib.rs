//! Cupertino container, ported to the Floom 2.0 widget API.
//!

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::props::container::ContainerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// A builder for the container's content, replayed each time the box is built.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoContainerStyle {
    pub surface: Color,
    pub content: Color,
    pub max_width: f32,
    pub padding: f32,
    pub fluid: bool,
}

/// Cupertino's container.
pub struct CupertinoContainer {
    props: ContainerProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl CupertinoContainer {
    pub fn new() -> Self {
        Self {
            props: ContainerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoContainerStyle {
        let theme = self.theme.get();
        CupertinoContainerStyle {
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

impl Default for CupertinoContainer {
    fn default() -> Self {
        Self::new()
    }
}
