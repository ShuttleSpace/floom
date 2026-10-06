//! Fluent icon button, ported to the Floom 2.0 widget API.
//!
//! ## Why this crate is now forty lines
//!
//! All three kits ship this component and they are the same component: **69 of 73
//! substantive lines are identical across `md-icon_button`,
//! `cupertino_icon_button` and `fluent_icon_button`**, and the four that differ are
//! the type name, the `DesignSystem` handed to `MultiTheme`, and whether the theme
//! came from `theme_signal()` or a fresh `MultiTheme`.
//!
//! So the shape went to the engine as [`floom_widgets::IconButton`] — the
//! composition (`Ripple(Pressable(Surface(icon)))`), the disabled rule, the
//! ripple-ink rule, and the fact that the glyph follows `disabled` while its size
//! does not. What is left here is the translation from Fluent's tokens to an
//! [`IconButtonStyle`], plus the one thing the engine cannot know: that `"close"`
//! is a path.
//!
//! ## What the port changed
//!
//! * `svg(icon)` — a v1 view that drew a path — is [`MdIcon`], already ported, and
//!   `MdIcon::resolve()` is what hands 2.0 a widget. Its `resolved_style` colour is
//!   *overridden* below, because v1 computed the glyph colour from the button's
//!   disabled state and `MdIcon` has no notion of being inside a disabled button.
//! * `.action(..)` became [`Pressable`] inside the engine widget. v1 wrote
//!   `if !disabled { .. }` inside the action closure, which guarded the pointer but
//!   not the keyboard; 2.0's gate is in `Pressable` and covers both. That is an
//!   improvement, not a translation, and the engine suite is what keeps it.
//! * `tokens.padding` is declared in `IconButtonTokens` and **never read by v1** —
//!   the box is pinned to `size` and the icon is centred, so there is nothing for
//!   padding to separate. Left unread deliberately; see the note on
//!   `ToolbarTokens::height` in the porting plan for the same decision elsewhere.

use floom_legacy_reactive::{RwSignal, SignalGet};
use std::sync::Arc;

use floom_types::Color as PaintColor;

use floom_widgets::{IconButton, IconButtonPaint, IconButtonStyle, IconButtonVariant};
use floom_widget_md_icon::MdIcon;

