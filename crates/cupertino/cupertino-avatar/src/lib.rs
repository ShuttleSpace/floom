use floem::prelude::*;
use floem::reactive::RwSignal;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::avatar::AvatarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoAvatarStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub size: f32,
    pub font_size: f32,
}

pub struct CupertinoAvatar {
    props: AvatarProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoAvatar {
    pub fn new() -> Self {
        Self {
            props: AvatarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn props(&self) -> &AvatarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoAvatarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Avatar(tokens) = theme.component(ComponentId::Avatar) else {
            unreachable!("ComponentId::Avatar must resolve to AvatarTokens")
        };
        CupertinoAvatarStyle {
            design_system: DESIGN_SYSTEM,
            background: theme.colors().surface_variant,
            text_color: theme.colors().on_surface,
            size: if self.props.size > 0.0 {
                self.props.size
            } else {
                tokens.size
            },
            font_size: tokens.font_size,
        }
    }

    /// Build a circular Cupertino avatar (neutral gray fill, centered initials).
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
                    .border_radius(t.radius().full)
                    .background(t.colors().surface_variant.to_peniko())
                    .color(t.colors().on_surface.to_peniko())
                    .font_size(font_size)
                    .items_center()
                    .justify_center()
            })
    }
}

impl Default for CupertinoAvatar {
    fn default() -> Self {
        Self::new()
    }
}
