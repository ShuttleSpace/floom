use floem::prelude::*;
use floem::views::{Decorators, svg};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::icon::IconProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;

mod icons;
pub use icons::fluent_icon_svg;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentIconStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub size: f32,
}

pub struct FluentIcon {
    props: IconProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentIcon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            props: IconProps {
                name: name.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = Some(size);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self.props.decorative = false;
        self
    }

    pub fn decorative(mut self, decorative: bool) -> Self {
        self.props.decorative = decorative;
        self
    }

    pub fn props(&self) -> &IconProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentIconStyle {
        let theme = self.theme.get();
        let ComponentTokens::Icon(tokens) = theme.component(ComponentId::Icon) else {
            unreachable!("ComponentId::Icon must resolve to IconTokens")
        };
        FluentIconStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().on_surface,
            size: self.props.size.unwrap_or(tokens.size),
        }
    }

    /// Build a renderable Fluent icon.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let name = self.props.name.clone();
        let size = self.props.size;
        svg(move || icons::fluent_icon_svg(&name)).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Icon(tokens) = t.component(ComponentId::Icon) else {
                unreachable!()
            };
            let sz = size.unwrap_or(tokens.size);
            s.size(sz, sz)
                .color(t.colors().on_surface.to_peniko())
        })
    }
}
