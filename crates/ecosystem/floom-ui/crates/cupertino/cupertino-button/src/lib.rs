//! Cupertino button, ported to the Floom 2.0 widget API.
//!

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

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoButtonStyle {
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

/// Cupertino's button.
pub struct CupertinoButton {
    props: ButtonProps,
    theme: RwSignal<MultiTheme>,
    /// Shared with the built widget. `Arc` rather than `Box` because 2.0's
    /// `Child` builder closure must be `Send + Sync + 'static`, so the callback
    /// has to be both cloneable and thread-safe to be rebuilt into the tree.
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl CupertinoButton {
    pub fn new() -> Self {
        Self {
            props: ButtonProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    /// The button's size. v1 had this builder too, but its `resolved_style`
    /// reported the *token* height whatever the size, so the size only showed
    /// up in the view it built. 2.0 reports the height the button is drawn at,
    /// so the size is visible in the style as well.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.props.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// v1 carried a `loading` flag on the props and never drew anything for it.
    /// It is carried here for the same reason: the props keep reporting it, and
    /// no kit claims a spinner it does not draw.
    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self
    }

    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`.
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
    pub fn resolved_style(&self) -> CupertinoButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        let (background, foreground) = base_colors(&theme, self.props.variant, self.props.disabled);
        // A disabled button has no hover or press of its own: v1's state layer
        // returned the disabled fill for both, and the engine ignores the
        // interaction colours outright while a control is disabled.
        let (hover_background, press_background) = if self.props.disabled {
            (background, background)
        } else {
            (with_alpha(foreground, 12), with_alpha(foreground, 20))
        };
        CupertinoButtonStyle {
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

    /// The fill and the label colour, as the engine paints them.
    ///
    /// A disabled control's colours are handed over double-alpha, because the
    /// engine halves the alpha of a disabled control's fill and its label.
    fn paint_pair(&self, style: &CupertinoButtonStyle) -> (PaintColor, PaintColor) {
        if self.props.disabled {
            (
                undim(to_paint(style.background)),
                undim(to_paint(style.foreground)),
            )
        } else {
            (to_paint(style.background), to_paint(style.foreground))
        }
    }

    /// The button itself, ready to mount.
    ///
    /// The 2.0 `Button` is a real engine widget, so the caller mounts it with
    /// `Child::new(..)` -- a 2.0 widget holds no children of its own.
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

    /// The button, ready to mount. The name the rest of the ported kits use.
    pub fn view(&self) -> Button {
        self.resolve()
    }

    /// Padding the label should sit inside, from the button tokens.
    pub fn padding(&self) -> Insets {
        let style = self.resolved_style();
        let h = f64::from(style.padding_horizontal);
        Insets::uniform(h.max(f64::from(style.height) / 2.0))
    }
}

impl Default for CupertinoButton {
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

/// v1's disabled palette: `on_surface` at 12/255 as the fill and 38/255 as the
/// label, for every design system.
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

/// The label as its own widget, for callers that want to compose by hand.
pub fn label_widget(text: &str) -> Text {
    Text::new(text)
}
