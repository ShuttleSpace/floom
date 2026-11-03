
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::Chip;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip::ChipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialChipStyle {
    pub design_system: &'static str,
    /// The pill's fill.
    pub container_color: Color,
    /// The label's ink.
    pub label_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub gap: f32,
}

/// Material's chip.
pub struct MaterialChip {
    props: ChipProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialChip {
    pub fn new() -> Self {
        Self {
            props: ChipProps::default(),
            theme: theme_signal(),
        }
    }

    /// The chip's text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    /// The chip in its chosen appearance.
    pub fn selected(mut self, selected: bool) -> Self {
        self.props.selected = selected;
        self
    }

    /// The chip greyed out, and no longer answering the pointer.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called when the chip is clicked. A disabled chip does not call it.
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

    pub fn resolved_style(&self) -> MaterialChipStyle {
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
            (
                theme.colors().secondary_container,
                theme.colors().on_secondary_container,
            )
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface)
        };
        MaterialChipStyle {
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

    /// The chip: the pill, its centred label, and the click.
    pub fn view(self) -> Chip {
        let style = self.resolved_style();
        let mut chip = Chip::new()
            .label(self.props.label.clone())
            .background(to_paint(style.container_color))
            .foreground(to_paint(style.label_color))
            .height(style.height)
            .padding_horizontal(style.padding_horizontal)
            .radius(style.border_radius)
            .font_size(style.font_size)
            .border(0.0, to_paint(style.container_color))
            .disabled(self.props.disabled);
        if let Some(on_click) = self.props.on_click {
            chip = chip.on_click(move || on_click());
        }
        chip
    }
}

impl Default for MaterialChip {
    fn default() -> Self {
        Self::new()
    }
}
