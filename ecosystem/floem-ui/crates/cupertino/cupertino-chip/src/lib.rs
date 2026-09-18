use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::chip::ChipProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoChipStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub gap: f32,
}

pub struct CupertinoChip {
    props: ChipProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoChip {
    pub fn new() -> Self {
        let props = ChipProps::default();
        Self {
            props,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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
    pub fn resolved_style(&self) -> CupertinoChipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Chip(tokens) = theme.component(ComponentId::Chip) else {
            unreachable!("ComponentId::Chip must resolve to ChipTokens")
        };
        let (bg, fg) = chip_colors(&theme, self.props.selected, self.props.disabled);
        CupertinoChipStyle {
            design_system: DESIGN_SYSTEM,
            background: bg,
            foreground: fg,
            border_color: if self.props.selected {
                theme.colors().primary
            } else {
                theme.colors().outline
            },
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            gap: tokens.gap,
        }
    }

    /// Build a renderable Cupertino chip pill.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let on_click = Rc::new(self.props.on_click);
        let disabled = self.props.disabled;
        let selected = self.props.selected;
        let label = self.props.label.clone();

        let tokens = match theme.get().component(ComponentId::Chip) {
            ComponentTokens::Chip(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let gap = tokens.gap;

        Container::new(Label::new(label))
            .style(move |s| {
                let t = theme.get();
                let (bg, fg) = chip_colors(&t, selected, disabled);
                s.height(height)
                    .padding_horiz(padding_horizontal)
                    .border(1.0)
                    .border_color(
                        if selected {
                            t.colors().primary
                        } else {
                            t.colors().outline
                        }
                        .to_peniko(),
                    )
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .font_size(font_size)
                    .gap(gap)
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .apply_if(disabled, |s| s.set_disabled(true).cursor(CursorStyle::Default))
                    .hover(move |s| {
                        let t = theme.get();
                        let bg = if disabled {
                            with_alpha(t.colors().on_surface, 4)
                        } else if selected {
                            with_alpha(t.colors().primary, 200)
                        } else {
                            with_alpha(t.colors().on_surface, 16)
                        };
                        s.background(bg.to_peniko())
                    })
            })
            .on_event(listener::Click, move |_, _| {
                if !disabled {
                    if let Some(cb) = on_click.as_ref() {
                        cb();
                    }
                }
                EventPropagation::Continue
            })
    }
}

/// Cupertino chip fill/foreground colors per selected/disabled state.
fn chip_colors(theme: &MultiTheme, selected: bool, disabled: bool) -> (Color, Color) {
    if disabled {
        (
            with_alpha(theme.colors().on_surface, 4),
            with_alpha(theme.colors().on_surface, 97),
        )
    } else if selected {
        (theme.colors().primary, theme.colors().on_primary)
    } else {
        (
            theme.colors().surface_variant,
            theme.colors().on_surface,
        )
    }
}

impl Default for CupertinoChip {
    fn default() -> Self {
        Self::new()
    }
}
