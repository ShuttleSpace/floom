
use std::sync::Arc;

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ContentPlacement, SelectGroup, SelectGroupStyle, SelectionMode};
use floom_widgets::{Align, Axis, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::radio_group::RadioGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

const RING_BORDER: f32 = 2.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialRadioGroupStyle {
    pub design_system: &'static str,
    /// The ring's outline when the option is chosen.
    pub selected_color: Color,
    /// The ring's outline when the option is not chosen.
    pub unselected_color: Color,
    /// The ring's corner radius.
    pub radius: f32,
    pub radio_size: f32,
    pub item_height: f32,
    pub gap: f32,
}

/// Material's radio group.
pub struct MaterialRadioGroup {
    props: RadioGroupProps,
    theme: RwSignal<MultiTheme>,
    on_change: Option<Callback>,
}

impl MaterialRadioGroup {
    pub fn new() -> Self {
        Self {
            props: RadioGroupProps::default(),
            theme: theme_signal(),
            on_change: None,
        }
    }

    /// Replace the options.
    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }

    /// Append an option.
    pub fn option(mut self, option: impl Into<String>) -> Self {
        self.props.options.push(option.into());
        self
    }

    /// The option chosen before mount.
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = Some(selected);
        self
    }

    /// Whether every option is turned off.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the chosen option's index whenever the choice changes.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        let shared: Callback = Arc::new(callback);
        self.props.on_change = Some(Box::new({
            let shared = shared.clone();
            move |index| shared(index)
        }));
        self.on_change = Some(shared);
        self
    }

    pub fn props(&self) -> &RadioGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialRadioGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::RadioGroup(tokens) = theme.component(ComponentId::RadioGroup) else {
            unreachable!("ComponentId::RadioGroup must resolve to RadioGroupTokens")
        };
        MaterialRadioGroupStyle {
            design_system: "material",
            selected_color: theme.colors().primary,
            unselected_color: theme.colors().outline,
            radius: 9999.0,
            radio_size: tokens.radio_size,
            item_height: tokens.item_height,
            gap: tokens.gap,
        }
    }

    /// The group: the options, and the one choice they share.
    pub fn view(self) -> SelectGroup {
        let style = self.resolved_style();
        let selected = self.props.selected;
        let on_change = self.on_change.clone();
        let mut group = SelectGroup::new()
            .mode(SelectionMode::Single)
            .disabled(self.props.disabled)
            .style(SelectGroupStyle {
                gap: style.gap,
                axis: Axis::Vertical,
                height: Some(style.item_height),
                pad: 0.0,
                content_placement: ContentPlacement::Centred,
                background: PaintColor::rgba(0, 0, 0, 0),
                foreground: PaintColor::rgba(0, 0, 0, 0),
                border_width: 0.0,
                border_color: PaintColor::rgba(0, 0, 0, 0),
                radius: 0.0,
                selected_background: PaintColor::rgba(0, 0, 0, 0),
                selected_foreground: PaintColor::rgba(0, 0, 0, 0),
                selected_border_color: PaintColor::rgba(0, 0, 0, 0),
                toggles_off_in_single_mode: false,
                mandatory_in_multiple_mode: false,
            });

        for label in self.props.options {
            let style = style;
            let label = label.clone();
            group = group.row_with_state(label.clone(), move |chosen| {
                let dot = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width(style.radio_size * 0.5)
                            .height(style.radio_size * 0.5),
                    )
                    .background(to_paint(style.selected_color))
                    .radius(style.radius);
                let mut ring = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width(style.radio_size)
                            .height(style.radio_size)
                            .main_align(Align::Center)
                            .cross_align(Align::Center),
                    )
                    .radius(style.radius)
                    .border(
                        RING_BORDER,
                        if chosen {
                            to_paint(style.selected_color)
                        } else {
                            to_paint(style.unselected_color)
                        },
                    );
                if chosen {
                    ring = ring.push({
                        let dot = dot.clone();
                        move || Box::new(dot.clone())
                    });
                }

                let row = Stack::new(Axis::Horizontal)
                    .gap(8.0)
                    .push({
                        let ring = ring.clone();
                        move || Box::new(ring.clone())
                    })
                    .push({
                        let label = label.clone();
                        move || Box::new(Text::new(label.clone()))
                    });

                Box::new(row)
            });
        }

        if let Some(on_change) = on_change {
            group = group.on_change(move |chosen| {
                if let Some(index) = chosen.last() {
                    on_change(*index);
                }
            });
        }

        if let Some(index) = selected {
            group = group.selected([index]);
        }

        group
    }
}

impl Default for MaterialRadioGroup {
    fn default() -> Self {
        Self::new()
    }
}
