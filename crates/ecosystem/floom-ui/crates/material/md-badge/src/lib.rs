//! Material 3 badge, ported to the Floom 2.0 widget API.
//!
//! v1 wrapped a child with an absolutely-positioned bubble at the top-right
//! corner. 2.0's `Badge` is the bubble itself -- a `Pressable`-free status
//! indicator that paints a dot or a text pill at explicit values. So what is left
//! is the token translation: read the palette (`error` / `on_error`) and the
//! `BadgeTokens`, hand them to `Badge`, and return it.
//!
//! ## What v1 drew that this port cannot
//!
//! **The overlay over a child.** v1's `view()` built `Stack::new((child, bubble))`
//! and positioned the bubble with `translate_x/y` at the child's top-right. The
//! engine `Badge` has no concept of "attached to a child" -- it is the bubble.
//! A kit that wants the overlay behaviour composes `Badge` over its own child,
//! the way v1 did, which is a layout decision the engine leaves to whoever mounts
//! it. Reported rather than re-implemented: a `child` field on `Badge` would be a
//! v1 layout opinion the engine deliberately does not answer.
//!
//! ## What this kit answers, and what the other two answer instead
//!
//! | | bubble colour | text colour |
//! | --- | --- | --- |
//! | Material | `error` | `on_error` |
//! | Cupertino | `error` | `on_error` |
//! | Fluent | `primary` | `on_primary` |
//!
//! Material and Cupertino are identical here, which is the only reason both are
//! kept: they are already the same component, and the badge is the simplest kit
//! to show that the engine widget plus a two-line palette is the whole port.

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::badge::BadgeProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::Badge;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdBadgeStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub font_size: f32,
    pub min_size: f32,
    pub padding: f32,
    pub dot_size: f32,
}

pub struct MdBadge {
    props: BadgeProps,
    theme: RwSignal<MultiTheme>,
}

impl MdBadge {
    pub fn new() -> Self {
        Self {
            props: BadgeProps {
                content: String::new(),
                dot: false,
            },
            theme: theme_signal(),
        }
    }

    /// Set the text shown inside the badge. Ignored when `dot` is set.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }

    /// Render as a plain dot with no text.
    pub fn dot(mut self, dot: bool) -> Self {
        self.props.dot = dot;
        self
    }

    pub fn props(&self) -> &BadgeProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdBadgeStyle {
        let theme = self.theme.get();
        let ComponentTokens::Badge(tokens) = theme.component(ComponentId::Badge) else {
            unreachable!("ComponentId::Badge must resolve to BadgeTokens")
        };
        MdBadgeStyle {
            design_system: "material",
            bubble_color: theme.colors().error,
            text_color: theme.colors().on_error,
            font_size: tokens.font_size,
            min_size: tokens.min_size,
            padding: tokens.padding,
            dot_size: tokens.dot_size,
        }
    }

    /// Build a renderable Material 3 badge bubble.
    pub fn view(self) -> Badge {
        let style = self.resolved_style();
        let mut badge = Badge::new()
            .content(self.props.content)
            .dot(self.props.dot)
            .background(to_paint(style.bubble_color))
            .foreground(to_paint(style.text_color))
            .font_size(style.font_size)
            .min_size(style.min_size)
            .padding(style.padding)
            .dot_size(style.dot_size);
        // `Badge::new()` already sets `radius: f32::INFINITY`, which lets the
        // engine compute the pill radius (half height) and the dot radius (half
        // its square side) -- v1 did `min_size / 2.0` and `dot_size / 2.0` by
        // hand; the engine does the same without a knob, so no radius is set.
        badge
    }
}

/// List colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor::rgba(c.r, c.g, c.b, c.a)
}
