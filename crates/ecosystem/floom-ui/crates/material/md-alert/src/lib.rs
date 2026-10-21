//! Material 3 alert banner, ported to the Floom 2.0 widget API.
//!
//! ## No new engine capability was needed
//!
//! This is the first of the six remaining `MdIcon::view()` callers, and it turned
//! out to need **nothing** from the engine: the banner is a [`Surface`] holding a
//! [`Stack`] of two more stacks and some [`Text`], and the dismiss affordance is a
//! bare pressable icon. Every piece already exists — which is worth recording,
//! because the six were treated as one batch and they are not: `alert` is a
//! composition, and `select` at 60% cross-kit overlap is a different thing again.
//!
//! ## What the port changed
//!
//! * `MdIcon::new("close").size(20.0).view().style(..).on_event(Click, ..)` — the
//!   `.view()` had no 2.0 meaning, and the chain after it was v1's
//!   `Decorators`. In 2.0 the icon *is* a widget, and a clickable one is that widget
//!   inside a [`Pressable`]. The v1 chain also set a pointer cursor, which 2.0's
//!   `PaintContext` has no vocabulary for; recorded rather than worked around, as it
//!   was for `md-item_group`'s rows.
//! * v1 mounted an **empty `Label`** as the dismiss slot when the alert was not
//!   dismissible, so the row always had two children. Here the slot is simply
//!   absent when there is nothing to put in it — the row has one child instead of
//!   two, and the eight-pixel gap that followed the empty label disappears with it.
//!   That is a visible difference: v1's non-dismissible banner was 8px narrower in
//!   its content than it needed to be. It is v1's bug, not a translation choice,
//!   and it is pinned by a test.
//! * `Label::new(..)` is [`Text`], `.style(|s| s.color(..))` is `.color(..)`, and
//!   `.font_weight(FontWeight::Bold.to_floom())` needed the same three-way
//!   `FontWeight` crossing that `md-list_item` documented: v1's `to_floom()` returns
//!   a `parley` weight, and 2.0's `Text::weight` takes `floom_types::FontWeight`.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};

use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::alert::{AlertProps, AlertVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;
use floom_widget_md_icon::MdIcon;


/// The theme's weight enum -> the engine's.
///
/// The same crossing `md-list_item` documents: v1's `to_floom()` returns a
/// `parley::style::FontWeight`, a third type with the same name. This names the
/// nine variants instead of reaching into parley's numeric weight.
pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        // v1's `to_floom()` sent `Black` to 800; parley's table has no 900.
        ThemeFontWeight::Black => FontWeight::Black,
    }
}

