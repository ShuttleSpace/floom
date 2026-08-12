use floem::prelude::*;
use floem::reactive::RwSignal;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::avatar::AvatarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, PartialEq)]
pub struct MdAvatarStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub size: f32,
    pub font_size: f32,
}

pub struct MdAvatar {
    props: AvatarProps,
    theme: RwSignal<MultiTheme>,
}

impl MdAvatar {
    pub fn new() -> Self {
        Self {
            props: AvatarProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = size;
        self
    }

    pub fn resolved_style(&self) -> MdAvatarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Avatar(tokens) = theme.component(ComponentId::Avatar) else {
            unreachable!("ComponentId::Avatar must resolve to AvatarTokens")
        };
        MdAvatarStyle {
            design_system: "material",
            background: theme.colors().primary_container,
            text_color: theme.colors().on_primary_container,
            size: if self.props.size > 0.0 {
                self.props.size
            } else {
                tokens.size
            },
            font_size: tokens.font_size,
        }
    }

    /// Build a circular Material 3 avatar showing initials/text.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let content = self.props.content.clone();
        let size = self.resolved_style().size;
        let font_size = self.resolved_style().font_size;

        Container::new(text(content))
            .style(move |s| {
                let t = theme.get();
                s.width(size)
                    .height(size)
                    .border_radius(9999.0)
                    .background(t.colors().primary_container.to_peniko())
                    .color(t.colors().on_primary_container.to_peniko())
                    .font_size(font_size)
                    .items_center()
                    .justify_center()
            })
    }
}

impl Default for MdAvatar {
    fn default() -> Self {
        Self::new()
    }
}
