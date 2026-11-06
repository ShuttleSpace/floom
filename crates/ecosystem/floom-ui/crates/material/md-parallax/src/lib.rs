use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
pub use floom_ui_composites::{
    read_scroll, shared_scroll, write_scroll, ParallaxLayer, SharedScroll,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::parallax::ParallaxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_runtime::theme_signal;

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

    pub fn scroll_signal(mut self, scroll: SharedScroll) -> Self {
        self.scroll = scroll;
        self
    }

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
