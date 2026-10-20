//! Material hover, ported to the Floom 2.0 widget API.
//!
//! A hover is a box that fills while the pointer is over it, and tells the
//! caller when that happens.
//!
//! ## What the port uses
//!
//! The engine's vocabulary, and one thing of its own. The box is a `Surface`-ish
//! fill over the caller's child, and the *hover* is shared state: a
//! [`SharedFlag`], the same bool-behind-an-`Arc<Mutex<_>>` the engine's
//! `TextInput` and the floem-ui menus use, because a value that changes after
//! mount cannot be held in a signal.
//!
//! ## The engine gap
//!
//! **The engine has no "fills while the pointer is over it" widget.** The
//! engine's `Chip` paints its own hover fill because a chip *is* the widget;
//! here the widget is a box around somebody else's content, and the engine has
//! nothing for that. The kit holds one, and this paragraph is the record of
//! that. The engine should grow one.
//!
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

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    PointerPhase, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{SharedFlag, read_flag, shared_flag, write_flag};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::hover::HoverProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

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
pub struct MaterialHoverStyle {
    pub design_system: &'static str,
    /// The fill under the pointer. v1's style did not report it.
    pub highlight_color: Color,
    /// The alpha of that fill.
    pub highlight_alpha: f32,
    /// v1 never animated anything, so this never showed.
    pub transition_ms: f32,
}

/// Material's hover.
pub struct MaterialHover {
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

impl MaterialHover {
    pub fn new() -> Self {
        Self {
            props: HoverProps::default(),
            child: None,
            theme: theme_signal(),
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
    pub fn on_hover_change(
        mut self,
        callback: impl Fn(bool) + Send + Sync + 'static,
    ) -> Self {
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

    pub fn resolved_style(&self) -> MaterialHoverStyle {
        let theme = self.theme.get();
        let ComponentTokens::Hover(tokens) = theme.component(ComponentId::Hover) else {
            unreachable!("ComponentId::Hover must resolve to HoverTokens")
        };
        MaterialHoverStyle {
            design_system: "material",
            // v1's paint: `with_alpha(on_surface, highlight_alpha as u8)`.
            highlight_color: with_alpha(theme.colors().on_surface, tokens.highlight_alpha as u8),
            highlight_alpha: tokens.highlight_alpha,
            transition_ms: tokens.transition_ms,
        }
    }

    /// The box: the child, and the fill that appears while the pointer is over
    /// it.
    pub fn view(self) -> MaterialHoverBox {
        let style = self.resolved_style();
        let child = match self.child {
            Some(child) => Child::new(ChildKey::index(0), LayoutRequest::new(), move || child()),
            // v1's empty child was an `Empty` view; an empty `Text` paints
            // nothing and asks for no space, which is the same thing.
            None => Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(floom_widgets::Text::new(""))
            }),
        };
        MaterialHoverBox {
            child,
            hovered: self.hovered,
            highlight: to_paint(style.highlight_color),
            on_hover_change: self.on_hover_change,
            last_hovered: false,
        }
    }
}

impl Default for MaterialHover {
    fn default() -> Self {
        Self::new()
    }
}

/// The box that fills while the pointer is over it.
///
/// The engine has no "fills while the pointer is over it" widget, so the kit
/// holds one -- see the module documentation.
pub struct MaterialHoverBox {
    child: Child,
    /// Whether the pointer is over the box.
    hovered: SharedFlag,
    /// The fill under the pointer.
    highlight: PaintColor,
    on_hover_change: Option<Callback>,
    /// What the flag was when this widget last acted, so a change is a change
    /// rather than a fresh read every frame.
    last_hovered: bool,
}

impl MaterialHoverBox {
    /// Whether the pointer is over the box right now.
    pub fn is_hovered(&self) -> bool {
        read_flag(&self.hovered)
    }

    /// The shared hover flag, so a caller can drive it from outside.
    pub fn hovered_flag(&self) -> SharedFlag {
        self.hovered.clone()
    }

    /// Writes the flag and reports the change.
    fn set_hovered(&mut self, hovered: bool) {
        if !write_flag(&self.hovered, hovered) {
            return;
        }
        if let Some(callback) = &self.on_hover_change {
            callback(hovered);
        }
    }
}

impl Widget for MaterialHoverBox {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let hovered = read_flag(&self.hovered);
        if hovered == self.last_hovered {
            return;
        }
        self.last_hovered = hovered;
        // The fill is this box's own decoration, so this box is what has to
        // invalidate it. A repaint and nothing else: the box does not move.
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        vec![self.child.clone()]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        None
    }

    /// The fill under the pointer, under the child.
    ///
    /// A widget paints itself before its children, so the fill lands *under*
    /// the content -- the way v1's `Container` background did.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if !read_flag(&self.hovered) {
            return;
        }
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, self.highlight);
    }

    /// The pointer arriving and leaving.
    ///
    /// The event is not consumed: v1's hover let the event through, so a
    /// click on the child still reached the child.
    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => self.set_hovered(true),
            PointerPhase::Leave | PointerPhase::Cancel => self.set_hovered(false),
            _ => {}
        }
        false
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(floom_widgets::SemanticsHint::decorative(), cx);
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
