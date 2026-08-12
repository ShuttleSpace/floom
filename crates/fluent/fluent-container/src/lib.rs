use floem::prelude::*;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::container::ContainerProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentContainerStyle {
    pub surface: Color,
    pub content: Color,
    pub max_width: f32,
    pub padding: f32,
    pub fluid: bool,
}

pub struct FluentContainer {
    props: ContainerProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentContainer {
    pub fn new() -> Self {
        Self {
            props: ContainerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    pub fn props(&self) -> &ContainerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> FluentContainerStyle {
        let theme = self.theme.get();
        FluentContainerStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            max_width: self.props.max_width,
            padding: self.props.padding,
            fluid: self.props.fluid,
        }
    }

    /// Build a centered, max-width-capped content wrapper.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child.unwrap_or_else(|| empty().into_any());
        let max_width = props.max_width;
        let padding = props.padding;
        let fluid = props.fluid;

        Container::new(
            Container::new(child).style(move |s| {
                let t = theme.get();
                let mut s = s
                    .width_pct(100.0)
                    .padding(padding)
                    .background(t.colors().surface.to_peniko());
                if !fluid {
                    s = s.max_width(max_width);
                }
                s
            }),
        )
        .style(|s| s.width_pct(100.0).justify_center())
    }
}

impl Default for FluentContainer {
    fn default() -> Self {
        Self::new()
    }
}
