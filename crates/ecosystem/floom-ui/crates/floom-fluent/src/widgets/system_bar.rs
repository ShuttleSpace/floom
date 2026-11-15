//! Fluent system bar, ported to the Floom 2.0 widget API.

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSystemBarStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub pad: f32,
    pub dark: bool,
}

pub struct FluentSystemBar {
    props: SystemBarProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl FluentSystemBar {
    pub fn new() -> Self {
        Self {
            props: SystemBarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    pub fn dark(mut self, dark: bool) -> Self {
        self.props.dark = dark;
        self
    }

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

    pub fn resolved_style(&self) -> FluentSystemBarStyle {
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
        FluentSystemBarStyle {
            design_system: "fluent",
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

    /// On every side, as v1 wrote it -- see the module note on the padding.
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

impl Default for FluentSystemBar {
    fn default() -> Self {
        Self::new()
    }
}
