use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::chip::ChipProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdChipStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub label_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub gap: f32,
}

pub struct MdChip {
    props: ChipProps,
    theme: RwSignal<MultiTheme>,
}

impl MdChip {
    pub fn new() -> Self {
        Self {
            props: ChipProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.props.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdChipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Chip(tokens) = theme.component(ComponentId::Chip) else {
            unreachable!("ComponentId::Chip must resolve to ChipTokens")
        };
        let (container_color, label_color) = if self.props.disabled {
            (
                with_alpha(theme.colors().on_surface, 12),
                with_alpha(theme.colors().on_surface, 38),
            )
        } else if self.props.selected {
            (theme.colors().secondary_container, theme.colors().on_secondary_container)
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface)
        };
        MdChipStyle {
            design_system: "material",
            container_color,
            label_color,
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            gap: tokens.gap,
        }
    }

    /// Build a renderable, interactive Material 3 chip (pill).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let label = self.props.label.clone();
        let on_click: Rc<Option<Box<dyn Fn()>>> = Rc::new(self.props.on_click);

        let tokens = match theme.get().component(ComponentId::Chip) {
            ComponentTokens::Chip(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let gap = tokens.gap;

        let selected = self.props.selected;

        Container::new(text(label))
            .style(move |s| {
                let t = theme.get();
                let bg = if disabled {
                    with_alpha(t.colors().on_surface, 12)
                } else if selected {
                    t.colors().secondary_container
                } else {
                    t.colors().surface_variant
                };
                let fg = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else if selected {
                    t.colors().on_secondary_container
                } else {
                    t.colors().on_surface
                };
                s.height(height)
                    .padding_horiz(padding_horizontal)
                    .gap(gap)
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .font_size(font_size)
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled)
            })
            .on_event(listener::Click, {
                let on_click = on_click.clone();
                move |_, _| {
                    if !disabled {
                        if let Some(cb) = on_click.as_ref() {
                            cb();
                        }
                    }
                    EventPropagation::Continue
                }
            })
    }
}

impl Default for MdChip {
    fn default() -> Self {
        Self::new()
    }
}
