use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::icon::IconProps;
use floem_core_theme::theme::{MaterialTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdIconStyle {
    pub color: Color,
    pub size: f32,
}

pub struct MdIcon {
    props: IconProps,
    theme: RwSignal<MaterialTheme>,
}

impl MdIcon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            props: IconProps {
                name: name.into(),
                ..Default::default()
            },
            theme: theme_signal(),
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

    pub fn theme(&self) -> RwSignal<MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdIconStyle {
        let theme = self.theme.get();
        let ComponentTokens::Icon(tokens) = theme.component(ComponentId::Icon) else {
            unreachable!("ComponentId::Icon must resolve to IconTokens")
        };
        MdIconStyle {
            color: theme.colors().on_surface_variant,
            size: self.props.size.unwrap_or(tokens.size),
        }
    }
}
