//! Fluent time picker, ported to the Floom 2.0 widget API.

//!

use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Shadow};
use floom_ui_composites::TimePicker;
pub use floom_ui_composites::{SharedTimeState, TimePickerState, TimePickerStyle, TimeValue};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::time_picker::{TimePickerProps, TimeValue as ThemeTimeValue};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const INCREASE: &str = "+";

fn to_engine_time(value: ThemeTimeValue) -> floom_ui_composites::TimeValue {
    floom_ui_composites::TimeValue {
        hour: value.hour,
        minute: value.minute,
    }
}

fn to_theme_time(value: floom_ui_composites::TimeValue) -> ThemeTimeValue {
    ThemeTimeValue {
        hour: value.hour,
        minute: value.minute,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentTimePickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub height: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub panel_width: f32,
    pub cell_height: f32,
    pub padding_horizontal: f32,
    pub border_width: f32,
    pub disabled_border_alpha: u8,
    pub panel_padding: f32,
    pub panel_border_alpha: u8,
    pub panel_shadow: Shadow,
    pub panel_gap: f32,
    pub row_gap: f32,
    pub stepper_size: f32,
    pub stepper_radius: f32,
    pub stepper_fill_alpha: u8,
    pub stepper_font_size: f32,
    pub value_font_size: f32,
    pub value_width: f32,
    pub done_height: f32,
    pub done_padding_horizontal: f32,
    pub text_size: f32,
}

pub struct FluentTimePicker {
    props: TimePickerProps,
    theme: RwSignal<MultiTheme>,
    state: SharedTimeState,
    on_change: Option<Arc<dyn Fn(TimeValue) + Send + Sync>>,
}

impl FluentTimePicker {
    pub fn new() -> Self {
        let props = TimePickerProps::default();
        Self {
            state: Arc::new(Mutex::new(TimePickerState {
                value: props.value.map(to_engine_time),
                open: false,
            })),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props,
            on_change: None,
        }
    }

    pub fn value(mut self, value: TimeValue) -> Self {
        self.props.value = Some(to_theme_time(value));
        floom_ui_composites::write_state(&self.state, |guard| guard.value = Some(value));
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(TimeValue) + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn(TimeValue) + Send + Sync> = Arc::new(callback);
        self.on_change = Some(shared.clone());
        self.props.on_change = Some(Box::new(move |value: ThemeTimeValue| {
            shared(to_engine_time(value))
        }));
        self
    }

    pub fn props(&self) -> &TimePickerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn state(&self) -> SharedTimeState {
        self.state.clone()
    }

    pub fn selected(&self) -> Option<TimeValue> {
        floom_ui_composites::read_state(&self.state).value
    }

    pub fn resolved_style(&self) -> FluentTimePickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::TimePicker(tokens) = theme.component(ComponentId::TimePicker) else {
            unreachable!("ComponentId::TimePicker must resolve to TimePickerTokens")
        };
        FluentTimePickerStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            primary: theme.colors().primary,
            height: tokens.height,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            panel_width: tokens.panel_width,
            cell_height: tokens.cell_height,
            padding_horizontal: 12.0,
            border_width: 1.0,
            disabled_border_alpha: 97,
            panel_padding: 12.0,
            panel_border_alpha: 120,
            panel_shadow: Shadow {
                color: to_paint(with_alpha(Color::BLACK, 24)),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
            panel_gap: 8.0,
            row_gap: 8.0,
            stepper_size: 32.0,
            stepper_radius: 16.0,
            stepper_fill_alpha: 8,
            stepper_font_size: 18.0,
            value_font_size: 24.0,
            value_width: 48.0,
            done_height: 32.0,
            done_padding_horizontal: 12.0,
            text_size: theme.typography().body.medium.font_size,
        }
    }

    pub fn view(self) -> TimePicker {
        let style = self.resolved_style();
        let mut picker = TimePicker::new()
            .style(TimePickerStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                outline: to_paint(style.outline),
                primary: to_paint(style.primary),
                height: style.height,
                border_radius: style.border_radius,
                min_width: style.min_width,
                panel_width: style.panel_width,
                cell_height: style.cell_height,
                padding_horizontal: style.padding_horizontal,
                border_width: style.border_width,
                disabled_border_alpha: style.disabled_border_alpha,
                panel_padding: style.panel_padding,
                panel_border_alpha: style.panel_border_alpha,
                panel_shadow: style.panel_shadow,
                panel_gap: style.panel_gap,
                row_gap: style.row_gap,
                stepper_size: style.stepper_size,
                stepper_radius: style.stepper_radius,
                stepper_fill_alpha: style.stepper_fill_alpha,
                stepper_font_size: style.stepper_font_size,
                value_font_size: style.value_font_size,
                value_width: style.value_width,
                done_height: style.done_height,
                done_padding_horizontal: style.done_padding_horizontal,
                text_size: style.text_size,
            })
            .state(self.state.clone())
            .disabled(self.props.disabled)
            .placeholder(self.props.placeholder.clone());
        if let Some(on_change) = self.on_change.clone() {
            picker = picker.on_change(move |value| on_change(value));
        }
        picker
    }
}

impl Default for FluentTimePicker {
    fn default() -> Self {
        Self::new()
    }
}
