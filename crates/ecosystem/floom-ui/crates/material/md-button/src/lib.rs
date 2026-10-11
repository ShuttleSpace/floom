//! Material 3 button, ported to the Floom 2.0 widget API.
//!
//! The **interactive** reference port, alongside `md-card` (surface) and the
//! `Ripple`/`Badge` engine widgets. It is here because an interactive control
//! exercises things a surface never touches: a child that consumes its own
//! events, a press ripple wrapping it, and theme- and state-derived values that
//! have to *change* after mount.
//!
//! # What changed from v1
//!
//! v1 composed a `View` and styled it with a closure that re-read the theme on
//! every use, and handed `Ripple` two closures (`radius_fn`, `color`) so both
//! followed the theme live. 2.0 mounts the tree once and mutates widgets
//! through `WidgetHost::widget_mut`, and its widgets are theme-agnostic, so:
//!
//! * the theme is resolved to a concrete style up front ([`MdButton::resolve`]),
//!   and
//! * the pieces that must track later changes are pushed in through the
//!   `set_*` mutators on [`Ripple`] and [`MdButton`].
//!
//! See `docs/porting-floem-ui-to-2.0.md`.

use std::sync::Arc;

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Button, Ripple, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button::{ButtonProps, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdButtonStyle {
    pub background: Color,
    pub foreground: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub gap: f32,
    pub icon_size: f32,
}

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

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
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

    pub fn resolved_style(&self) -> MdButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        let (background, foreground) =
            base_colors(&theme, self.props.variant, self.props.disabled);
        MdButtonStyle {
            background,
            foreground,
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            gap: tokens.gap,
            icon_size: tokens.icon_size,
        }
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

    /// The button itself, ready to mount.
    ///
    /// The 2.0 `Button` is a real engine widget, so the caller mounts it with
    /// `Child::new(..)` — a 2.0 widget holds no children of its own.
    pub fn resolve(&self) -> Button {
        build_button(
            &self.props.label,
            to_paint(self.resolved_style().foreground),
            self.props.disabled,
            self.on_click.clone(),
        )
    }

    /// The button wrapped in its press ripple, ready to mount as one child.
    pub fn resolve_with_ripple(&self) -> Ripple {
        Ripple::new()
            .radius(self.ripple_radius())
            .ink(self.ripple_ink())
            .disabled(self.props.disabled)
            .child(LayoutRequest::new().width(f32::INFINITY), {
                // `Button` holds a boxed callback and so is not `Clone`; the
                // tree rebuilds a fresh one per call from captured plain data,
                // which is the 2.0 idiom.
                let label = self.props.label.clone();
                let color = to_paint(self.resolved_style().foreground);
                let disabled = self.props.disabled;
                let on_click = self.on_click.clone();
                move || Box::new(build_button(&label, color, disabled, on_click.clone()))
            })
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

    /// Padding the label should sit inside, from the button tokens.
    pub fn padding(&self) -> Insets {
        let style = self.resolved_style();
        let h = f64::from(style.padding_horizontal);
        Insets::uniform(h.max(f64::from(style.height) / 2.0))
    }
}

impl Default for MdButton {
    fn default() -> Self {
        Self::new()
    }
}

fn build_button(
    label: &str,
    color: PaintColor,
    disabled: bool,
    on_click: Option<Arc<dyn Fn() + Send + Sync>>,
) -> Button {
    let mut button = Button::new(label).color(color).disabled(disabled);
    if let Some(on_click) = on_click {
        button = button.on_click(move || on_click());
    }
    button
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

/// Foreground/background pair for a variant, honouring the disabled palette.
fn base_colors(theme: &MultiTheme, variant: ButtonVariant, disabled: bool) -> (Color, Color) {
    if disabled {
        let d = disabled_colors(theme);
        return (d.0, d.1);
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
    let c = theme.colors();
    (c.on_surface_variant, c.on_surface_variant)
}

/// The label as its own widget, for callers that want to compose by hand.
pub fn label_widget(text: &str) -> Text {
    Text::new(text)
}