/// The accent for each variant.
///
/// Material has no built-in info / success / warning tokens, so three of these are
/// fixed RGB literals and only `error` reads the theme. Kept as v1 wrote it: the
/// literals are the design, not an oversight, and a port that resolved them through
/// the theme would change three kits' colours.
pub fn alert_accent(theme: &MultiTheme, variant: AlertVariant) -> Color {
    match variant {
        AlertVariant::Error => theme.colors().error,
        AlertVariant::Info => Color::rgb(33, 150, 243),
        AlertVariant::Success => Color::rgb(76, 175, 80),
        AlertVariant::Warning => Color::rgb(255, 152, 0),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MdAlertStyle {
    pub design_system: &'static str,
    pub accent: Color,
    pub container_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub title_font_size: f32,
}

/// v1's three fixed gaps, at the call sites where it wrote them.
const DOT_BODY_GAP: f32 = 12.0;
const BODY_TEXT_GAP: f32 = 4.0;
const CONTENT_DISMISS_GAP: f32 = 8.0;
/// The accent dot: a 10pt circle.
const DOT_SIZE: f32 = 10.0;
/// The dismiss icon's size, and the padding around it.
const DISMISS_SIZE: f32 = 20.0;
const DISMISS_PADDING: f64 = 2.0;

pub struct MdAlert {
    props: AlertProps,
    theme: RwSignal<MultiTheme>,
    /// The dismiss callback in the shape 2.0 needs.
    ///
    /// `AlertProps::on_dismiss` is `Option<Box<dyn Fn()>>` — a v1 type — and the
    /// engine's slot requires `Send + Sync`. Stored twice, from one closure; the
    /// v1-shaped copy keeps `props()` honest.
    on_dismiss: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl MdAlert {
    pub fn new() -> Self {
        Self {
            props: AlertProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            on_dismiss: None,
        }
    }

    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.props.dismissible = dismissible;
        self
    }

    pub fn on_dismiss(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_dismiss = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_dismiss = Some(shared);
        self
    }

    pub fn props(&self) -> &AlertProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdAlertStyle {
        let theme = self.theme.get();
        let accent = alert_accent(&theme, self.props.variant);
        let ComponentTokens::Alert(tokens) = theme.component(ComponentId::Alert) else {
            unreachable!("ComponentId::Alert must resolve to AlertTokens")
        };
        MdAlertStyle {
            design_system: "material",
            accent,
            // v1: `with_alpha(accent, 26)`, i.e. about 10% — a tint, not a wash.
            container_color: with_alpha(accent, 26),
            text_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            title_font_size: tokens.title_font_size,
        }
    }

    /// The banner, ready to mount.
    ///
    /// The tree, outermost first:
    ///
    /// ```text
    /// Surface            the tinted, rounded box: padding, radius, accent @ 26
    ///  └ Stack           gap 8: the content, then the dismiss slot if there is one
    ///     ├ Stack       gap 12: the accent dot, then the text column
    ///     │  ├ Surface  the dot: 10pt, radius 9999, the accent itself
    ///     │  └ Stack    gap 4: the title, then the message
    ///     └ Pressable   only when `dismissible` — the close icon
    /// ```
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let accent = to_paint(style.accent);
        let container = to_paint(style.container_color);
        let title = self.props.title.clone();
        let message = self.props.message.clone();
        let dismissible = self.props.dismissible;
        let on_dismiss = self.on_dismiss.clone();
        let title_size = style.title_font_size;
        let message_size = style.font_size;
        let title_color = to_paint(style.text_color);
        let message_color = to_paint(self.theme.get().colors().on_surface_variant);

        Surface::new()
            .request(
                LayoutRequest::new()
                    .padding_horiz(f64::from(style.padding))
                    .padding_vert(f64::from(style.padding))
                    // v1's `.items_center()` on the banner.
                    .cross_align(Align::Center),
            )
            .background(container)
            .radius(style.border_radius)
            .push({
                let accent = accent;
                move || {
                    // The content: the dot and the two lines of text.
                    let content = Stack::new(Axis::Horizontal)
                        .gap(DOT_BODY_GAP)
                        .align(Align::Start, Align::Center)
                        .push({
                            let accent = accent;
                            move || {
                                Box::new(
                                    Surface::new()
                                        .request(
                                            LayoutRequest::new()
                                                .width(DOT_SIZE)
                                                .height(DOT_SIZE),
                                        )
                                        .background(accent)
                                        // v1's `9999.0`: a circle from a rounded box.
                                        .radius(9999.0),
                                )
                            }
                        })
                        .push({
                            let title = title.clone();
                            let message = message.clone();
                            move || {
                                Box::new(
                                    Stack::new(Axis::Vertical)
                                        .gap(BODY_TEXT_GAP)
                                        .align(Align::Center, Align::Start)
                                        .push({
                                            let title = title.clone();
                                            move || {
                                                Box::new(
                                                    Text::new(title.clone())
                                                        .size(title_size)
                                                        // v1: `FontWeight::Bold.to_floom()`
                                                        .weight(FontWeight::Bold)
                                                        .color(title_color),
                                                )
                                            }
                                        })
                                        .push({
                                            let message = message.clone();
                                            move || {
                                                Box::new(
                                                    Text::new(message.clone())
                                                        .size(message_size)
                                                        .weight(FontWeight::Normal)
                                                        .color(message_color),
                                                )
                                            }
                                        }),
                                )
                            }
                        });

                    // The row: the content, and the dismiss affordance *only if there
                    // is one*. v1 mounted an empty `Label` here when the alert was not
                    // dismissible, so the row always had two children and the 8px gap
                    // was spent on nothing.
                    let mut row = Stack::new(Axis::Horizontal)
                        .gap(CONTENT_DISMISS_GAP)
                        .align(Align::Start, Align::Center)
                        .push(move || Box::new(content.clone()));

                    if dismissible {
                        row = row.push({
                            let on_dismiss = on_dismiss.clone();
                            move || {
                                let icon = MdIcon::new("close")
                                    .size(DISMISS_SIZE)
                                    .resolve();
                                Box::new(
                                    Pressable::new(Child::new(
                                        ChildKey::index(0),
                                        LayoutRequest::new()
                                            .padding_horiz(DISMISS_PADDING)
                                            .padding_vert(DISMISS_PADDING),
                                        move || Box::new(icon.clone()),
                                    ))
                                    .label("Dismiss"),
                                )
                            }
                        });
                    }

                    Box::new(row)
                }
            })
    }
}

impl Default for MdAlert {
    fn default() -> Self {
        Self::new()
    }
}
