//! Fluent parallax, ported to the Floom 2.0 widget API.

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
pub use floom_ui_composites::{
    read_scroll, shared_scroll, write_scroll, ParallaxLayer, SharedScroll,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::parallax::ParallaxProps;
use floom_core_theme::theme::{MultiTheme, Theme};

/// so this kit carries one of its own. See the module note: this is the gap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentParallaxStyle {
    pub design_system: &'static str,
    pub factor: f32,
}

pub struct FluentParallax {
    props: ParallaxProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
    scroll: SharedScroll,
}

impl FluentParallax {
    pub fn new() -> Self {
        Self {
            props: ParallaxProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            scroll: shared_scroll(0.0),
        }
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

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

    pub fn resolved_style(&self) -> FluentParallaxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Parallax(tokens) = theme.component(ComponentId::Parallax) else {
            unreachable!("ComponentId::Parallax must resolve to ParallaxTokens")
        };
        let factor = if self.props.factor > 0.0 {
            self.props.factor
        } else {
            tokens.factor
        };
        FluentParallaxStyle {
            design_system: "fluent",
            factor,
        }
    }

    pub fn view(self) -> ParallaxLayer {
        let style = self.resolved_style();
        ParallaxLayer::new(self.child.clone(), self.scroll.clone(), style.factor)
    }
}

impl Default for FluentParallax {
    fn default() -> Self {
        Self::new()
    }
}
