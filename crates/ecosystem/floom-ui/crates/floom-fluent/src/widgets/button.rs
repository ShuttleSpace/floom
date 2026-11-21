//! Fluent button, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Button, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentButtonStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub gap: f32,
    pub icon_size: f32,
    pub border_radius: f32,
    pub hover_background: Color,
    pub press_background: Color,
}

pub struct FluentButton {
    props: ButtonProps,
    theme: RwSignal<MultiTheme>,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl FluentButton {
    pub fn new() -> Self {
        Self {
            props: ButtonProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.props.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &ButtonProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn is_disabled(&self) -> bool {
        self.props.disabled
    }

    pub fn resolved_style(&self) -> FluentButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        let (background, foreground) = base_colors(&theme, self.props.variant, self.props.disabled);
        let (hover_background, press_background) = if self.props.disabled {
            (background, background)
        } else {
            (with_alpha(foreground, 12), with_alpha(foreground, 20))
        };
        FluentButtonStyle {
            design_system: DESIGN_SYSTEM,
            background,
            foreground,
            height: match self.props.size {
                ButtonSize::Small => tokens.height - 8.0,
                ButtonSize::Medium => tokens.height,
                ButtonSize::Large => tokens.height + 8.0,
            },
            padding_horizontal: tokens.padding_horizontal,
            gap: tokens.gap,
            icon_size: tokens.icon_size,
            border_radius: theme.radius().lg,
            hover_background,
            press_background,
        }
    }

    fn paint_pair(&self, style: &FluentButtonStyle) -> (PaintColor, PaintColor) {
        if self.props.disabled {
            (
                undim(to_paint(style.background)),
                undim(to_paint(style.foreground)),
            )
        } else {
            (to_paint(style.background), to_paint(style.foreground))
        }
    }

    pub fn resolve(&self) -> Button {
        let style = self.resolved_style();
        let (fill, label) = self.paint_pair(&style);
        build_button(
            &self.props.label,
            fill,
            label,
            style.border_radius,
            to_paint(style.hover_background),
            to_paint(style.press_background),
            self.props.disabled,
            self.on_click.clone(),
        )
    }

    pub fn view(&self) -> Button {
        self.resolve()
    }

    pub fn padding(&self) -> Insets {
        let style = self.resolved_style();
        let h = f64::from(style.padding_horizontal);
        Insets::uniform(h.max(f64::from(style.height) / 2.0))
    }
}

impl Default for FluentButton {
    fn default() -> Self {
        Self::new()
    }
}

fn build_button(
    label: &str,
    fill: PaintColor,
    text: PaintColor,
    radius: f32,
    hover: PaintColor,
    press: PaintColor,
    disabled: bool,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Button {
    let mut button = Button::new(label)
        .color(fill)
        .text_color(text)
        .radius(radius)
        .hover_color(hover)
        .press_color(press)
        .disabled(disabled);
    if let Some(on_click) = on_click {
        button = button.on_click(move || on_click());
    }
    button
}

fn base_colors(theme: &MultiTheme, variant: ButtonVariant, disabled: bool) -> (Color, Color) {
    if disabled {
        return disabled_colors(theme);
    }
    match variant {
        ButtonVariant::Primary => (theme.colors().primary, theme.colors().on_primary),
        ButtonVariant::Secondary => (
            theme.colors().secondary_container,
            theme.colors().on_secondary_container,
        ),
        ButtonVariant::Danger => (theme.colors().error, theme.colors().on_error),
        ButtonVariant::Ghost | ButtonVariant::Text => (Color::TRANSPARENT, theme.colors().primary),
    }
}

fn disabled_colors(theme: &MultiTheme) -> (Color, Color) {
    (
        with_alpha(theme.colors().on_surface, 12),
        with_alpha(theme.colors().on_surface, 38),
    )
}

fn undim(c: PaintColor) -> PaintColor {
    PaintColor {
        a: c.a.saturating_mul(2),
        ..c
    }
}

pub fn label_widget(text: &str) -> Text {
    Text::new(text)
}
