use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::avatar::AvatarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

const DESIGN_SYSTEM: &str = "material";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialAvatarStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub size: f32,
    pub font_size: f32,
}

/// Material's avatar.
pub struct MaterialAvatar {
    props: AvatarProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialAvatar {
    pub fn new() -> Self {
        Self {
            props: AvatarProps::default(),
            theme: theme_signal(),
        }
    }

    /// The initials or short text shown in the avatar.
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

    pub fn resolved_style(&self) -> MaterialAvatarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Avatar(tokens) = theme.component(ComponentId::Avatar) else {
            unreachable!("ComponentId::Avatar must resolve to AvatarTokens")
        };
        MaterialAvatarStyle {
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

    /// The avatar, ready to mount.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let content = self.props.content.clone();
        let size = style.size;
        let font_size = style.font_size;
        let text_color = to_paint(style.text_color);
        let radius = 9999.0;

        Surface::new()
            .request(
                LayoutRequest::new()
                    .width(size)
                    .height(size)
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(to_paint(style.background))
            .radius(radius)
            .push(move || {
                Box::new(
                    Text::new(content.clone())
                        .size(font_size)
                        .weight(FontWeight::Normal)
                        .color(text_color),
                )
            })
    }
}

impl Default for MaterialAvatar {
    fn default() -> Self {
        Self::new()
    }
}
