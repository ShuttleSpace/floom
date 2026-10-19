//! Material list subheader, ported to the Floom 2.0 widget API.
//!
//! A subheader is a row of small, muted text above a group of list items. v1
//! spelled it as a `Container` holding a `Label`; 2.0 spells it a [`Surface`]
//! holding a [`Text`], which is the same two things with the roles named: the
//! surface is the row, the text is the label, and the colour belongs to the
//! text rather than to the box.
//!
//! ## The two paddings
//!
//! A subheader is padded one way when it lines up with the list's own content
//! and another when it is *inset* from it -- v1's `inset` flag chose between
//! `padding_horizontal` and `inset_padding`, and the port keeps both.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_core::LayoutRequest;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ListSubheaderTokens};
use floom_core_theme::props::list_subheader::ListSubheaderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialListSubheaderStyle {
    pub color: Color,
    pub font_size: f32,
    pub height: f32,
    pub padding: f32,
    pub inset: bool,
    pub letter_spacing: f32,
}

/// Material's list subheader.
pub struct MaterialListSubheader {
    props: ListSubheaderProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialListSubheader {
    pub fn new(text_content: impl Into<String>) -> Self {
        Self {
            props: ListSubheaderProps {
                text: text_content.into(),
                inset: false,
            },
            theme: theme_signal(),
        }
    }

    /// Pad the subheader from the list's own edge rather than its content's.
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

    pub fn resolved_style(&self) -> MaterialListSubheaderStyle {
        let theme = self.theme.get();
        let ComponentTokens::ListSubheader(tokens) = theme.component(ComponentId::ListSubheader)
        else {
            unreachable!("ComponentId::ListSubheader must resolve to ListSubheaderTokens")
        };
        MaterialListSubheaderStyle {
            color: theme.colors().on_surface_variant,
            font_size: tokens.font_size,
            height: tokens.height,
            // v1: `if inset { tk.inset_padding } else { tk.padding_horizontal }`.
            padding: if self.props.inset {
                tokens.inset_padding
            } else {
                tokens.padding_horizontal
            },
            inset: self.props.inset,
            letter_spacing: tokens.letter_spacing,
        }
    }

    /// The row: a padded, full-width box with the subheader's text centred in
    /// it.
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
                    // v1's `items_center()`: the text is centred on the cross axis.
                    .cross_align(Align::Center),
            )
            .push(move || {
                // The slot is a replayed `Fn`, so the text is cloned per mount.
                let text = text.clone();
                Box::new(Text::new(text).size(font_size).color(color))
            })
    }
}

impl Default for MaterialListSubheader {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
