//! Fluent button, ported to the Floom 2.0 widget API.
//!
//! v1 built a `View` and styled it with a closure that re-read the theme on
//! every use. 2.0 mounts the tree once and its widgets are theme-agnostic, so
//! the theme is resolved to a concrete style up front
//! ([`FluentButton::resolved_style`]) and handed to the engine `Button` as explicit
//! values.
//!
//! ## What the port reproduces
//!
//! * the variant palette: `primary`/`on_primary`, `secondary_container`/
//!   `on_secondary_container`, `error`/`on_error`, and a transparent fill with a
//!   `primary` label for the ghost and text variants;
//! * the disabled palette: `on_surface` at 12/255 as the fill and at 38/255 as
//!   the label;
//! * the height for each size: the token height, 8 shorter for a small button
//!   and 8 taller for a large one;
//! * the corner radius: `radius().lg`;
//! * the hover and press fills.
//!
//! ## The engine `Button` and the fill/label pair
//!
//! The engine `Button` *is* its label: it paints a fill and a run of text, and
//! it has no children. So the fill is `Button::color` and the label colour is
//! `Button::text_color` -- two different knobs, and the kit has to use the right
//! one for each. Passing the label colour to `color` paints a
//! foreground-coloured button with a white label on it, which is a bug this port
//! is careful not to reproduce.
//!
//! ## The disabled palette and the engine's dimming rule
//!
//! The engine dims a disabled control by halving the alpha of its fill and of
//! its label. v1 named a disabled palette outright, so the kit hands the engine a
//! double-alpha colour and the engine's halving lands on v1's value. The
//! doubling is undone exactly, because the dimming is a plain `a / 2`.
//!
//! ## The height the style reports
//!
//! v1's Fluent `resolved_style` reported the *token* height, whatever the
//! size, while the view it built drew the height for the chosen size. The port
//! reports the height the button is actually drawn at, so the two agree. For a
//! medium button -- the default -- the two are the same number; they differ
//! only for a small or a large one.
//!
//! ## What the port names (not fakes)
//!
//! * **the Ghost variant's 1px `primary` border is gone.** v1 drew a frame
//! around the ghost button; the engine `Button` paints a fill and a label and has
//! no border, and a `Surface` wrapper would have to change the family's return
//! type (`resolve` hands back a `Button`). Named, not faked: the border is a
//! design-system answer the engine has no place for.
//! * **the hover and press fills are a flat tint of the foreground, not a mix with
//! the base.** v1's `.hover(..)` replaced the background with `foreground` at
//! 12/255, so hovering a filled button drops its fill entirely and washes the
//! whole button toward the label colour. That is what v1 did, and the engine's
//! `hover_color` replaces the fill the same way -- but where Material's hover is
//! a state layer *mixed over* the base, this is not. Named, not smoothed over.
//! * **v1's focus ring is gone.** v1 drew a 2px outline in the `primary`
//!   colour; the engine paints its own focus band, in its own colour.
//! * **the label's type is the engine's, not the theme's.** v1 drew the label at
//!   `typography().label.large` (its size and weight); the engine draws it at a
//!   fixed size with the default weight. The engine `Button` has no label-metric
//!   knob, and adding one is an engine change rather than a kit one.
//! * **`props().on_click` is always `None`.** v1 stored the callback in the
//!   props as a `Box<dyn Fn()>`; 2.0's `Child` builder closure must be
//!   `Send + Sync`, so the callback the tree uses is an `Arc<dyn Fn() + Send +
//!   Sync>` held beside the props instead of inside them.
//! * **the label is set by a builder, not by the constructor.** v1 took it in
//!   `new("Save")`; the ported kits use `new()` plus a builder, and so does this
//!   one.

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Button, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
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
    /// The corner radius the button's fill is drawn with.
    pub border_radius: f32,
    /// The fill while the pointer is over the button.
    pub hover_background: Color,
    /// The fill while the button is pressed.
    pub press_background: Color,
}

/// Fluent's button.
pub struct FluentButton {
    props: ButtonProps,
    theme: RwSignal<MultiTheme>,
    /// Shared with the built widget. `Arc` rather than `Box` because 2.0's
    /// `Child` builder closure must be `Send + Sync + 'static`, so the callback
    /// has to be both clonable and thread-safe to be rebuilt into the tree.
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
    pub fn resolved_style(&self) -> FluentButtonStyle {
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

    /// The fill and the label colour, as the engine paints them.
    ///
    /// A disabled control's colours are handed over double-alpha, because the
    /// engine halves the alpha of a disabled control's fill and its label.
    fn paint_pair(&self, style: &FluentButtonStyle) -> (PaintColor, PaintColor) {
        if self.props.disabled {
            (undim(to_paint(style.background)), undim(to_paint(style.foreground)))
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

impl Default for FluentButton {
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
        ButtonVariant::Ghost | ButtonVariant::Text => (Color::rgba(0, 0, 0, 0), theme.colors().primary),
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
    PaintColor { a: c.a.saturating_mul(2), ..c }
}

/// Design-system colour -> engine colour. Both are plain `{r, g, b, a}` byte
/// structs; `Color::to_peniko` goes the other way.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// The label as its own widget, for callers that want to compose by hand.
pub fn label_widget(text: &str) -> Text {
    Text::new(text)
}
