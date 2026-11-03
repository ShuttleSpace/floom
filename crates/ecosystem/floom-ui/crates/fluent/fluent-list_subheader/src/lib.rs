//! Fluent list subheader, ported to the Floom 2.0 widget API.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ListSubheaderTokens};
use floom_core_theme::props::list_subheader::ListSubheaderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentListSubheaderStyle {
    pub color: Color,
    pub font_size: f32,
    pub height: f32,
    pub padding: f32,
    pub inset: bool,
    pub letter_spacing: f32,
}

pub struct FluentListSubheader {
    props: ListSubheaderProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentListSubheader {
    pub fn new(text_content: impl Into<String>) -> Self {
        Self {
            props: ListSubheaderProps {
                text: text_content.into(),
                inset: false,
            },
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn inset(mut self, inset: bool) -> Self {
        self.props.inset = inset;
        self
    }

    pub fn props(&self) -> &ListSubheaderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentListSubheaderStyle {
        let theme = self.theme.get();
        let ComponentTokens::ListSubheader(tokens) = theme.component(ComponentId::ListSubheader)
        else {
            unreachable!("ComponentId::ListSubheader must resolve to ListSubheaderTokens")
        };
        FluentListSubheaderStyle {
            color: theme.colors().on_surface_variant,
            font_size: tokens.font_size,
            height: tokens.height,
            padding: if self.props.inset {
                tokens.inset_padding
            } else {
                tokens.padding_horizontal
            },
            inset: self.props.inset,
            letter_spacing: tokens.letter_spacing,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let text = self.props.text.clone();
        let color = to_paint(style.color);
        let font_size = style.font_size;
        Surface::new()
            .request(
                LayoutRequest::new()
                    .height(style.height)
                    .padding_horiz(f64::from(style.padding))
                    .cross_align(Align::Center),
            )
            .push(move || {
                let text = text.clone();
                Box::new(Text::new(text).size(font_size).color(color))
            })
    }
}

impl Default for FluentListSubheader {
    fn default() -> Self {
        Self::new(String::new())
    }
}