use floom_widget_md_icon::MdIconStyle;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon_button::{IconButtonProps, IconButtonVariant as PropsVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Fully transparent, which is what the text variant's fill and outline both are.
const CLEAR: Color = Color { r: 0, g: 0, b: 0, a: 0 };

/// Fluent's icon button, resolved to plain data.
///
/// The four `props` v1 read are all it needs: the icon's name, an optional
/// explicit size, the variant, and `disabled`. Nothing here is stateful — v1 held
/// no open state, and the press ripple's phase lives in the engine widget.
pub struct FluentIconButton {
    props: IconButtonProps,
    theme: RwSignal<MultiTheme>,
    /// The same callback as [`Self::props`]'s, in the shape 2.0 needs.
    ///
    /// It has to be held twice. `IconButtonProps::on_click` is
    /// `Option<Box<dyn Fn()>>` — a v1 type, shared with the other two kits and
    /// deliberately not changed here — and the engine's slot requires
    /// `Send + Sync`, because the click lands inside a replayed builder closure the
    /// host may move between threads. A `Box<dyn Fn()>` cannot be widened back, so
    /// the `Send + Sync` handle is kept alongside and this is the one that is
    /// mounted. Recorded rather than hidden: it is the last v1-shaped field in this
    /// crate, and it is a duplicate of something already stored.
    on_click: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl FluentIconButton {
    pub fn new(icon: impl Into<String>) -> Self {
        Self {
            props: IconButtonProps {
                icon: icon.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_click: None,
        }
    }

    /// v1's `.size(..)`: replaces the token rather than scaling it, and leaves the
    /// icon's own token size alone.
    pub fn size(mut self, size: f32) -> Self {
        self.props.size = Some(size);
        self
    }

    pub fn variant(mut self, variant: PropsVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        // Both handles, one callback. An `Arc<dyn Fn()>` is *not* itself callable in
        // Rust's current `Fn` impls, so the props get a thin closure over the shared
        // handle rather than a widened one; the engine gets the `Send + Sync`
        // handle, which is the one that gets mounted.
        self.props.on_click = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_click = Some(shared);
        self
    }

    pub fn props(&self) -> &IconButtonProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The variant, in the engine's vocabulary.
    ///
    /// The two enums are separate types on purpose: `floom-core-theme`'s is part of
    /// the props contract a caller writes, and the engine's is part of the widget
    /// contract it mounts. Translating here means a kit can change one without the
    /// other silently moving.
    pub fn engine_variant(&self) -> IconButtonVariant {
        match self.props.variant {
            PropsVariant::Text => IconButtonVariant::Text,
            PropsVariant::Contained => IconButtonVariant::Contained,
            PropsVariant::Outlined => IconButtonVariant::Outlined,
        }
    }

    /// Fluent's tokens, as the engine's style.
    ///
    /// This is the whole port. Every field is a read v1 also made, at the same
    /// moment, from the same theme — the engine widget's own defaults are never
    /// relied on, so a token change shows up here rather than as a silent fallback.
    pub fn resolved_style(&self) -> IconButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::IconButton(tokens) = theme.component(ComponentId::IconButton) else {
            unreachable!("ComponentId::IconButton must resolve to IconButtonTokens")
        };
        let on_surface = theme.colors().on_surface;

        IconButtonStyle {
            // v1: `let btn_size = size.unwrap_or(tokens.size);`
            size: self.props.size.unwrap_or(tokens.size),
            icon_size: tokens.icon_size,
            radius: tokens.radius,
            // v1 wrote `.border(1.0)` unconditionally, for all three variants. The
            // text variant's *colour* is transparent, so what 1.0 buys there is
            // nothing visible — which is why this is the token-independent constant
            // it is, and why the engine takes it as a field rather than assuming it.
            border_width: 1.0,

            // v1: the icon's `fg`, enabled branch.
            foreground: to_paint(theme.colors().on_surface_variant),
            // v1: the icon's `fg`, disabled branch.
            disabled_foreground: to_paint(with_alpha(on_surface, 38)),
            // v1: the `(bg, border)` disabled override, which replaced the
            // variant's pair outright.
            disabled_background: to_paint(with_alpha(on_surface, 12)),
            disabled_border: to_paint(with_alpha(on_surface, 38)),

            // v1's three `(bg, border)` pairs and its ripple `color(..)`.
            text: IconButtonPaint {
                background: to_paint(CLEAR),
                border: to_paint(CLEAR),
                ripple_ink: to_paint(with_alpha(on_surface, 24)),
            },
            contained: IconButtonPaint {
                background: to_paint(theme.colors().primary),
                border: to_paint(theme.colors().primary),
                ripple_ink: to_paint(with_alpha(theme.colors().on_primary, 24)),
            },
            outlined: IconButtonPaint {
                background: to_paint(CLEAR),
                border: to_paint(theme.colors().outline),
                ripple_ink: to_paint(with_alpha(on_surface, 24)),
            },
        }
    }

    /// The icon's resolved style, for a caller that wants to read the button's
    /// colours rather than mount it.
    pub fn icon_style(&self) -> MdIconStyle {
        MdIcon::new(self.props.icon.clone())
            .size(self.resolved_style().icon_size)
            .with_theme(self.theme.get())
            .resolved_style()
    }

    /// The button, ready to mount.
    ///
    /// `view` rather than `into_any`: 2.0 widgets are composed by the *parent*
    /// declaring them, so the terminal call that used to turn a builder into a
    /// `View` has no 2.0 meaning. A caller writes
    /// `Child::new(key, request, move || Box::new(button.view()))`.
    ///
    /// The style is resolved *here*, once, and the icon's colour is overridden to
    /// the button's disabled state — `MdIcon::resolve` would otherwise use the
    /// icon's own idea of the foreground, which knows nothing about being inside a
    /// button that is currently disabled.
    pub fn view(self) -> IconButton {
        let style = self.resolved_style();
        let icon_color = if self.props.disabled {
            style.disabled_foreground
        } else {
            style.foreground
        };
        let icon = MdIcon::new(self.props.icon.clone())
            .size(style.icon_size)
            .with_theme(self.theme.get())
            .resolve()
            .color(icon_color);

        let mut button = IconButton::new(icon)
            .variant(self.engine_variant())
            .disabled(self.props.disabled)
            .style(style);

        if let Some(on_click) = &self.on_click {
            let on_click = on_click.clone();
            button = button.on_click(move || on_click());
        }

        button
    }
}
