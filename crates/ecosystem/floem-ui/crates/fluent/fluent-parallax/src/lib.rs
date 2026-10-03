use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::parallax::ParallaxProps;
use floem_core_theme::theme::{MultiTheme, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentParallaxStyle {
    pub design_system: &'static str,
    pub factor: f32,
}

pub struct FluentParallax {
    props: ParallaxProps,
    child: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
    scroll: RwSignal<f32>,
}

impl FluentParallax {
    pub fn new() -> Self {
        Self {
            props: ParallaxProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            scroll: RwSignal::new(0.0),
        }
    }

    /// Set the parallax content (a background layer, image, etc.).
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Parallax strength: the content is translated by `-scroll * factor`, so a
    /// smaller factor makes it appear to move slower than the foreground.
    pub fn factor(mut self, factor: f32) -> Self {
        self.props.factor = factor;
        self
    }

    /// Host-provided scroll offset (e.g. a page/scroller's current scroll Y). The
    /// host owns the scroll source; `Parallax` stays decoupled from any specific
    /// ancestor scroller (mirrors how `Radio` leaves mutual exclusion to the host).
    pub fn scroll_signal(mut self, scroll: RwSignal<f32>) -> Self {
        self.scroll = scroll;
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

    pub fn view(self) -> impl View {
        let scroll = self.scroll;
        let factor = self.resolved_style().factor;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        Container::new(child).style(move |s| s.translate_y(-scroll.get() * factor))
    }
}

impl Default for FluentParallax {
    fn default() -> Self {
        Self::new()
    }
}
