//! Material parallax, ported to the Floom 2.0 widget API.
//!
//! A parallax is a layer that moves more slowly than the page it sits on: the
//! content is translated by `-scroll * factor` as the page scrolls, so a
//! smaller factor makes it appear to move slower than the foreground.
//!
//! ## What the port uses
//!
//! A kit-local widget, [`ParallaxLayer`], and the engine's compositor
//! channel. The effect is a translate, and the engine's only translate is
//! `CompositorProp::TranslateY` -- a property a widget declares for itself
//! and the host folds onto its whole subtree. There is no translate in
//! `LayoutRequest`, and no widget in the engine that translates a subtree, so
//! the layer is the kit's.
//!
//! ## What this kit answers
//!
//! * the factor: the caller's, when it is above zero; the token's otherwise.
//!   Material's token is 0.3.
//!
//! ## The token that never showed
//!
//! `ParallaxProps::default()` already carries a factor of 0.3, which is above
//! zero, so the rule "the caller's factor, or the token's when the caller's is
//! not positive" never reaches the token -- the props' default is positive.
//! The token's number is kept in the style's rule and named here rather than
//! quietly relied on.
//!
//! ## The engine gap: nothing can read a scroll offset
//!
//! v1's parallax was driven by a host-provided scroll offset:
//! `scroll_signal(RwSignal<f32>)` took the page's current scroll Y, and the
//! content was translated by `-scroll * factor`. The host owned the scroll
//! source on purpose -- v1's own comment says the parallax "stays decoupled
//! from any specific ancestor scroller (mirrors how `Radio` leaves mutual
//! exclusion to the host)".
//!
//! 2.0's engine has a `Scroll` widget, and it was read for this port. It
//! carries the *offset* but it cannot carry the *effect*:
//!
//! * the offset lives in a private `Arc<Mutex<ScrollOffset>>` inside the
//!   `Scroll` value, and a descendant cannot reach an ancestor's `Scroll` --
//!   there is no context lookup that walks up the tree. A layer inside a
//!   scroller cannot ask the scroller how far it has scrolled;
//! * `Scroll` applies **one** offset to its whole subtree, so a layer inside
//!   it moves at the page's rate. A parallax needs a *fraction* of that rate,
//!   and the engine has no per-child factor;
//! * `LayoutRequest` has no translate at all.
//!
//! So the kit keeps v1's contract -- the host owns the scroll source -- and
//! carries the offset in a shared value of its own. **That is the gap**: the
//! engine's shared family (`SharedFlag`, `SharedBuffer`, `SharedMap`) has no
//! number, so the kit defines [`SharedScroll`] locally, and the engine piece
//! that would own it -- a `SharedOffset` beside the other three, or a `Scroll`
//! that can hand its offset to a descendant -- is the piece that is missing.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The accessors.** v1's parallax had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.
//! * **The scroll accessor.** v1 only had the setter. The port adds
//!   [`MaterialParallax::scroll_offset`], because without a way to *read* the
//!   shared offset the host has nothing to write to -- the setter alone is
//!   half a contract.
//! * **The signal's type.** v1's was an `RwSignal<f32>`, which is `Rc`-based
//!   and cannot cross a `Send + Sync` slot boundary. The port's is the shared
//!   value described above.

use std::any::Any;
use std::sync::{Arc, Mutex};

use floom_core::{
    Child, CreateContext, ElementId, PaintContext, UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{AnimationSpec, CompositorProp};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::parallax::ParallaxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_runtime::theme_signal;

/// The scroll offset, shared between the parallax and whoever drives it.
///
/// The engine's shared family has a flag, a buffer and a map, and no number,
/// so this kit carries one of its own. See the module note: this is the gap.
pub type SharedScroll = Arc<Mutex<f32>>;

/// A fresh shared scroll offset.
pub fn shared_scroll(value: f32) -> SharedScroll {
    Arc::new(Mutex::new(value))
}

/// Read the shared offset, treating a poisoned mutex as "no scroll".
///
/// The same posture as the engine's `read_flag`: a caller that poisoned the
/// mutex is as stuck as the widget is, and panicking here would take down a
/// paint pass over a number nobody was looking at.
pub fn read_scroll(scroll: &SharedScroll) -> f32 {
    scroll.lock().map(|guard| *guard).unwrap_or(0.0)
}

/// Write the shared offset. Reports whether it changed, so a caller can skip a
/// frame.
pub fn write_scroll(scroll: &SharedScroll, value: f32) -> bool {
    match scroll.lock() {
        Ok(mut guard) if *guard != value => {
            *guard = value;
            true
        }
        _ => false,
    }
}

/// The parallax's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialParallaxStyle {
    pub design_system: &'static str,
    /// The content is translated by `-scroll * factor`.
    pub factor: f32,
}

