//! Cupertino card, ported to the Floom 2.0 widget API.
//!
//! The third card in the library, and the second one-word shape change: the card is
//! a decorated box inside a press ripple, exactly as in `fluent-card`, but it is
//! drawn at `t.radius().md` where Fluent uses `t.radius().xl` and Material uses
//! `radius().xl` too. Three kits, one shape, two different radii — which is the
//! argument for keeping the radius a *method* rather than a field.
//!
//! # What changed from v1
//!
//! ```ignore
//! // v1
//! let inner = Container::new(child).style(|s| s.background(..).color(..)
//!     .padding(..).border_radius(t.radius().md).box_shadow_*);
//! Ripple::new(inner).radius_fn(|t| t.radius().md).view()
//! ```
//!
//! * the box is a [`Surface`] — filled, rounded, elevated, holding its content,
//!   which a 2.0 `Container` cannot do because it is a leaf;
//! * the ripple is `floom_widgets::Ripple`, a *widget* rather than a view wrapper,
//!   so it takes its child as a replayed builder instead of a stored view;
//! * `.color(..)` on the box has no 2.0 home: a `Surface` paints a fill, and only
//!   the child knows it is text. See [`CupertinoCard::content_color`].
//!
//! # What is kept deliberately
//!
//! * v1 read `CardTokens::border_radius` into the style struct and never used it,
//!   drawing the box at `t.radius().md` instead. Both are kept, and they **coincide
//!   for Cupertino** — the token says `10.0` and `radius().md` says `10.0`. They are
//!   still separate methods because they do *not* coincide for Material
//!   (`12.0` against `24.0`), and one shape serves all three kits. See
//!   `tests/card.rs::material_is_the_kit_where_the_token_and_the_drawn_radius_disagree`.
//! * v1's default child was `Empty::new()`, i.e. "a box with nothing in it". A
//!   childless `Surface` is already that, so the port has no child rather than an
//!   empty one. [`CupertinoCard::surface`]'s child count is the assertion.
//! * `BLACK` comes from `floom_core_theme::tokens::color_ext`, as it did in v1
//!   through the glob import. It is a shared token, not a per-crate constant, and
//!   `fluent-card`'s port briefly redefined it locally before this one corrected it.

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Ripple, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::card::CardProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::shadow::ShadowData;

/// Translate a design-system shadow token into the engine-neutral `Shadow`.
///
/// The same translation `md-card` and `fluent-card` use, duplicated rather than
/// shared because the three crates are separate packages that may be ported
/// independently — a shared helper would be a dependency *between kits*, which is
/// exactly the coupling this port exists to remove.
fn shadow_from(data: ShadowData) -> Shadow {
    Shadow {
        color: to_paint(with_alpha(BLACK, data.color_a)),
        blur: f64::from(data.blur),
        offset_x: f64::from(data.offset_x),
        offset_y: f64::from(data.offset_y),
        spread: f64::from(data.spread),
    }
}

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoCardStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct CupertinoCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
    child:
        Option<std::sync::Arc<dyn Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>>,
}

impl CupertinoCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
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

    pub fn resolved_style(&self) -> CupertinoCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        CupertinoCardStyle {
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
    /// `t.radius().md`, *not* [`Self::resolved_style`]'s `border_radius`. v1 read
    /// the token into the style struct without using it, so keeping the two apart
    /// is what makes the translation honest. For Cupertino they coincide; for
    /// Material they do not, which is the case that would break if they were merged.
    pub fn corner_radius(&self) -> f32 {
        self.theme.get().radius().md
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    /// The colour the card's content should be drawn in.
    ///
    /// v1 set this on the box as `color(..)`, which in 2.0 has nowhere to live: a
    /// `Surface` paints a fill, and only the child knows it is text.
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
    /// different widgets, and a caller composing by hand should not have to unwrap
    /// a ripple to get the card.
    pub fn surface(&self) -> Surface {
        surface_from(
            to_paint(self.resolved_style().container_color),
            self.corner_radius(),
            self.padding(),
            self.shadow(),
        )
    }

    /// The card as a pressable: the surface, the content inside it, and a ripple
    /// over the whole thing clipped to the card's rounded bounds.
    ///
    /// The surface is rebuilt *inside* the ripple's slot rather than captured,
    /// because a 2.0 slot is a replayed `Fn` and cannot hand out the same widget
    /// twice. So the slot captures plain data — a colour, three lengths and a
    /// `Shadow` — and composes the box on every rebuild, which is also why
    /// [`Self::content_color`] is asked of the caller rather than baked in: a
    /// rebuilt surface cannot know what its child is.
    ///
    /// The ripple's radius is resolved here rather than through v1's
    /// `radius_fn(|t| t.radius().md)`, because the 2.0 ripple takes a fixed radius
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
fn surface_from(background: PaintColor, radius: f32, padding: Insets, shadow: Shadow) -> Surface {
    Surface::new()
        .background(background)
        .radius(radius)
        .padding(padding)
        .shadow(shadow)
}

impl Default for CupertinoCard {
    fn default() -> Self {
        Self::new()
    }
}
