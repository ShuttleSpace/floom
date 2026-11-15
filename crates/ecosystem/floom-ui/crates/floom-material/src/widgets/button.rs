use std::sync::Arc;

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Button, Ripple, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

const DESIGN_SYSTEM: &str = "material";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdButtonStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub gap: f32,
    pub icon_size: f32,
    /// The corner radius the button's fill is drawn with.
    pub border_radius: f32,
    /// The fill while the pointer is over the button.
    pub hover_background: Color,
    /// The fill while the button is pressed.
    pub press_background: Color,
}

/// Material's button.
pub struct MdButton {
    props: ButtonProps,
    theme: RwSignal<MultiTheme>,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl MdButton {
    pub fn new() -> Self {
        Self {
            props: ButtonProps::default(),
            theme: theme_signal(),
            on_click: None,
        }
    }

    /// The button's label.
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

    /// Everything the button resolves to, so a caller can read it without
    /// mounting.
    pub fn resolved_style(&self) -> MdButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        let (background, foreground) = base_colors(&theme, self.props.variant, self.props.disabled);
        let (hover_background, press_background) = if self.props.disabled {
            (background, background)
        } else {
            (
                state_layer(&theme, self.props.variant, true),
                state_layer(&theme, self.props.variant, false),
            )
        };
        MdButtonStyle {
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
            border_radius: theme.radius().full,
            hover_background,
            press_background,
        }
    }

    fn paint_pair(&self, style: &MdButtonStyle) -> (PaintColor, PaintColor) {
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

    /// Padding the label should sit inside, from the button tokens.
    pub fn padding(&self) -> Insets {
        let style = self.resolved_style();
        let h = f64::from(style.padding_horizontal);
        Insets::uniform(h.max(f64::from(style.height) / 2.0))
    }

    pub fn resolve_with_ripple(&self) -> Ripple {
        let style = self.resolved_style();
        let (fill, label) = self.paint_pair(&style);
        let label_text = self.props.label.clone();
        let radius = style.border_radius;
        let hover = to_paint(style.hover_background);
        let press = to_paint(style.press_background);
        let disabled = self.props.disabled;
        let on_click = self.on_click.clone();
        Ripple::new()
            .radius(self.ripple_radius())
            .ink(self.ripple_ink())
            .disabled(disabled)
            .child(LayoutRequest::new().width(f32::INFINITY), move || {
                Box::new(build_button(
                    &label_text,
                    fill,
                    label,
                    radius,
                    hover,
                    press,
                    disabled,
                    on_click.clone(),
                ))
            })
    }

    /// The press ripple's ink: the button's own foreground at 24/255, which is
    /// what makes the ripple read on every variant.
    pub fn ripple_ink(&self) -> PaintColor {
        let style = self.resolved_style();
        to_paint(with_alpha(style.foreground, 24))
    }

    /// A fully-rounded clip for the ink. v1 asked the theme for `radius().full`
    /// on every use; here it is resolved once and pushed in.
    pub fn ripple_radius(&self) -> f64 {
        self.theme.get().radius().full as f64
    }

    pub fn follow_theme(&self, ripple: &mut Ripple) {
        ripple.set_radius(self.ripple_radius());
        ripple.set_ink(self.ripple_ink());
        ripple.set_disabled(self.props.disabled);
    }
}

impl Default for MdButton {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine button, assembled from plain values so the tree can rebuild one
/// per mount.
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

/// Fill and foreground for a variant, honouring the disabled palette.
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
        ButtonVariant::Ghost | ButtonVariant::Text => {
            (Color::rgba(0, 0, 0, 0), theme.colors().primary)
        }
    }
}

fn disabled_colors(theme: &MultiTheme) -> (Color, Color) {
    (
        with_alpha(theme.colors().on_surface, 12),
        with_alpha(theme.colors().on_surface, 38),
    )
}

/// Doubles a colour's alpha, so the engine's `a / 2` dimming of a disabled
/// control lands back on the value the kit meant.
fn undim(c: PaintColor) -> PaintColor {
    PaintColor {
        a: c.a.saturating_mul(2),
        ..c
    }
}

fn state_layer(theme: &MultiTheme, variant: ButtonVariant, hover: bool) -> Color {
    let overlay = match variant {
        ButtonVariant::Primary | ButtonVariant::Danger => theme.colors().on_primary,
        ButtonVariant::Secondary => theme.colors().on_secondary_container,
        ButtonVariant::Ghost | ButtonVariant::Text => theme.colors().primary,
    };
    let base = base_colors(theme, variant, false).0;
    let amt = if hover { 0.08 } else { 0.12 };
    mix(base, overlay, amt)
}

/// The label as its own widget, for callers that want to compose by hand.
pub fn label_widget(text: &str) -> Text {
    Text::new(text)
}
