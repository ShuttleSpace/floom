use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::chip::ChipProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentChipStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub gap: f32,
}

pub struct FluentChip {
    props: ChipProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentChip {
    pub fn new() -> Self {
        Self {
            props: ChipProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    pub fn props(&self) -> &ChipProps {
        &self.props
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentChipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Chip(tokens) = theme.component(ComponentId::Chip) else {
            unreachable!("ComponentId::Chip must resolve to ChipTokens")
        };
        let (bg, fg) = if self.props.disabled {
            (
                with_alpha(theme.colors().on_surface, 12),
                with_alpha(theme.colors().on_surface, 38),
            )
        } else if self.props.selected {
            (theme.colors().primary_container, theme.colors().on_primary_container)
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface_variant)
        };
        FluentChipStyle {
            design_system: DESIGN_SYSTEM,
            background: bg,
            text_color: fg,
            border_color: if self.props.selected {
                theme.colors().primary
            } else {
                theme.colors().outline_variant
            },
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            gap: tokens.gap,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let label = self.props.label.clone();
        let selected = self.props.selected;
        let disabled = self.props.disabled;
        let on_click = Rc::new(self.props.on_click);

        let tokens = match theme.get().component(ComponentId::Chip) {
            ComponentTokens::Chip(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let gap = tokens.gap;

        let chip = Container::new(text(label))
            .style(move |s| {
                let t = theme.get();
                let (bg, fg) = if disabled {
                    (
                        with_alpha(t.colors().on_surface, 12),
                        with_alpha(t.colors().on_surface, 38),
                    )
                } else if selected {
                    (t.colors().primary_container, t.colors().on_primary_container)
                } else {
                    (t.colors().surface_variant, t.colors().on_surface_variant)
                };
                s.height(height)
                    .padding_horiz(padding_horizontal)
                    .border_radius(radius)
                    .gap(gap)
                    .items_center()
                    .justify_center()
                    .background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .font_size(font_size)
                    .font_weight(FontWeight::Medium.to_floem())
                    .apply_if(selected, |s| {
                        s.border(1.0).border_color(t.colors().primary.to_peniko())
                    })
                    .cursor(if disabled {
                        CursorStyle::Default
                    } else {
                        CursorStyle::Pointer
                    })
                    .apply_if(disabled, |s| s.set_disabled(true))
            })
            .on_event(listener::Click, move |_, _| {
                if !disabled {
                    if let Some(cb) = on_click.as_ref() {
                        cb();
                    }
                }
                EventPropagation::Continue
            });

        chip
    }
}

impl Default for FluentChip {
    fn default() -> Self {
        Self::new()
    }
}
