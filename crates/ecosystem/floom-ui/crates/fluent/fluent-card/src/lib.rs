//! Fluent card, ported to the Floom 2.0 widget API.
//!
//! The second card in the library, and the first one where a *ported* shape had to
//! grow: `md-card` is a decorated box, and this one is a decorated box **inside a
//! press ripple**. So it is the case that proves `Surface` composes — a card you can
//! press is still a card you can hold content in.
//!
//! # What changed from v1
//!
//! ```ignore
//! // v1
//! let inner = Container::new(child).style(|s| s.background(..).padding(..)
//!     .border_radius(t.radius().xl).box_shadow_*);
//! Ripple::new(inner).radius_fn(|t| t.radius().xl).view()
//! ```
//!
//! * the box is a [`Surface`] — filled, rounded, elevated, and holding its content,
//!   which a 2.0 `Container` cannot do because it is a leaf;
//! * the ripple is `floom_widgets::Ripple`, a *widget* rather than a view
//!   wrapper, so it takes its child as a replayed builder instead of a stored view;
//! * `.color(..)` on the box has no 2.0 home, again: the box paints a fill and only
//!   the child knows it is text. See [`FluentCard::content_color`].
//!
//! # What is kept deliberately
//!
//! * The card's corner radius comes from `t.radius().xl` — the *theme's* radius
//!   scale — not from `CardTokens::border_radius`, which v1 also read into
//!   [`FluentCardStyle::border_radius`] but never used for the box. Both are kept:
//!   the style struct reports what v1 reported, and the box uses what v1 drew.
//! * The ripple's radius is resolved **once, here**, rather than through a
//!   `radius_fn(|t| ..)` re-read on every render. `floom_widgets::Ripple` takes a
//!   fixed radius for the same reason it takes a fixed ink: it is a value in a
//!   builder. See the note on `md-card` about theme timing.

use floom_legacy_reactive::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Ripple, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::card::CardProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::shadow::ShadowData;

/// A builder for the card's content.
///
/// An [`std::sync::Arc`] because a 2.0 slot is a replayed `Fn`: every rebuild needs
/// its own copy to move into it.
pub type ContentBuilder =
    std::sync::Arc<dyn Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Translate a design-system shadow token into the engine-neutral `Shadow`.
///
/// The same translation `md-card` uses, and duplicated rather than shared because
/// the two crates are separate packages that may be ported independently — a
/// shared helper would be a dependency between kits, which is exactly the coupling
/// the port exists to remove.
fn shadow_from(data: ShadowData) -> Shadow {
    Shadow {
        color: to_paint(with_alpha(BLACK, data.color_a)),
        blur: f64::from(data.blur),
        offset_x: f64::from(data.offset_x),
        offset_y: f64::from(data.offset_y),
        spread: f64::from(data.spread),
    }
}

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentCardStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct FluentCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ContentBuilder>,
}

impl FluentCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
        self
    }

    /// Set the card's content.
    ///
    /// v1 took `impl IntoView`; 2.0 takes a replayed builder. Callers holding a
    /// widget already write `move || Box::new(..)`.
    pub fn child(
        mut self,
        child: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(std::sync::Arc::new(child));
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

    pub fn resolved_style(&self) -> FluentCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        FluentCardStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }

    /// The corner radius the box is actually drawn with.
    ///
    /// Not [`Self::resolved_style`]'s `border_radius`: v1 drew `t.radius().xl` and
    /// read `tokens.border_radius` into the style struct without using it, so
    /// keeping the two apart is what makes the translation honest. For Fluent the
    /// two happen to differ, which is how the difference was noticed.
    pub fn corner_radius(&self) -> f32 {
        self.theme.get().radius().xl
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    /// The colour the card's content should be drawn in.
    ///
    /// v1 set this on the box as `color(..)`, which in 2.0 has nowhere to live:
    /// a `Surface` paints a fill, and only the child knows it is text.
    pub fn content_color(&self) -> PaintColor {
        to_paint(self.resolved_style().content_color)
    }

    /// The 2.0 [`Shadow`] for this card's resolved style.
    pub fn shadow(&self) -> Shadow {
        shadow_from(self.resolved_style().shadow)
    }

    /// The card's surface — the filled, rounded, elevated box with the card's
    /// padding on it — **without** the press ripple.
    ///
    /// Separate from [`Self::view`] because "the box" and "the pressable box" are
    /// different widgets, and a caller composing by hand should not have to
    /// unwrap a ripple to get the card.
    pub fn surface(&self) -> Surface {
        let style = self.resolved_style();
        surface_from(
            to_paint(style.container_color),
            self.corner_radius(),
            self.padding(),
            self.shadow(),
        )
    }

    /// The card as a pressable: the surface, the content inside it, and a ripple
    /// over the whole thing clipped to the card's rounded bounds.
    ///
    /// This is the difference from `md-card`, and it is why `Surface` had to be a
    /// widget with children rather than a leaf: a pressable card is still a card
    /// that holds its content, and swapping which half is composed is the whole
    /// translation.
    ///
    /// The surface is rebuilt *inside* the ripple's slot rather than captured,
    /// because a 2.0 slot is a replayed `Fn` and cannot hand out the same widget
    /// twice. So the slot captures plain data — a colour, three lengths and a
    /// `Shadow` — and composes the box on every rebuild, which is also why
    /// [`Self::content_color`] is asked of the caller rather than baked in: a
    /// rebuilt surface cannot know what its child is.
    ///
    /// The ripple's radius is resolved here rather than through v1's
    /// `radius_fn(|t| t.radius().xl)`, because the 2.0 ripple takes a fixed radius
    /// in its builder. Both resolve from the same theme value at the same moment,
    /// so they cannot disagree — but neither re-reads the theme afterwards.
    pub fn view(self) -> Ripple {
        let style = self.resolved_style();
        let background = to_paint(style.container_color);
        let radius = self.corner_radius();
        let padding = self.padding();
        let shadow = self.shadow();
        // Last, because it moves.
        let content = self.child;

        Ripple::new()
            .child(floom_core::LayoutRequest::new(), move || {
                let mut surface = surface_from(background, radius, padding, shadow);
                if let Some(content) = &content {
                    surface = surface.push({
                        let content = content.clone();
                        move || content()
                    });
                }
                Box::new(surface)
            })
            .radius(f64::from(radius))
    }
}

/// The filled, rounded, elevated box — from plain data, because two callers need
/// it and one of them is a replayed slot that cannot capture a built widget.
fn surface_from(
    background: PaintColor,
    radius: f32,
    padding: Insets,
    shadow: Shadow,
) -> Surface {
    Surface::new()
        .background(background)
        .radius(radius)
        .padding(padding)
        .shadow(shadow)
}

impl Default for FluentCard {
    fn default() -> Self {
        Self::new()
    }
}
