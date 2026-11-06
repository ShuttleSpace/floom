use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::component_tokens::{CodeTokens, ComponentId, ComponentTokens};
use floom_core_theme::props::code::CodeProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// A builder for the an inline code chip's content, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialCodeStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub padding_horizontal: f32,
    pub padding_vertical: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub mono: bool,
}

/// Material's code.
pub struct MaterialCode {
    props: CodeProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
    content: Option<String>,
}

impl MaterialCode {
    pub fn new() -> Self {
        Self {
            props: CodeProps::default(),
            theme: theme_signal(),
            child: None,
            content: None,
        }
    }

    /// Convenience text content, for a an inline code chip with no child of its own.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.props.mono = mono;
        self
    }

    /// Supply the an inline code chip's content instead of its text.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &CodeProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialCodeStyle {
        let theme = self.theme.get();
        let ComponentTokens::Code(tokens) = theme.component(ComponentId::Code) else {
            unreachable!("ComponentId::Code must resolve to CodeTokens")
        };
        MaterialCodeStyle {
            design_system: "material",
            background: theme.colors().surface_variant,
            text_color: theme.colors().on_surface,
            padding_horizontal: tokens.padding_horizontal,
            padding_vertical: tokens.padding_vertical,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            mono: self.props.mono,
        }
    }

    /// The an inline code chip, ready to mount.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let content = self.content.clone().unwrap_or_default();
        let child = self.child.clone();
        let font_size = style.font_size;
        let text_color = to_paint(style.text_color);

        let mut surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .padding_horiz(f64::from(style.padding_horizontal))
                    .padding_vert(f64::from(style.padding_vertical))
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(to_paint(style.background))
            .radius(style.border_radius);
        match child {
            Some(build) => surface = surface.push(move || build()),
            None => {
                surface = surface.push(move || {
                    Box::new(Text::new(content.clone()).size(font_size).color(text_color))
                });
            }
        }
        surface
    }
}

impl Default for MaterialCode {
    fn default() -> Self {
        Self::new()
    }
}
