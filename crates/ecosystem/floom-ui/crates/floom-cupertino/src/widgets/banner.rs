//! Cupertino banner, ported to the Floom 2.0 widget API.
//!

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;

use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use crate::tokens::{INFO, SUCCESS, WARNING};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::banner::{BannerProps, BannerVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

/// The action's own padding, at every side. A v1 literal.
const ACTION_PADDING: f64 = 4.0;

/// The accent for each variant.
///
/// Cupertino has no built-in info / success / warning tokens, so three of these
/// are fixed RGB literals and only `error` reads the theme. Kept as v1 wrote
/// it: the literals are the design, not an oversight, and a port that
/// resolved them through the theme would change three kits' colours.
pub fn banner_accent(theme: &MultiTheme, variant: BannerVariant) -> Color {
    match variant {
        BannerVariant::Error => theme.colors().error,
        BannerVariant::Info => INFO,
        BannerVariant::Success => SUCCESS,
        BannerVariant::Warning => WARNING,
    }
}

/// The banner's appearance, in the terms v1 used.
///
/// The fields v1's style did not report but the paint used -- the strip's
/// fill, the action's ink and the action's padding -- are added here, because
/// a style that does not say what the widget draws is not the widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoBannerStyle {
    /// Added: v1's banner style named no design system.
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

/// Cupertino's banner.
pub struct CupertinoBanner {
    props: BannerProps,
    theme: RwSignal<MultiTheme>,
    /// The action callback in the shape 2.0 needs.
    ///
    /// `BannerProps::on_action` is `Option<Box<dyn Fn()>>` -- a v1 type -- and the
    /// engine's slot requires `Send + Sync`. Stored twice, from one closure; the
    /// v1-shaped copy keeps `props()` honest.
    on_action: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl CupertinoBanner {
    pub fn new() -> Self {
        Self {
            props: BannerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    /// The action's callback.
    ///
    /// v1 took `impl Fn() + 'static`; 2.0's slots are shared between threads, so
    /// the callback has to be `Send + Sync` as well.
    pub fn on_action(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_action = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_action = Some(shared);
        self
    }

    /// Restrict the banner to a single line.
    ///
    /// v1's `single_line` never reached the paint -- the message was never
    /// truncated -- and neither does this. The prop is kept because v1's props
    /// carried it.
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

    pub fn resolved_style(&self) -> CupertinoBannerStyle {
        let theme = self.theme.get();
        let accent = banner_accent(&theme, self.props.variant);
        let ComponentTokens::Banner(tokens) = theme.component(ComponentId::Banner) else {
            unreachable!("ComponentId::Banner must resolve to BannerTokens")
        };
        CupertinoBannerStyle {
            design_system: "cupertino",
            accent,
            // v1: `with_alpha(accent, 26)`, i.e. about 10% -- a tint, not a wash.
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

    /// The banner, ready to mount.
    ///
    /// The tree, outermost first:
    ///
    /// ```text
    /// Surface          the tinted strip: padding, min height, accent @ 26
    ///  └ Stack         space-between, centred: the message group, then the action
    ///     ├ Stack      gap 12: the icon (if there is one), then the message
    ///     │  ├ Text    the icon, in the accent
    ///     │  └ Text    the message, in `on_surface`
    ///     └ Pressable  only when there is an action label
    /// ```
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
                    // v1's `.min_height(tokens.height)`.
                    .min_height(style.height)
                    // v1's `.items_center()` on the banner.
                    .cross_align(Align::Center),
            )
            // v1's `.padding(padding)`.
            .padding(Insets::uniform(f64::from(style.padding)))
            .background(container)
            .push(move || {
                // The left group: the icon, if there is one, and the message.
                let mut left = Stack::new(Axis::Horizontal)
                    // The live gap: the one between the icon and the message.
                    .gap(gap)
                    // v1's `.items_center()`.
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

                // The row: the group on the left, the action on the right.
                let mut row = Stack::new(Axis::Horizontal)
                    // v1's `.justify_between()`.
                    .align(Align::SpaceBetween, Align::Center)
                    .push(move || Box::new(left.clone()));

                // The action, only when there is a label for it. v1 mounted an
                // `Empty` view here otherwise, so its row always had two children;
                // `Empty` had no size, so the only difference is structural.
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

impl Default for CupertinoBanner {
    fn default() -> Self {
        Self::new()
    }
}
