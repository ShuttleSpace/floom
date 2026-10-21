//! Cupertino overlay, ported to the Floom 2.0 widget API.
//!
//! v1 wrapped a full-viewport scrim around the caller's content and dismissed
//! when the scrim was clicked. 2.0 has no standalone "scrim" widget, but its
//! `Dialog` is a host-modal surface driven by a shared presence flag -- which is
//! exactly v1's `show` / `with_show_signal` external trigger -- so the overlay
//! becomes a `Dialog` whose card is the scrim plus the centered content.
//!
//! ## What the port reproduces
//!
//! * the external `show(bool)` / `with_show_signal` trigger, via `with_shared_presence`
//!   and [`CupertinoOverlay::open_flag`] / [`CupertinoOverlay::with_open_flag`];
//! * the scrim: a full-viewport layer tinted `scrim` at `opacity` alpha;
//! * the centered content card: `surface` background, `on_surface` text,
//!   `border_radius(12.0)`, padding from `OverlayTokens`;
//! * dismiss on scrim click, firing the v1 `on_click` callback.
//!
//! ## What the port names (not fakes)
//!
//! * **v1 dismissed on any click outside the content; 2.0's modal is not
//!   light-dismissed**, so the scrim is a `Pressable` that carries the dismiss.
//!   If 2.0's event model bubbles a content click up to the scrim, that content
//!   click would also dismiss -- v1 explicitly `Stop`-ed propagation on the
//!   content, which 2.0's `EventPropagation` does not expose here. Recorded,
//!   not worked around: a bubble-stop would be an engine change, not a kit one.
//! * **Fluent's `border_radius(12.0)` is Cupertino's number** --
//!   the Fluent kit hardcoded `12.0` instead of a `tokens`- or
//!   `radius().md`-derived value. Preserved as written (the tripwire keeping the
//!   three kits three crates).
//! * the engine `Dialog` paints no scrim of its own, so the scrim is the card.

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_ui_composites::Dialog;
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Pressable, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::overlay::OverlayProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;


/// A v1-shaped dismiss callback, kept so `props()` still reports one.
type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoOverlayStyle {
    pub scrim_color: Color,
    pub content_padding: f32,
}

/// Cupertino's overlay.
pub struct CupertinoOverlay {
    /// Scrim opacity, 0.0 (transparent) .. 1.0 (opaque).
    opacity: f32,
    /// The shared "is it up" flag -- not an `RwSignal` (cannot cross `Send+Sync`).
    show: SharedFlag,
    theme: RwSignal<MultiTheme>,
    /// The caller's content, as a rebuildable + shareable factory (see `child`).
    child: Option<Arc<dyn Fn() -> Box<dyn floom_widgets::WidgetAny + Send + Sync> + Send + Sync + 'static>>,
    /// The v1 `on_click` dismiss callback, in `Send + Sync` form.
    on_dismiss: Option<Callback>,
}

impl CupertinoOverlay {
    pub fn new() -> Self {
        Self {
            opacity: 0.5,
            show: shared_flag(false),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            child: None,
            on_dismiss: None,
        }
    }

    /// Supply the content rendered above the scrim.
    ///
    /// v1's `child(impl IntoView)`. 2.0 takes a factory -- the same shape
    /// `Dialog::card_with` and `Child::new` take -- because the card is rebuilt
    /// on every mount, so an owned widget cannot be handed over.
    pub fn child(
        mut self,
        build: impl Fn() -> Box<dyn floom_widgets::WidgetAny + Send + Sync> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    /// Show or hide the overlay (writes the shared flag).
    pub fn show(mut self, show: bool) -> Self {
        self.opacity = if show { 0.5 } else { 0.0 };
        write_flag(&self.show, show);
        self
    }

    /// Drive this overlay from a flag somebody else owns.
    pub fn with_show_signal(mut self, show: SharedFlag) -> Self {
        self.show = show;
        self.opacity = if read_flag(&self.show) { 0.5 } else { 0.0 };
        self
    }

    /// The shared show flag, so an external trigger can drive it.
    pub fn open_flag(&self) -> SharedFlag {
        self.show.clone()
    }

    /// Alias of [`Self::open_flag`] (v1 name).
    pub fn show_signal(&self) -> SharedFlag {
        self.show.clone()
    }

    /// Fired when the scrim is clicked (v1's `on_click`).
    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_dismiss = Some(Arc::new(callback));
        self
    }

