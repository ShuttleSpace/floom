//! Fluent floating action button, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Shadow};
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{
    ComponentId, ComponentTokens, FloatingActionButtonTokens,
};
use floom_core_theme::props::floating_action_button::FloatingActionButtonProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFloatingActionButtonStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub size: f32,
    pub border_radius: f32,
    pub elevation: u8,
    pub icon_size: f32,
    pub extended: bool,
}

pub struct FluentFloatingActionButton {
    props: FloatingActionButtonProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
    on_click: Option<Callback>,
}

impl FluentFloatingActionButton {
    pub fn new() -> Self {
        Self {
            props: FloatingActionButtonProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
            on_click: None,
        }
    }

    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    pub fn extended(mut self, extended: bool) -> Self {
        self.props.extended = extended;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &FloatingActionButtonProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentFloatingActionButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::FloatingActionButton(tokens) =
            theme.component(ComponentId::FloatingActionButton)
        else {
            unreachable!(
                "ComponentId::FloatingActionButton must resolve to FloatingActionButtonTokens"
            )
        };
        let disabled = self.props.disabled;
        FluentFloatingActionButtonStyle {
            design_system: "fluent",
            background: if disabled {
                with_alpha(theme.colors().on_surface, 12)
            } else {
                theme.colors().primary
            },
            foreground: if disabled {
                with_alpha(theme.colors().on_surface, 38)
            } else {
                theme.colors().on_primary
            },
            size: tokens.size,
            border_radius: tokens.border_radius,
            elevation: tokens.elevation,
            icon_size: tokens.icon_size,
            extended: self.props.extended && !self.props.label.is_empty(),
        }
    }

    pub fn view(self) -> Pressable {
        let style = self.resolved_style();
        let label = self.props.label.clone();
        let extended = style.extended;
        let size = style.size;
        let foreground = to_paint(style.foreground);
        let child = self.child.clone();
        let on_click = self.on_click.clone();
        let disabled = self.props.disabled;

        let surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(to_paint(style.background))
            .radius(style.border_radius)
            .shadow(Shadow {
                color: to_paint(with_alpha(Color::BLACK, 30)),
                blur: 2.0 + f64::from(style.elevation),
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            })
            .push(move || {
                if extended {
                    let label = label.clone();
                    Box::new(
                        Stack::new(Axis::Horizontal)
                            .request(
                                LayoutRequest::new()
                                    .height(size)
                                    .padding_horiz(20.0)
                                    .main_align(Align::Center)
                                    .cross_align(Align::Center),
                            )
                            .gap(8.0)
                            .push({
                                let child = child.clone();
                                move || match child.clone() {
                                    Some(build) => build(),
                                    None => Box::new(Text::new(String::new())),
                                }
                            })
                            .push(move || {
                                Box::new(
                                    Text::new(label.clone()).size(size * 0.5).color(foreground),
                                )
                            }),
                    )
                } else {
                    Box::new(
                        Stack::new(Axis::Horizontal)
                            .request(
                                LayoutRequest::new()
                                    .width(size)
                                    .height(size)
                                    .main_align(Align::Center)
                                    .cross_align(Align::Center),
                            )
                            .push({
                                let child = child.clone();
                                move || match child.clone() {
                                    Some(build) => build(),
                                    None => Box::new(Text::new(String::new())),
                                }
                            }),
                    )
                }
            });

        Pressable::new(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(surface.clone()),
        ))
        .disabled(disabled)
        .on_click(move || {
            if !disabled {
                if let Some(cb) = &on_click {
                    cb();
                }
            }
        })
        .label("Floating action button")
    }
}

impl Default for FluentFloatingActionButton {
    fn default() -> Self {
        Self::new()
    }
}
