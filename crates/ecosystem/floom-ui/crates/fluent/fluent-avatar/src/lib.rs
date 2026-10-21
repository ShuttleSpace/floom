//! Fluent avatar, ported to the Floom 2.0 widget API.
//!
//! An avatar is a circle (or, in Fluent's case, a rounded square) with a piece
//! of text in it. v1 spelled it as a `Container` holding a `Label`; 2.0 spells
//! it a [`Surface`] holding a [`Text`], which is the same two things with the
//! roles named: the surface is the shape, the text is the initials.
//!
//! ## What this kit answers
//!
//! * **the shape**: a **rounded square** -- `radius().md`, not a circle;
//! * **the palette**: `primary_container` for the shape and `on_primary_container` for the text;
//! * **the type**: the text at the token's size, at **SemiBold**.

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_core::LayoutRequest;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::avatar::AvatarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentAvatarStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub size: f32,
    pub font_size: f32,
}

/// Fluent's avatar.
pub struct FluentAvatar {
    props: AvatarProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentAvatar {
    pub fn new() -> Self {
        Self {
            props: AvatarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    /// The initials or short text shown in the avatar.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }

    /// The avatar's diameter in dp.
    ///
    /// The *props'* default is 40, not zero, so the rule below never reaches
    /// `AvatarTokens::size` -- and that token is dead in all three kits, for
    /// the same reason the circular progress's was. v1 wrote
    /// `if props.size > 0.0 { props.size } else { tokens.size }`; the props'
    /// default is 40, so the props always won. The rule is kept, because the
    /// rule is what v1 drew, and the dead token is named rather than quietly
    /// switched. (For Fluent it shows: its token says 36, and its avatar was
    /// 40 all the same.)
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
            // v1: the props' size when it is set, the token's otherwise.
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
        let radius = self.theme.get().radius().md;

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
                // The slot is a replayed `Fn`, so the text is cloned per mount.
                Box::new(Text::new(content.clone()).size(font_size).weight(FontWeight::SemiBold).color(text_color))
            })
    }
}

impl Default for FluentAvatar {
    fn default() -> Self {
        Self::new()
    }
}

