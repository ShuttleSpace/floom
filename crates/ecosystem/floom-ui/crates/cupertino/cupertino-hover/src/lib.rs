//! Cupertino hover, ported to the Floom 2.0 widget API.
//!
//! A hover is a box that fills while the pointer is over it, and tells the
//! caller when that happens.
//!
//! ## What the port uses
//!
//! The engine's [`HoverBox`]: a box that fills while the pointer is over it,
//! around the caller's child. The engine grew it for this -- the engine's
//! `Chip` paints its own hover fill because a chip *is* the widget, and here
//! the widget is a box around somebody else's content.
//!
//! The *hover* is shared state: a [`SharedFlag`], the same
//! bool-behind-an-`Arc<Mutex<_>>` the engine's `TextInput` and the floem-ui
//! menus use, because a value that changes after mount cannot be held in a
//! signal.
//! ## What this kit answers
//!
//! * the fill under the pointer: `on_surface` at the token's `highlight_alpha`;
//! * the alpha itself: the token.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The transition.** v1's style reported a `transition_ms` and nothing ever
//!   read it: the fill appeared and disappeared instantly. The port reports the
//!   number, because v1's style did, and the port does not animate either.
//! * **`hovered_signal()` was an `RwSignal<bool>`.** 2.0's shared state is a
//!   [`SharedFlag`], so the kit hands out `hovered_flag()` and takes one back
//!   through `with_hovered_flag()`.
//! * **The child was an `IntoView`.** 2.0's slots are replayed `Fn`s and a
//!   widget is mounted by its parent, so the caller hands over a builder.
//! * **`on_hover_change` was a `Box<dyn Fn(bool)>`.** 2.0's slots are shared
//!   between threads, so the callback is `Fn(bool) + Send + Sync` and the kit
//!   keeps it as an `Arc` of its own; the props' slot stays empty.
//!
//! ## The one difference inside the callback
//!
//! v1 reported *every* enter and *every* leave, including one that changed
//! nothing. The port reports a change, so a pointer that enters twice reports
//! once.
//!
//! ## The one thing v1's style did not have
//!
//! `design_system`. The other two kits' styles name theirs and this one's did
//! not; the port gives all three the same field. The port also reports
//! `highlight_color` -- the colour the fill is actually painted in -- which
//! v1's style did not, because v1's style reported the alpha and left the
//! colour to the theme.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::HoverBox;
use floom_widgets::{read_flag, shared_flag, SharedFlag};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::hover::HoverProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// A builder for the box's content.
///
/// v1 took an `IntoView`; 2.0's slots are replayed `Fn`s and a widget is
/// mounted by its parent, so the caller hands over a builder.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// A callback in the shape 2.0's slots need.
///
/// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
/// `Child` closure the host may hand between threads, so the callback is
/// shared as an `Arc`.
type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

/// The hover's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoHoverStyle {
    pub design_system: &'static str,
    /// The fill under the pointer. v1's style did not report it.
    pub highlight_color: Color,
    /// The alpha of that fill.
    pub highlight_alpha: f32,
    /// v1 never animated anything, so this never showed.
    pub transition_ms: f32,
}

/// Cupertino's hover.
pub struct CupertinoHover {
    props: HoverProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    /// Whether the pointer is over the box. Shared with whoever drives it.
    hovered: SharedFlag,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// `HoverProps::on_hover_change` stays empty: it is a `Box<dyn Fn(bool)>`,
    /// which cannot cross a slot boundary.
    on_hover_change: Option<Callback>,
}

impl CupertinoHover {
    pub fn new() -> Self {
        Self {
            props: HoverProps::default(),
            child: None,
            // v1's Cupertino hover read a snapshot of its own design
            // system's theme, not the app-wide signal.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            hovered: shared_flag(false),
            on_hover_change: None,
        }
    }

    /// The element that reacts to the pointer.
    pub fn child(mut self, child: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(child));
        self
    }

    /// Fired whenever the hover state changes (true on enter, false on leave).
    pub fn on_hover_change(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_hover_change = Some(Arc::new(callback));
        self
    }

    /// The shared hover flag, so the host can drive a custom appearance (e.g.
    /// reveal an overlay) without the component imposing it.
    ///
    /// v1's `hovered_signal()` was an `RwSignal<bool>`; 2.0's shared state is a
    /// [`SharedFlag`], because a value that changes after mount cannot be held
    /// in a signal.
    pub fn hovered_flag(&self) -> SharedFlag {
        self.hovered.clone()
    }

    /// Use a flag the caller already has, so the thing that watches the hover
    /// and the thing that reports it share one truth.
    pub fn with_hovered_flag(mut self, hovered: SharedFlag) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn props(&self) -> &HoverProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoHoverStyle {
        let theme = self.theme.get();
        let ComponentTokens::Hover(tokens) = theme.component(ComponentId::Hover) else {
            unreachable!("ComponentId::Hover must resolve to HoverTokens")
        };
        CupertinoHoverStyle {
            design_system: "cupertino",
            // v1's paint: `with_alpha(on_surface, highlight_alpha as u8)`.
            highlight_color: with_alpha(theme.colors().on_surface, tokens.highlight_alpha as u8),
            highlight_alpha: tokens.highlight_alpha,
            transition_ms: tokens.transition_ms,
        }
    }

    /// The box: the child, and the fill that appears while the pointer is over
    /// it.
    pub fn view(self) -> HoverBox {
        let style = self.resolved_style();
        let child = match self.child {
            Some(child) => Child::new(ChildKey::index(0), LayoutRequest::new(), move || child()),
            // v1's empty child was an `Empty` view; an empty `Text` paints
            // nothing and asks for no space, which is the same thing.
            None => Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(floom_widgets::Text::new(""))
            }),
        };
        let mut hover_box = HoverBox::new()
            .child(child)
            .highlight(to_paint(style.highlight_color))
            .hovered_flag(self.hovered);
        if let Some(callback) = self.on_hover_change {
            hover_box = hover_box.on_hover_change(move |hovered| callback(hovered));
        }
        hover_box
    }
}

impl Default for CupertinoHover {
    fn default() -> Self {
        Self::new()
    }
}
