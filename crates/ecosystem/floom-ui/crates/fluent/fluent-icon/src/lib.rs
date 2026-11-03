use floom_signal::{RwSignal, SignalGet};
use floom_types::Rect;
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon::IconProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

pub use floom_widget_fluent_icon_paths::{fluent_icon_path, fluent_icon_svg};

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
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
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

    pub fn path_data(&self) -> &'static str {
        fluent_icon_path(&self.props.name)
    }

    pub fn resolve(&self) -> VectorIcon {
        fluent_icon(&self.props.name)
    }
}

// ported kit uses, and note that they resolve **no theme at all** — which is the point:

pub const VIEW_BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

pub fn fluent_icon(name: &str) -> VectorIcon {
    VectorIcon::from_path_data(fluent_icon_path(name), VIEW_BOX)
}
