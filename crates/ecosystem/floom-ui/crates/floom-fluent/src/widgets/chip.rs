//! Fluent chip, ported to the Floom 2.0 widget API.

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_ui_composites::Chip;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip::ChipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

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
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
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
        let (background, text_color) = if self.props.disabled {
            (
                with_alpha(theme.colors().on_surface, 12),
                with_alpha(theme.colors().on_surface, 38),
            )
        } else if self.props.selected {
            (
                theme.colors().primary_container,
                theme.colors().on_primary_container,
            )
        } else {
            (
                theme.colors().surface_variant,
                theme.colors().on_surface_variant,
            )
        };
        FluentChipStyle {
            design_system: DESIGN_SYSTEM,
            background,
            text_color,
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

    pub fn view(self) -> Chip {
        let style = self.resolved_style();
        let selected = self.props.selected;
        let mut chip = Chip::new()
            .label(self.props.label.clone())
            .background(to_paint(style.background))
            .foreground(to_paint(style.text_color))
            .height(style.height)
            .padding_horizontal(style.padding_horizontal)
            .radius(style.border_radius)
            .font_size(style.font_size)
            .font_weight(FontWeight::Medium)
            .border(
                if selected { 1.0 } else { 0.0 },
                to_paint(style.border_color),
            )
            .disabled(self.props.disabled);
        if let Some(on_click) = self.props.on_click {
            chip = chip.on_click(move || on_click());
        }
        chip
    }
}

impl Default for FluentChip {
    fn default() -> Self {
        Self::new()
    }
}
