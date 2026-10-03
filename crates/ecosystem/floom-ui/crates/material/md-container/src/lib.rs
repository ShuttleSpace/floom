use floom::prelude::*;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::container::ContainerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdContainerStyle {
    pub surface: Color,
    pub content: Color,
    pub max_width: f32,
    pub padding: f32,
    pub fluid: bool,
}

pub struct MdContainer {
    props: ContainerProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl MdContainer {
    pub fn new() -> Self {
        Self {
            props: ContainerProps::default(),
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

    pub fn resolved_style(&self) -> MdContainerStyle {
        let theme = self.theme.get();
        MdContainerStyle {
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
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
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

impl Default for MdContainer {
    fn default() -> Self {
        Self::new()
    }
}
