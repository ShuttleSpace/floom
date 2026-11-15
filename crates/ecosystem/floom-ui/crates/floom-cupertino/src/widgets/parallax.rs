//! Cupertino parallax, ported to the Floom 2.0 widget API.
//!

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
pub use floom_ui_composites::{
    read_scroll, shared_scroll, write_scroll, ParallaxLayer, SharedScroll,
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
