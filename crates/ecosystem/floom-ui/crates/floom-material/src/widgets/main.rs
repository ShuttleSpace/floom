use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Axis, Stack, Surface};

use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialMainStyle {
    pub design_system: &'static str,
    /// The fill behind the content.
    pub background: Color,
    /// The registered top inset: the system bar plus the app bar.
    pub top: f32,
    /// The registered bottom inset: the footer plus the bottom navigation.
    pub bottom: f32,
    /// The open left navigation drawer's width.
    pub left: f32,
    /// The open right navigation drawer's width.
    pub right: f32,
}

/// Material's main content region.
pub struct MaterialMain {
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl MaterialMain {
    pub fn new() -> Self {
        Self {
            theme: theme_signal(),
            child: None,
        }
    }

    /// The scrollable/static content of the application.
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

    pub fn resolved_style(&self) -> MaterialMainStyle {
        let theme = self.theme.get();
        let layout = layout_signal().get();
        MaterialMainStyle {
            design_system: "material",
            background: theme.colors().background,
            top: layout.top(),
            bottom: layout.bottom(),
            left: layout.left(),
            right: layout.right(),
        }
    }

    /// Build the main content region with automatic layout insets.
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

impl Default for MaterialMain {
    fn default() -> Self {
        Self::new()
    }
}
