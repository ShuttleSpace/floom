//! Material button, ported to the Floom 2.0 widget API.
//!
//! v1 built a `View` and styled it with a closure that re-read the theme on
//! every use. 2.0 mounts the tree once and its widgets are theme-agnostic, so
//! the theme is resolved to a concrete style up front
//! ([`MdButton::resolved_style`]) and handed to the engine `Button` as explicit
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
//! * the corner radius: a pill: `radius().full`;
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
//! v1's Material `resolved_style` already reported the height for the chosen
//! size, and the view it built drew that same height. The port agrees with
//! both.
//!
//! ## What the port names (not fakes)
//!
//! * **the Ghost variant's 1px `outline` border is gone.** v1 drew a frame
//! around the ghost button; the engine `Button` paints a fill and a label and has
//! no border, and a `Surface` wrapper would have to change the family's return
//! type (`resolve` hands back a `Button`). Named, not faked: the border is a
//! design-system answer the engine has no place for.
//! * **the hover and press fills replace the base fill rather than mixing with it.**
//! v1's `.hover(..)` set the background to the hover colour outright, and the
//! engine's `hover_color` does the same, so the two agree.
//! * **v1's focus ring is gone.** v1 drew a 2px outline in the `outline`
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
    /// Shared with the built widget. `Arc` rather than `Box` because 2.0's
    /// `Child` builder closure must be `Send + Sync + 'static`, so the callback
    /// has to be both clonable and thread-safe to be rebuilt into the tree.
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

    /// The button's size. v1 had this builder, and reported the height for the
    /// chosen size in the style -- the same as here.
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
    pub fn resolved_style(&self) -> MdButtonStyle {
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

    /// The fill and the label colour, as the engine paints them.
    ///
    /// A disabled control's colours are handed over double-alpha, because the
    /// engine halves the alpha of a disabled control's fill and its label.
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

    /// The button wrapped in its press ripple, ready to mount as one child.
    ///
    /// v1 wrapped the button in the shared Material press ripple: the ink circle is
    /// clipped to the button's fully-rounded bounds and tinted with the button's
    /// foreground at 24/255.
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
                // `Button` holds a boxed callback and so is not `Clone`; the tree
                // rebuilds a fresh one per call from captured plain data, which is
                // the 2.0 idiom.
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

    /// Push a freshly resolved theme into an already-mounted ripple.
    ///
    /// This is what replaces v1's `radius_fn` / `color` closures. Idempotent:
    /// setting the same values dirties nothing.
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

/// The M3 state layer: an 8% (hover) / 12% (press) overlay of the variant's own
/// foreground colour, mixed over the base fill.
///
/// v1's `state_layer`, reproduced exactly -- including that the overlay is the
/// *foreground* (white on a primary button), not a generic scrim.
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
