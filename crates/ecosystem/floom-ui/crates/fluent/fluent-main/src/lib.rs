//! Fluent main content region, ported to the Floom 2.0 widget API.

use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Axis, Stack, Surface};

use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentMainStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

pub struct FluentMain {
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl FluentMain {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentMainStyle {
        let theme = self.theme.get();
        // The insets are read here, once: see the module's note on the layout
        let layout = layout_signal().get();
        FluentMainStyle {
            design_system: "fluent",
            background: theme.colors().background,
            top: layout.top(),
            bottom: layout.bottom(),
            left: layout.left(),
            right: layout.right(),
        }
    }

    pub fn view(self) -> Stack {
        let style = self.resolved_style();

        let top = Surface::new().request(LayoutRequest::new().width_pct(100.0).height(style.top));
        let bottom =
            Surface::new().request(LayoutRequest::new().width_pct(100.0).height(style.bottom));
        let left = Surface::new().request(LayoutRequest::new().height_pct(100.0).width(style.left));
        let right =
            Surface::new().request(LayoutRequest::new().height_pct(100.0).width(style.right));

        let mut content = Surface::new().background(to_paint(style.background));
        if let Some(child) = self.child {
            content = content.child(child);
        }

        let middle = Stack::new(Axis::Horizontal)
            .request(LayoutRequest::new().width_pct(100.0))
            .push(move || Box::new(left.clone()))
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(content.clone())
            })
            .push(move || Box::new(right.clone()));

        Stack::new(Axis::Vertical)
            .request(LayoutRequest::new().width_pct(100.0).height_pct(100.0))
            .push(move || Box::new(top.clone()))
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(middle.clone())
            })
            .push(move || Box::new(bottom.clone()))
    }
}

impl Default for FluentMain {
    fn default() -> Self {
        Self::new()
    }
}
