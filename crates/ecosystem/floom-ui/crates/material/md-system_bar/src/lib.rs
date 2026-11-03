
use floom_core::Child;
use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;
use floom_widgets::{Align, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::system_bar::SystemBarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSystemBarStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub surface: Color,
    /// The colour the bar's content should be drawn in.
    pub content: Color,
    /// The bar's height: the props', or the token's when the props defer.
    pub height: f32,
    /// The bar's padding: the props', or the token's when the props defer.
    pub pad: f32,
    /// Whether the dark variant is in force.
    pub dark: bool,
}

/// Material's system bar: the thin status strip at the top of the shell.
pub struct MaterialSystemBar {
    props: SystemBarProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl MaterialSystemBar {
    pub fn new() -> Self {
        Self {
            props: SystemBarProps::default(),
            theme: theme_signal(),
            child: None,
        }
    }

    /// The bar's height in dp. `None` -- the default -- defers to the design
    /// system's `SystemBarTokens::height` (M3 24 / Cupertino 22 / Fluent 28).
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// Dark variant: uses a darker, higher-contrast background.
    pub fn dark(mut self, dark: bool) -> Self {
        self.props.dark = dark;
        self
    }

    /// The bar's padding. `None` -- the default -- defers to the design
    /// system's `SystemBarTokens::pad` (M3 8 / Cupertino 6 / Fluent 8).
    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = Some(pad);
        self
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn props(&self) -> &SystemBarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialSystemBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::SystemBar(tokens) = theme.component(ComponentId::SystemBar) else {
            unreachable!("ComponentId::SystemBar must resolve to SystemBarTokens")
        };
        let (surface, content) = if self.props.dark {
            (theme.colors().scrim, theme.colors().on_surface)
        } else {
            (
                theme.colors().surface_variant,
                theme.colors().on_surface_variant,
            )
        };
        MaterialSystemBarStyle {
            design_system: "material",
            surface,
            content,
            height: self.props.height.unwrap_or(tokens.height),
            pad: self.props.pad.unwrap_or(tokens.pad),
            dark: self.props.dark,
        }
    }

    pub fn request(&self) -> LayoutRequest {
        let style = self.resolved_style();
        LayoutRequest::new()
            .width_pct(100.0)
            .height(style.height)
            .cross_align(Align::Center)
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().pad))
    }

    pub fn content_color(&self) -> PaintColor {
        to_paint(self.resolved_style().content)
    }

    pub fn publish_layout(&self) {
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::SystemBar,
            self.resolved_style().height,
        );
    }

    /// The bar: the filled box, its padding, its extent, and the caller's
    /// content inside it.
    pub fn view(self) -> Surface {
        self.publish_layout();

        let request = self.request();
        let padding = self.padding();
        let fill = to_paint(self.resolved_style().surface);
        let child = self.child;

        let bar = Surface::new()
            .request(request)
            .padding(padding)
            .background(fill);
        match child {
            Some(child) => bar.child(child),
            None => bar,
        }
    }
}

impl Default for MaterialSystemBar {
    fn default() -> Self {
        Self::new()
    }
}
