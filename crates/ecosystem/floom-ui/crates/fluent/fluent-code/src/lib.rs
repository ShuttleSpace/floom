//! Fluent code, ported to the Floom 2.0 widget API.
//!
//! An inline code chip: v1 spelled it as a `Container` holding a `Label`; 2.0 spells it
//! a [`Surface`] holding a [`Text`], which is the same two things with the roles
//! named -- the surface is the an inline code chip, the text is the label.
//!
//! ## The monospace font is gone, and the props say so
//!
//! v1 drew the an inline code chip in a monospace font, and the props carried a `mono` flag
//! to ask for it. 2.0's text layer has no notion of a font family at all, so
//! there is nothing to ask for: the text is drawn in the default family. The
//! `mono` prop is carried and reports what the caller asked for, because a
//! caller reading it back should not be told the font changed when it did not.

use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, CodeTokens};
use floom_core_theme::props::code::{CodeProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// A builder for the an inline code chip's content, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentCodeStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub padding_horizontal: f32,
    pub padding_vertical: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub mono: bool,
}

/// Fluent's code.
pub struct FluentCode {
    props: CodeProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
    /// The convenience text, for a kit with no child of its own.
    ///
    /// v1 kept this beside the props rather than in them: `CodeProps`
    /// and `KbdProps` carry only `mono`.
    content: Option<String>,
}

impl FluentCode {
    pub fn new() -> Self {
        Self {
            props: CodeProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
            content: None,
        }
    }

    /// Convenience text content, for a an inline code chip with no child of its own.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Ask for a monospace font.
    ///
    /// Carried, and reported, and not honoured: 2.0's text layer has no font
    /// family. See the module note.
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

    pub fn resolved_style(&self) -> FluentCodeStyle {
        let theme = self.theme.get();
        let ComponentTokens::Code(tokens) = theme.component(ComponentId::Code) else {
            unreachable!("ComponentId::Code must resolve to CodeTokens")
        };
        FluentCodeStyle {
            design_system: "fluent",
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
                    // The slot is a replayed `Fn`, so the text is cloned per mount.
                    Box::new(Text::new(content.clone()).size(font_size).color(text_color))
                });
            }
        }
        surface
    }
}

impl Default for FluentCode {
    fn default() -> Self {
        Self::new()
    }
}

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
