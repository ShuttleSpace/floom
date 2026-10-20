//! Material breadcrumbs, ported to the Floom 2.0 widget API.
//!
//! A breadcrumb trail is a row of labels separated by dividers, where the last
//! label is the current location and the earlier ones lead back.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for the padded box, a horizontal
//! `Stack` for the trail, a `Text` for each label and for each divider, and a
//! [`Pressable`] around each label that is not the current one. There is nothing
//! here that a widget could own, so the shape stays in the kit.
//!
//! ## What this kit answers
//!
//! * the gap between the items and the dividers, the items' size and the
//!   divider's text: the tokens;
//! * the current item's ink: `on_surface`; the other items':
//!   `on_surface_variant`; the divider's: `outline_variant`;
//! * the current item's weight: medium; the other items' and the divider's:
//!   regular -- v1 wrote the first two explicitly and the third by not writing
//!   it, which is the text's own default;
//! * the padding around the trail (8 on every side): a v1 literal.
//!
//! ## The style the port adds
//!
//! v1's `MdBreadcrumbsStyle` reported the gap, the size and the three inks, and
//! nothing else -- although the paint also used the items' weights, the
//! divider's text and the padding around the trail. The port reports all three
//! and says so here rather than leaving the style to be half the answer.
//!
//! One consequence: v1's style was `Copy` and the port's is not, because the
//! divider's text is a `String` and the port reports it. The three kits stay
//! consistent with each other; they differ from the other families' styles,
//! which have nothing stringy to report.
//!
//! ## The trail does not follow the items
//!
//! v1 held the items in an `RwSignal` and drew them through a `dyn_view`, so a
//! host that wrote the signal saw the trail change. 2.0's tree is mounted once
//! -- a slot's build closure runs when the key is new and never again -- so the
//! trail is the one the kit was built with. The kit still holds the signal: v1's
//! `items(..)` wrote it and so does the port's, but nothing observes it after
//! the mount.
//!
//! ## The accessors
//!
//! v1's breadcrumbs had none: no `props()`, no `theme()`, no `with_theme()`. The
//! port gives them `theme()` and `with_theme()`, because a kit that cannot be
//! given a theme cannot be tested against one. There is no `props()`: v1's kit
//! held no props at all -- the shared `BreadcrumbsProps` was never read by any
//! of the three kits, which read the component tokens instead -- and inventing a
//! props field the kit does not use would be a second source of truth.
//!
//! ## The one difference between the kits that the port drops
//!
//! v1's three kits disagreed about one thing here: Material's `items` took
//! `mut self` and the other two took `self`. The difference was v1's accident:
//! `RwSignal::set` takes `&self`, so `mut` was never needed, and the three
//! bodies were the same line. The port takes `self` in all three.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`** on the clickable items. 2.0's paint
//!   context has no cursor vocabulary. The items are still `Pressable`s, so they
//!   are still activatable by keyboard, which the pointer was not.
//! * **`h_stack_from_iter`** in Material's import list, which v1 never used: the
//!   trail was built with `Stack::horizontal_from_iter`. A dead import, not a
//!   behaviour.
//!
//! ## The one thing the port adds
//!
//! v1 published nothing to assistive technology. 2.0's `Pressable` does, so the
//! port gives each clickable item the link role and the item's own text as its
//! accessible name. That is a fix rather than a translation, and it is named
//! here rather than slipped in.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_accessibility::SemanticsRole;
use floom_signal::{RwSignal, SignalGet, SignalUpdate};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The padding around the trail, on every side. A v1 literal.
const PADDING: f32 = 8.0;

/// The breadcrumbs' appearance, in the terms v1 used.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialBreadcrumbsStyle {
    pub design_system: &'static str,
    /// The gap between the items and the dividers, from the token.
    pub gap: f32,
    /// The items' size, from the token.
    pub font_size: f32,
    /// The current item's ink.
    pub on_surface: Color,
    /// The other items' ink.
    pub on_surface_variant: Color,
    /// The divider's ink.
    pub outline_variant: Color,
    /// The current item's weight. v1 wrote medium.
    pub current_font_weight: FontWeight,
    /// The other items' weight. v1 wrote regular.
    pub item_font_weight: FontWeight,
    /// The divider's weight: v1 wrote none, so the text's own default.
    pub divider_font_weight: FontWeight,
    /// The divider's text, from the token.
    pub divider: String,
    /// The padding around the trail. A v1 literal.
    pub padding: f32,
}

