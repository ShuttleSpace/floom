use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;

use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::banner::{BannerProps, BannerVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

const ACTION_PADDING: f64 = 4.0;

pub fn banner_accent(theme: &MultiTheme, variant: BannerVariant) -> Color {
    match variant {
        BannerVariant::Error => theme.colors().error,
        BannerVariant::Info => Color::rgb(33, 150, 243),
        BannerVariant::Success => Color::rgb(76, 175, 80),
        BannerVariant::Warning => Color::rgb(255, 152, 0),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialBannerStyle {
    pub design_system: &'static str,
    /// The variant's accent. The icon's ink.
    pub accent: Color,
    /// Added: the strip's fill -- the accent at alpha 26.
    pub container_color: Color,
    /// The message's ink.
    pub text_color: Color,
    /// Added: the action's ink -- `primary`.
    pub action_color: Color,
    /// Added: the action's own padding. A v1 literal.
    pub action_padding: f32,
    pub height: f32,
    pub padding: f32,
    pub font_size: f32,
    pub action_font_size: f32,
    pub gap: f32,
    pub icon_size: f32,
}

/// Material's banner.
pub struct MaterialBanner {
    props: BannerProps,
    theme: RwSignal<MultiTheme>,
    on_action: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl MaterialBanner {
    pub fn new() -> Self {
        Self {
            props: BannerProps::default(),
            theme: theme_signal(),
            on_action: None,
        }
    }

    pub fn variant(mut self, variant: BannerVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.props.icon = Some(icon.into());
        self
    }

    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.props.action_label = Some(label.into());
        self
    }

    pub fn on_action(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_action = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_action = Some(shared);
        self
    }

    pub fn single_line(mut self, single_line: bool) -> Self {
        self.props.single_line = single_line;
        self
    }

    pub fn props(&self) -> &BannerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialBannerStyle {
        let theme = self.theme.get();
        let accent = banner_accent(&theme, self.props.variant);
        let ComponentTokens::Banner(tokens) = theme.component(ComponentId::Banner) else {
            unreachable!("ComponentId::Banner must resolve to BannerTokens")
        };
        MaterialBannerStyle {
            design_system: "material",
            accent,
            container_color: with_alpha(accent, 26),
            text_color: theme.colors().on_surface,
            action_color: theme.colors().primary,
            action_padding: ACTION_PADDING as f32,
            height: tokens.height,
            padding: tokens.padding,
            font_size: tokens.font_size,
            action_font_size: tokens.action_font_size,
            gap: tokens.gap,
            icon_size: tokens.icon_size,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let accent = to_paint(style.accent);
        let container = to_paint(style.container_color);
        let message_color = to_paint(style.text_color);
        let action_color = to_paint(style.action_color);
        let message = self.props.message.clone();
        let icon = self.props.icon.clone();
        let action_label = self.props.action_label.clone();
        let on_action = self.on_action.clone();
        let message_size = style.font_size;
        let action_size = style.action_font_size;
        let icon_size = style.icon_size;
        let gap = style.gap;

        Surface::new()
            .request(
                LayoutRequest::new()
                    .min_height(style.height)
                    .cross_align(Align::Center),
            )
            .padding(Insets::uniform(f64::from(style.padding)))
            .background(container)
            .push(move || {
                let mut left = Stack::new(Axis::Horizontal)
                    .gap(gap)
                    .align(Align::Start, Align::Center);
                if let Some(icon) = icon.clone() {
                    left = left.push(move || {
                        Box::new(Text::new(icon.clone()).size(icon_size).color(accent))
                    });
                }
                let message = message.clone();
                left = left.push(move || {
                    Box::new(
                        Text::new(message.clone())
                            .size(message_size)
                            .color(message_color),
                    )
                });

                let mut row = Stack::new(Axis::Horizontal)
                    .align(Align::SpaceBetween, Align::Center)
                    .push(move || Box::new(left.clone()));

                if let Some(label) = action_label.clone() {
                    let on_action = on_action.clone();
                    row = row.push(move || {
                        let label = label.clone();
                        let on_action = on_action.clone();
                        let semantics = label.clone();
                        Box::new(
                            Pressable::new(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new(),
                                move || {
                                    let label = label.clone();
                                    Box::new(
                                        Surface::new()
                                            .request(
                                                LayoutRequest::new()
                                                    .padding_horiz(ACTION_PADDING)
                                                    .padding_vert(ACTION_PADDING),
                                            )
                                            .child(Child::new(
                                                ChildKey::index(0),
                                                LayoutRequest::new(),
                                                move || {
                                                    let label = label.clone();
                                                    Box::new(
                                                        Text::new(label)
                                                            .size(action_size)
                                                            .color(action_color),
                                                    )
                                                },
                                            )),
                                    )
                                },
                            ))
                            .on_click(move || {
                                if let Some(callback) = &on_action {
                                    callback();
                                }
                            })
                            .label(semantics),
                        )
                    });
                }

                Box::new(row)
            })
    }
}

impl Default for MaterialBanner {
    fn default() -> Self {
        Self::new()
    }
}