/// Material's parallax.
pub struct MaterialParallax {
    props: ParallaxProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
    /// The host-provided scroll offset.
    scroll: SharedScroll,
}

impl MaterialParallax {
    pub fn new() -> Self {
        Self {
            props: ParallaxProps::default(),
            child: None,
            theme: theme_signal(),
            scroll: shared_scroll(0.0),
        }
    }

    /// Set the parallax content (a background layer, image, etc.).
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Parallax strength: the content is translated by `-scroll * factor`, so a
    /// smaller factor makes it appear to move slower than the foreground.
    pub fn factor(mut self, factor: f32) -> Self {
        self.props.factor = factor;
        self
    }

    /// Host-provided scroll offset (e.g. a page/scroller's current scroll Y).
    /// The host owns the scroll source; the parallax stays decoupled from any
    /// specific ancestor scroller (mirrors how `Radio` leaves mutual exclusion
    /// to the host).
    pub fn scroll_signal(mut self, scroll: SharedScroll) -> Self {
        self.scroll = scroll;
        self
    }

    /// The shared scroll offset, so the host can drive it.
    ///
    /// v1 had only the setter; without a way to read the value back the host
    /// had nothing to write to.
    pub fn scroll_offset(&self) -> SharedScroll {
        self.scroll.clone()
    }

    pub fn props(&self) -> &ParallaxProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialParallaxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Parallax(tokens) = theme.component(ComponentId::Parallax) else {
            unreachable!("ComponentId::Parallax must resolve to ParallaxTokens")
        };
        // v1: the caller's factor when it is above zero, the token's otherwise.
        let factor = if self.props.factor > 0.0 {
            self.props.factor
        } else {
            tokens.factor
        };
        MaterialParallaxStyle {
            design_system: "material",
            factor,
        }
    }

    /// The parallax layer, ready to mount.
    pub fn view(self) -> ParallaxLayer {
        let style = self.resolved_style();
        ParallaxLayer::new(self.child.clone(), self.scroll.clone(), style.factor)
    }
}

impl Default for MaterialParallax {
    fn default() -> Self {
        Self::new()
    }
}

/// The parallax layer: the content, moved by `-scroll * factor`.
///
/// The kit's own widget, because the effect needs a translate and the engine
/// has none in its layout vocabulary. It is a layer and nothing else: it
/// declares no size of its own -- v1's `Container` had none either -- so the
/// caller's slot decides how big the layer is.
pub struct ParallaxLayer {
    /// The content being moved.
    child: Option<Child>,
    /// The host-provided scroll offset.
    scroll: SharedScroll,
    /// How much of the offset the content is moved by.
    factor: f32,
    /// The offset this widget last read.
    last_offset: f32,
    /// Bumped when the offset moves.
    ///
    /// The host reconciles a widget's declaration against the one it is
    /// already running, and an unchanged declaration is left alone. The offset
   /// is part of the declaration, so a moved offset is a new declaration --
    /// and the key is what makes the host see it as one.
    rev: u64,
}

impl ParallaxLayer {
    pub fn new(child: Option<Child>, scroll: SharedScroll, factor: f32) -> Self {
        Self {
            child,
            scroll,
            factor,
            last_offset: 0.0,
            rev: 0,
        }
    }

    /// The offset this layer last read.
    pub fn offset(&self) -> f32 {
        self.last_offset
    }
}

impl Widget for ParallaxLayer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Read the offset, and ask for a repaint when it moved.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let offset = read_scroll(&self.scroll);
        if offset != self.last_offset {
            self.last_offset = offset;
            self.rev = self.rev.wrapping_add(1);
            cx.mark_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) => vec![child.clone()],
            None => Vec::new(),
        }
    }

    /// v1's `translate_y(-scroll * factor)`.
    ///
    /// The engine's translate is a compositor channel, so the layer declares
    /// one: a zero-length tween whose endpoints are the offset it wants,
    /// which the host parks on the node and folds onto the whole subtree.
    fn animation(&self) -> Option<AnimationSpec> {
        let ty = -f64::from(self.last_offset) * f64::from(self.factor);
        Some(
            AnimationSpec::new(ty, ty, 0.0)
                .with_prop(CompositorProp::TranslateY(0.0))
                .with_key(self.rev),
        )
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}
}

impl WidgetAny for ParallaxLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        // A layer that moves: it publishes nothing of its own, and the content
        // inside it publishes whatever it is.
        None
    }
}
