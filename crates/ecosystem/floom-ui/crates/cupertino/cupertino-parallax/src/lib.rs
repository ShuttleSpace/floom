//! Cupertino parallax, ported to the Floom 2.0 widget API.
//!
//! A parallax is a layer that moves more slowly than the page it sits on: the
//! content is translated by `-scroll * factor` as the page scrolls, so a
//! smaller factor makes it appear to move slower than the foreground.
//!
//! ## What the port uses
//!
//! The engine's [`ParallaxLayer`] and the engine's compositor channel. The
//! effect is a translate, and the engine's only translate is
//! `CompositorProp::TranslateY` -- a property a widget declares for itself
//! and the host folds onto its whole subtree. There is no translate in
//! `LayoutRequest`, so the layer declares that channel instead.
//!
//! ## What this kit answers
//!
//! * the factor: the caller's, when it is above zero; the token's otherwise.
//!   Cupertino's token is 0.3.
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
//! carries the offset in a shared value. **That was the gap**: the engine's
//! shared family (`SharedFlag`, `SharedBuffer`, `SharedMap`) had no number.
//! The engine has one now -- `SharedScroll`, a shared `f32` beside the other
//! three -- and the layer that reads it is the engine's too:
//! `floom_ui_composites::ParallaxLayer` is content that moves at a fraction
//! of the page's rate.
//!
//! What is still the engine's to answer is the other half: a `Scroll` that
//! can hand its offset to a descendant, so that a layer inside one does not
//! need a host to write the offset for it.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The accessors.** v1's parallax had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.
//! * **The scroll accessor.** v1 only had the setter. The port adds
//!   [`CupertinoParallax::scroll_offset`], because without a way to *read* the
//!   shared offset the host has nothing to write to -- the setter alone is
//!   half a contract.
//! * **The signal's type.** v1's was an `RwSignal<f32>`, which is `Rc`-based
//!   and cannot cross a `Send + Sync` slot boundary. The port's is the shared
//!   value described above.


use floom_core::{Child};
use floom_signal::{RwSignal, SignalGet};
pub use floom_ui_composites::{
    ParallaxLayer, SharedScroll, read_scroll, shared_scroll, write_scroll,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::parallax::ParallaxProps;
use floom_core_theme::theme::{MultiTheme, Theme};

/// The scroll offset, shared between the parallax and whoever drives it.
///
/// The engine's shared family has a flag, a buffer and a map, and no number,
/// so this kit carries one of its own. See the module note: this is the gap.
/// The parallax's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoParallaxStyle {
    pub design_system: &'static str,
    /// The content is translated by `-scroll * factor`.
    pub factor: f32,
}

/// Cupertino's parallax.
pub struct CupertinoParallax {
    props: ParallaxProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
    /// The host-provided scroll offset.
    scroll: SharedScroll,
}

impl CupertinoParallax {
    pub fn new() -> Self {
        Self {
            props: ParallaxProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoParallaxStyle {
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
        CupertinoParallaxStyle {
            design_system: "cupertino",
            factor,
        }
    }

    /// The parallax layer, ready to mount.
    pub fn view(self) -> ParallaxLayer {
        let style = self.resolved_style();
        ParallaxLayer::new(self.child.clone(), self.scroll.clone(), style.factor)
    }
}

impl Default for CupertinoParallax {
    fn default() -> Self {
        Self::new()
    }
}