/// Material's breadcrumbs.
pub struct MaterialBreadcrumbs {
    theme: RwSignal<MultiTheme>,
    /// The trail's labels, in v1's shape: a signal the caller can write.
    items: RwSignal<Vec<String>>,
    /// The click callback in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(usize)>`, which cannot
    /// cross that boundary.
    on_click: Option<Callback>,
}

impl MaterialBreadcrumbs {
    pub fn new() -> Self {
        Self {
            theme: theme_signal(),
            items: RwSignal::new(Vec::new()),
            on_click: None,
        }
    }

    /// The ordered breadcrumb labels, last item is the current location.
    pub fn items(self, items: Vec<String>) -> Self {
        self.items.set(items);
        self
    }

    /// Fired when a non-current item is clicked, passing its index.
    pub fn on_click(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialBreadcrumbsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Breadcrumbs(tokens) = theme.component(ComponentId::Breadcrumbs) else {
            unreachable!("ComponentId::Breadcrumbs must resolve to BreadcrumbsTokens")
        };
        let c = theme.colors();
        MaterialBreadcrumbsStyle {
            design_system: "material",
            gap: tokens.gap,
            font_size: tokens.font_size,
            on_surface: c.on_surface,
            on_surface_variant: c.on_surface_variant,
            outline_variant: c.outline_variant,
            // v1's `FontWeight::Medium` for the current item ...
            current_font_weight: FontWeight::Medium,
            // ... and `FontWeight::Regular` for the others.
            item_font_weight: FontWeight::Normal,
            // v1's divider wrote no weight at all, so the text's own default.
            divider_font_weight: FontWeight::Normal,
            divider: tokens.divider,
            padding: PADDING,
        }
    }

    /// The trail: the padded box, and the row of items and dividers inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        // Read once: 2.0's tree is mounted once.
        let items = self.items.get();
        let on_click = self.on_click.clone();

        let font_size = style.font_size;
        let count = items.len();

        let mut trail = Stack::new(Axis::Horizontal)
            .gap(style.gap)
            // v1's `.items_center()`; the main axis kept v1's default.
            .align(Align::Start, Align::Center)
            .request(LayoutRequest::new().width_pct(100.0));

        for (index, label) in items.iter().enumerate() {
            let is_last = index + 1 == count;
            let ink = to_paint(if is_last {
                style.on_surface
            } else {
                style.on_surface_variant
            });
            let weight = if is_last {
                style.current_font_weight
            } else {
                style.item_font_weight
            };
            let text = label.clone();
            // The accessible name is a second copy of the label: the text itself
            // is moved into the pressable's slot.
            let accessible = label.clone();

            if is_last {
                // v1 gave the current item no listener: it is not a link.
                trail = trail.push(move || {
                    Box::new(
                        Text::new(text.clone())
                            .size(font_size)
                            .weight(weight)
                            .color(ink),
                    )
                });
            } else {
                // v1 put a `Click` listener on every item but the last.
                let on_click = on_click.clone();
                trail = trail.push(move || {
                    let on_click = on_click.clone();
                    let text = text.clone();
                    let accessible = accessible.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new(),
                            move || {
                                Box::new(
                                    Text::new(text.clone())
                                        .size(font_size)
                                        .weight(weight)
                                        .color(ink),
                                )
                            },
                        ))
                        .role(SemanticsRole::Link)
                        .label(accessible)
                        .on_click(move || {
                            if let Some(cb) = &on_click {
                                cb(index);
                            }
                        }),
                    )
                });
            }

            if !is_last {
                // v1's divider: the token's text, in the outline variant.
                let divider = style.divider.clone();
                let divider_ink = to_paint(style.outline_variant);
                let divider_weight = style.divider_font_weight;
                trail = trail.push(move || {
                    Box::new(
                        Text::new(divider.clone())
                            .size(font_size)
                            .weight(divider_weight)
                            .color(divider_ink),
                    )
                });
            }
        }

        Surface::new()
            .request(LayoutRequest::new().width_pct(100.0))
            .padding_horiz(f64::from(style.padding))
            .padding_vert(f64::from(style.padding))
            .push(move || Box::new(trail.clone()))
    }
}

impl Default for MaterialBreadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
