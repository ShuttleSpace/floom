
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Rect};
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon::IconProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

mod icons;

pub use icons::md_icon_svg;

/// The Material Symbols view box every glyph in [`icons`] is authored against.
pub const MD_ICON_VIEW_BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdIconStyle {
    pub color: Color,
    pub size: f32,
}

pub struct MdIcon {
    props: IconProps,
    theme: RwSignal<MultiTheme>,
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

    /// Names the icon for assistive technology. Implies `decorative(false)`:
    /// an icon with a label is no longer decoration.
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

    /// The glyph's name, as given.
    pub fn name(&self) -> &str {
        &self.props.name
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
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

    /// The path data for this icon's glyph, or the fallback dot for a name the
    /// set does not carry.
    pub fn path_data(&self) -> &'static str {
        icons::md_icon_path_data(&self.props.name)
    }

    pub fn resolve(&self) -> VectorIcon {
        let style = self.resolved_style();
        let mut icon = VectorIcon::from_path_data(self.path_data(), MD_ICON_VIEW_BOX)
            .size(style.size)
            .color(to_paint(style.color));
        if let Some(label) = self.props.label.as_deref() {
            icon = icon.label(label);
        }
        icon
    }
}

impl Default for MdIcon {
    fn default() -> Self {
        Self::new("")
    }
}
