//! Cupertino hover, ported to the Floom 2.0 widget API.
//!

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
