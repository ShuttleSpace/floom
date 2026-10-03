//! Material 3 card, ported to the Floom 2.0 widget API.
//!
//! This is the **reference port** for the floom-ui component library: the shape
//! every other component follows. See `docs/porting-floem-ui-to-2.0.md`.
//!
//! # What changed from v1
//!
//! v1 built a view by composing and styling:
//!
//! ```ignore
//! Container::new(child).style(|s| s.background(..).padding(..).box_shadow_..())
//! ```
//!
//! 2.0 has no style closure and no `IntoView`. `Container` is a *leaf* widget —
//! it paints a background, radius, padding, shadow and filters, and holds no
//! children — because composition moved to the parent:
//!
//! ```ignore
//! Child::new(ChildKey::index(0), LayoutRequest::new(), || Box::new(Text::new("..")))
//! ```
//!
//! So this port resolves the theme to a style struct (unchanged, and still the
//! valuable part) and then hands back a configured `Container`. Mounting
//! content is the caller's job. That is not a workaround; it is the 2.0 model,
//! and it is why the kits get *thinner* as they port.
//!
//! `Shadow` gained `spread` (CSS's fourth `box-shadow` length) to match v1's
//! `box_shadow_spread`, which this card used.

use floom_legacy_reactive::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::Container;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::card::CardProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::shadow::ShadowData;
use floom_widget_runtime::theme_signal;

/// Opaque black, the base every elevation shadow tints.
const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// Design-system colour -> engine colour.
///
/// Both are plain `{r, g, b, a}` byte structs, so this is a field copy. It
/// exists so the conversion is written once: `Color::to_peniko` goes the other
/// way (to peniko), and the engine's `Color` is not that type.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdCardStyle {
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct MdCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
}

impl MdCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
        self
    }

    pub fn props(&self) -> &CardProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        MdCardStyle {
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }

    /// The 2.0 `Shadow` for this card's resolved style.
    ///
    /// Kept separate from [`MdCard::surface`] so a caller that composes the card
    /// itself (rather than using `Container`) can reuse the same shadow.
    pub fn shadow(&self) -> Shadow {
        shadow_from(self.resolved_style().shadow)
    }

    /// The card's surface as a 2.0 widget.
    ///
    /// Mount content into it from the parent, e.g.
    /// `Child::new(key, LayoutRequest::new(), || Box::new(Text::new("hi")))`.
    pub fn surface(&self) -> Container {
        let style = self.resolved_style();
        Container::new()
            .background(to_paint(style.container_color))
            .radius(style.border_radius)
            .padding(Insets::uniform(f64::from(style.padding)))
            .shadow(shadow_from(style.shadow))
    }
}

impl Default for MdCard {
    fn default() -> Self {
        Self::new()
    }
}

/// Translate a design-system shadow token into the engine-neutral `Shadow`.
///
/// v1 spelled this across six `box_shadow_*` style calls; the 2.0 renderer
/// takes one value. `spread` is carried through — it dilates the silhouette
/// before the blur, and is honoured by `floom-renderer-software`.
fn shadow_from(data: ShadowData) -> Shadow {
    Shadow {
        color: to_paint(with_alpha(BLACK, data.color_a)),
        blur: f64::from(data.blur),
        offset_x: f64::from(data.offset_x),
        offset_y: f64::from(data.offset_y),
        spread: f64::from(data.spread),
    }
}