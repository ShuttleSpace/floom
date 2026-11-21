use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};

use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use crate::tokens::{INFO, SUCCESS, WARNING};
use crate::widgets::icon::MdIcon;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::alert::{AlertProps, AlertVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}

pub fn alert_accent(theme: &MultiTheme, variant: AlertVariant) -> Color {
    match variant {
        AlertVariant::Error => theme.colors().error,
        AlertVariant::Info => INFO,
        AlertVariant::Success => SUCCESS,
        AlertVariant::Warning => WARNING,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MdAlertStyle {
    pub design_system: &'static str,
    pub accent: Color,
    pub container_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub title_font_size: f32,
}

const DOT_BODY_GAP: f32 = 12.0;
const BODY_TEXT_GAP: f32 = 4.0;
const CONTENT_DISMISS_GAP: f32 = 8.0;
/// The accent dot: a 10pt circle.
const DOT_SIZE: f32 = 10.0;
/// The dismiss icon's size, and the padding around it.
const DISMISS_SIZE: f32 = 20.0;
const DISMISS_PADDING: f64 = 2.0;

pub struct MdAlert {
    props: AlertProps,
    theme: RwSignal<MultiTheme>,
    on_dismiss: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl MdAlert {
    pub fn new() -> Self {
        Self {
            props: AlertProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            on_dismiss: None,
        }
    }

    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.props.dismissible = dismissible;
        self
    }

    pub fn on_dismiss(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_dismiss = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_dismiss = Some(shared);
        self
    }

    pub fn props(&self) -> &AlertProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdAlertStyle {
        let theme = self.theme.get();
        let accent = alert_accent(&theme, self.props.variant);
        let ComponentTokens::Alert(tokens) = theme.component(ComponentId::Alert) else {
            unreachable!("ComponentId::Alert must resolve to AlertTokens")
        };
        MdAlertStyle {
            design_system: "material",
            accent,
            container_color: with_alpha(accent, 26),
            text_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            title_font_size: tokens.title_font_size,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let accent = to_paint(style.accent);
        let container = to_paint(style.container_color);
        let title = self.props.title.clone();
        let message = self.props.message.clone();
        let dismissible = self.props.dismissible;
        let on_dismiss = self.on_dismiss.clone();
        let title_size = style.title_font_size;
        let message_size = style.font_size;
        let title_color = to_paint(style.text_color);
        let message_color = to_paint(self.theme.get().colors().on_surface_variant);

        Surface::new()
            .request(
                LayoutRequest::new()
                    .padding_horiz(f64::from(style.padding))
                    .padding_vert(f64::from(style.padding))
                    .cross_align(Align::Center),
            )
            .background(container)
            .radius(style.border_radius)
            .push({
                let accent = accent;
                move || {
                    let content = Stack::new(Axis::Horizontal)
                        .gap(DOT_BODY_GAP)
                        .align(Align::Start, Align::Center)
                        .push({
                            let accent = accent;
                            move || {
                                Box::new(
                                    Surface::new()
                                        .request(
                                            LayoutRequest::new().width(DOT_SIZE).height(DOT_SIZE),
                                        )
                                        .background(accent)
                                        .radius(9999.0),
                                )
                            }
                        })
                        .push({
                            let title = title.clone();
                            let message = message.clone();
                            move || {
                                Box::new(
                                    Stack::new(Axis::Vertical)
                                        .gap(BODY_TEXT_GAP)
                                        .align(Align::Center, Align::Start)
                                        .push({
                                            let title = title.clone();
                                            move || {
                                                Box::new(
                                                    Text::new(title.clone())
                                                        .size(title_size)
                                                        .weight(FontWeight::Bold)
                                                        .color(title_color),
                                                )
                                            }
                                        })
                                        .push({
                                            let message = message.clone();
                                            move || {
                                                Box::new(
                                                    Text::new(message.clone())
                                                        .size(message_size)
                                                        .weight(FontWeight::Normal)
                                                        .color(message_color),
                                                )
                                            }
                                        }),
                                )
                            }
                        });

                    let mut row = Stack::new(Axis::Horizontal)
                        .gap(CONTENT_DISMISS_GAP)
                        .align(Align::Start, Align::Center)
                        .push(move || Box::new(content.clone()));

                    if dismissible {
                        row = row.push({
                            let on_dismiss = on_dismiss.clone();
                            move || {
                                let icon = MdIcon::new("close").size(DISMISS_SIZE).resolve();
                                Box::new(
                                    Pressable::new(Child::new(
                                        ChildKey::index(0),
                                        LayoutRequest::new()
                                            .padding_horiz(DISMISS_PADDING)
                                            .padding_vert(DISMISS_PADDING),
                                        move || Box::new(icon.clone()),
                                    ))
                                    .label("Dismiss"),
                                )
                            }
                        });
                    }

                    Box::new(row)
                }
            })
    }
}

impl Default for MdAlert {
    fn default() -> Self {
        Self::new()
    }
}
