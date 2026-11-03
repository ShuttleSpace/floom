//! Fluent chip group, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::Axis;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ContentPlacement, SelectGroup, SelectGroupStyle, SelectionMode};
use floom_widgets::Text;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip_group::ChipGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

type Callback = Arc<dyn Fn(Vec<usize>) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentChipGroupStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub selected_background: Color,
    pub selected_foreground: Color,
    pub gap: f32,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
}

pub struct FluentChipGroup {
    props: ChipGroupProps,
    labels: Vec<String>,
    theme: RwSignal<MultiTheme>,
    on_change: Option<Callback>,
}

impl FluentChipGroup {
    pub fn new() -> Self {
        Self {
            props: ChipGroupProps::default(),
            labels: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_change: None,
        }
    }

    pub fn chip(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(Vec<usize>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// that as an `Arc` of its own (see the field's note), and `ChipGroupProps`
    pub fn props(&self) -> &ChipGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentChipGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::ChipGroup(tokens) = theme.component(ComponentId::ChipGroup) else {
            unreachable!("ComponentId::ChipGroup must resolve to ChipGroupTokens")
        };
        FluentChipGroupStyle {
            design_system: "fluent",
            background: theme.colors().surface_variant,
            foreground: theme.colors().on_surface,
            selected_background: theme.colors().secondary_container,
            selected_foreground: theme.colors().on_secondary_container,
            gap: tokens.gap,
            height: 32.0,
            padding_horizontal: 12.0,
            border_radius: 16.0,
            font_size: 13.0,
        }
    }

    pub fn select_group_style(&self) -> SelectGroupStyle {
        let style = self.resolved_style();
        SelectGroupStyle {
            gap: style.gap,
            axis: Axis::Horizontal,
            height: Some(style.height),
            pad: style.padding_horizontal,
            content_placement: ContentPlacement::Centred,
            toggles_off_in_single_mode: true,
            mandatory_in_multiple_mode: false,
            radius: style.border_radius,
            border_width: 0.0,
            background: to_paint(style.background),
            foreground: to_paint(style.foreground),
            border_color: to_paint(style.foreground),
            selected_background: to_paint(style.selected_background),
            selected_foreground: to_paint(style.selected_foreground),
            selected_border_color: to_paint(style.selected_foreground),
        }
    }

    pub fn view(self) -> SelectGroup {
        let style = self.resolved_style();
        let mut group = SelectGroup::new()
            .mode(if self.props.multiple {
                SelectionMode::Multiple
            } else {
                SelectionMode::Single
            })
            .mandatory(self.props.mandatory)
            .style(self.select_group_style());

        let ink = to_paint(style.foreground);
        let selected_ink = to_paint(style.selected_foreground);
        let font_size = style.font_size;
        for label in self.labels {
            let text = label.clone();
            group = group.row_with_state(label, move |selected| {
                let color = if selected { selected_ink } else { ink };
                Box::new(Text::new(text.clone()).size(font_size).color(color))
            });
        }

        if let Some(on_change) = self.on_change.clone() {
            group = group.on_change(move |indices| on_change(indices));
        }

        group
    }
}

impl Default for FluentChipGroup {
    fn default() -> Self {
        Self::new()
    }
}
