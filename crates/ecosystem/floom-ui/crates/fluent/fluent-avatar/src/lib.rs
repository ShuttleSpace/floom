use floom::prelude::*;
use floom::reactive::RwSignal;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::avatar::AvatarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentAvatarStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub size: f32,
    pub font_size: f32,
}

pub struct FluentAvatar {
    props: AvatarProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentAvatar {
    pub fn new() -> Self {
        Self {
            props: AvatarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
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

    pub fn resolved_style(&self) -> FluentAvatarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Avatar(tokens) = theme.component(ComponentId::Avatar) else {
            unreachable!("ComponentId::Avatar must resolve to AvatarTokens")
        };
        FluentAvatarStyle {
            design_system: DESIGN_SYSTEM,
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

    /// Build a Fluent 2 avatar showing initials/text. Fluent 2 avatars are
    /// rounded squares (4dp corner radius), not circles.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let content = self.props.content.clone();
        let size = self.resolved_style().size;
        let font_size = self.resolved_style().font_size;

        Container::new(Label::new(content))
            .style(move |s| {
                let t = theme.get();
                s.width(size)
                    .height(size)
                    .border_radius(t.radius().md)
                    .background(t.colors().primary_container.to_peniko())
                    .color(t.colors().on_primary_container.to_peniko())
                    .font_size(font_size)
                    .font_weight(FontWeight::SemiBold.to_floom())
                    .items_center()
                    .justify_center()
            })
    }
}

impl Default for FluentAvatar {
    fn default() -> Self {
        Self::new()
    }
}