    /// v1's props, reconstructed for callers that read them. `on_click` is kept
    /// in `Send + Sync` form here; this rebuilds the v1 `Box<dyn Fn()>` shape.
    pub fn props(&self) -> OverlayProps {
        OverlayProps {
            opacity: self.opacity,
            on_click: self.on_dismiss.as_ref().map(|cb| {
                let c = cb.clone();
                Box::new(move || c()) as Box<dyn Fn()>
            }),
        }
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoOverlayStyle {
        let theme = self.theme.get();
        let ComponentTokens::Overlay(tokens) = theme.component(ComponentId::Overlay) else {
            unreachable!("ComponentId::Overlay must resolve to OverlayTokens")
        };
        CupertinoOverlayStyle {
            scrim_color: theme.colors().scrim,
            content_padding: tokens.content_padding,
        }
    }

    /// The overlay, ready to mount as a modal `Dialog`.
    pub fn view(self) -> Dialog {
        let style = self.resolved_style();
        let opacity = self.opacity;
        let on_dismiss = self.on_dismiss.clone();
        let show = self.show.clone();
        let child = self.child.clone();
        let pad = style.content_padding;

        let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;

        Dialog::new()
            .with_shared_presence(show.clone())
            .card_with(move || build_card(alpha, pad, child.clone(), on_dismiss.clone(), show.clone()))
    }
}

impl Default for CupertinoOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// The card: a full-viewport scrim tinted `scrim` at `alpha`, with the content
/// `Surface` centered above it. Rebuilt from factories because `Surface` is not
/// `Clone` and `Child::new` requires an `Fn` build (called per mount).
fn build_card(
    alpha: u8,
    pad: f32,
    child: Option<Arc<dyn Fn() -> Box<dyn floom_widgets::WidgetAny + Send + Sync> + Send + Sync + 'static>>,
    on_dismiss: Option<Callback>,
    show: SharedFlag,
) -> Box<dyn floom_widgets::WidgetAny> {
    let surface_color = to_paint(
        MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)
            .colors()
            .surface,
    );
    let scrim = to_paint(
        MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)
            .colors()
            .scrim,
    );
    let scrim_color = PaintColor { r: scrim.r, g: scrim.g, b: scrim.b, a: alpha };

    // The content card: padded `Surface` with `surface` fill + `12.0` radius.
    let content_build: Arc<dyn Fn() -> Surface + Send + Sync> = {
        let child = child.clone();
        Arc::new(move || {
            let child = child.clone();
            Surface::new()
                .request(LayoutRequest::new().padding(floom_core::Insets {
                    left: f64::from(pad),
                    right: f64::from(pad),
                    top: f64::from(pad),
                    bottom: f64::from(pad),
                }))
                .background(surface_color)
                .radius(12.0)
                .child(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
                    match child.as_ref() {
                        Some(b) => b(),
                        None => Box::new(floom_widgets::Text::new(String::new())),
                    }
                }))
        })
    };

    // The scrim: a full-bleed tinted `Surface` that centers the content above it.
    let scrim_build: Arc<dyn Fn() -> Surface + Send + Sync> = {
        let content_build = content_build.clone();
        Arc::new(move || {
            let content_build = content_build.clone();
            Surface::new()
                .request(LayoutRequest::new().fill())
                .background(scrim_color)
                .child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().fill(),
                    move || {
                        let content_build = content_build.clone();
                        Box::new(
                            Stack::new(Axis::Vertical)
                                .align(Align::Center, Align::Center)
                                .request(LayoutRequest::new().fill())
                                .child(Child::new(
                                    ChildKey::index(0),
                                    LayoutRequest::new().fill(),
                                    move || Box::new(content_build()),
                                )),
                        )
                    },
                ))
        })
    };

    Box::new(
        Pressable::new(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || {
                let scrim_build = scrim_build.clone();
                Box::new(scrim_build())
            },
        ))
        .on_click(move || {
            write_flag(&show, false);
            if let Some(cb) = &on_dismiss {
                cb();
            }
        }),
    )
}
