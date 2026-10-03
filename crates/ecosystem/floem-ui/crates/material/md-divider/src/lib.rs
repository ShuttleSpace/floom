use floem::prelude::*;
use floem::views::{Container, Decorators, Empty};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::divider::{DividerOrientation, DividerProps};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdDividerStyle {
    pub color: Color,
    pub thickness: f32,
    pub inset: f32,
    pub orientation: DividerOrientation,
}

pub struct MdDivider {
    props: DividerProps,
    theme: RwSignal<MultiTheme>,
}

impl MdDivider {
    pub fn new() -> Self {
        Self {
            props: DividerProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn orientation(mut self, orientation: DividerOrientation) -> Self {
        self.props.orientation = orientation;
        self
    }

    pub fn inset(mut self, inset: f32) -> Self {
        self.props.inset = inset;
        self
    }

    pub fn props(&self) -> &DividerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdDividerStyle {
        let theme = self.theme.get();
        let ComponentTokens::Divider(tokens) = theme.component(ComponentId::Divider) else {
            unreachable!("ComponentId::Divider must resolve to DividerTokens")
        };
        MdDividerStyle {
            color: theme.colors().outline_variant,
            thickness: tokens.thickness,
            inset: self.props.inset,
            orientation: self.props.orientation,
        }
    }

    /// Build a renderable Material 3 divider.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let orientation = self.props.orientation;
        let inset = self.props.inset;
        Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Divider(tokens) = t.component(ComponentId::Divider) else {
                unreachable!()
            };
            let color = t.colors().outline_variant.to_peniko();
            match orientation {
                DividerOrientation::Horizontal => s.height(tokens.thickness)
                    .width_full()
                    .background(color)
                    .margin_horiz(inset),
                DividerOrientation::Vertical => s.width(tokens.thickness)
                    .height_full()
                    .background(color)
                    .margin_vert(inset),
            }
        })
    }
}

impl Default for MdDivider {
    fn default() -> Self {
        Self::new()
    }
}
