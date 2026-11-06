use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Shadow};
use floom_ui_composites::DatePicker;
pub use floom_ui_composites::{DatePickerState, DatePickerStyle, DateValue, SharedDateState};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::date_picker::{DatePickerProps, DateValue as ThemeDateValue};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

fn to_engine_date(value: ThemeDateValue) -> floom_ui_composites::DateValue {
    floom_ui_composites::DateValue {
        year: value.year,
        month: value.month,
        day: value.day,
    }
}

/// The engine's date value -> the theme's.
fn to_theme_date(value: floom_ui_composites::DateValue) -> ThemeDateValue {
    ThemeDateValue {
        year: value.year,
        month: value.month,
        day: value.day,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialDatePickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub height: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub panel_width: f32,
    pub cell_size: f32,
    pub header_font_size: f32,
    pub padding_horizontal: f32,
    pub border_width: f32,
    pub disabled_border_alpha: u8,
    pub panel_padding: f32,
    pub panel_border_alpha: u8,
    pub panel_shadow: Shadow,
    pub panel_gap: f32,
    pub grid_gap: f32,
    pub weekday_font_size: f32,
    pub arrow_font_size: f32,
    pub arrow_padding: f32,
    /// The field's label size: the theme's body-medium.
    pub text_size: f32,
    pub hover_alpha: u8,
}

/// Material's date picker.
pub struct MaterialDatePicker {
    props: DatePickerProps,
    theme: RwSignal<MultiTheme>,
    /// The picker's state, shared with the tree it builds.
    state: SharedDateState,
    /// The callback, held as a `Send + Sync` handle because the tree's slots
    /// have to be.
    on_change: Option<Arc<dyn Fn(DateValue) + Send + Sync>>,
}

impl MaterialDatePicker {
    pub fn new() -> Self {
        let props = DatePickerProps::default();
        let view_month = props
            .value
            .map(|d| (d.year, d.month))
            .unwrap_or(floom_ui_composites::DEFAULT_VIEW_MONTH);
        Self {
            state: Arc::new(Mutex::new(DatePickerState {
                value: props.value.map(to_engine_date),
                open: false,
                view_month,
            })),
            theme: theme_signal(),
            props,
            on_change: None,
        }
    }

    /// The date to show, and the month the calendar opens on.
    pub fn value(mut self, value: DateValue) -> Self {
        self.props.value = Some(to_theme_date(value));
        floom_ui_composites::write_date_state(&self.state, |guard| {
            guard.value = Some(value);
            guard.view_month = (value.year, value.month);
        });
        self
    }

    /// The text shown while nothing is chosen.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    /// Whether the picker answers at all.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(DateValue) + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn(DateValue) + Send + Sync> = Arc::new(callback);
        self.on_change = Some(shared.clone());
        self.props.on_change = Some(Box::new(move |value: ThemeDateValue| {
            shared(to_engine_date(value))
        }));
        self
    }

    pub fn props(&self) -> &DatePickerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The picker's shared state, for a caller that wants to read it.
    pub fn state(&self) -> SharedDateState {
        self.state.clone()
    }

    /// The date the picker holds, if any.
    pub fn selected(&self) -> Option<DateValue> {
        floom_ui_composites::read_date_state(&self.state).value
    }

    pub fn resolved_style(&self) -> MaterialDatePickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::DatePicker(tokens) = theme.component(ComponentId::DatePicker) else {
            unreachable!("ComponentId::DatePicker must resolve to DatePickerTokens")
        };
        MaterialDatePickerStyle {
            design_system: "material",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            primary: theme.colors().primary,
            on_primary: theme.colors().on_primary,
            height: tokens.height,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            panel_width: tokens.panel_width,
            cell_size: tokens.cell_size,
            header_font_size: tokens.header_font_size,
            padding_horizontal: 12.0,
            border_width: 1.0,
            disabled_border_alpha: 97,
            panel_padding: 8.0,
            panel_border_alpha: 120,
            panel_shadow: Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 24)),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
            panel_gap: 6.0,
            grid_gap: 2.0,
            weekday_font_size: 11.0,
            arrow_font_size: 18.0,
            arrow_padding: 4.0,
            text_size: theme.typography().body.medium.font_size,
            hover_alpha: 16,
        }
    }

    /// The picker, ready to mount.
    pub fn view(self) -> DatePicker {
        let style = self.resolved_style();
        let mut picker = DatePicker::new()
            .style(DatePickerStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                outline: to_paint(style.outline),
                primary: to_paint(style.primary),
                on_primary: to_paint(style.on_primary),
                height: style.height,
                border_radius: style.border_radius,
                min_width: style.min_width,
                panel_width: style.panel_width,
                cell_size: style.cell_size,
                header_font_size: style.header_font_size,
                padding_horizontal: style.padding_horizontal,
                border_width: style.border_width,
                disabled_border_alpha: style.disabled_border_alpha,
                panel_padding: style.panel_padding,
                panel_border_alpha: style.panel_border_alpha,
                panel_shadow: style.panel_shadow,
                panel_gap: style.panel_gap,
                grid_gap: style.grid_gap,
                weekday_font_size: style.weekday_font_size,
                arrow_font_size: style.arrow_font_size,
                arrow_padding: style.arrow_padding,
                text_size: style.text_size,
                hover_alpha: style.hover_alpha,
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

impl Default for MaterialDatePicker {
    fn default() -> Self {
        Self::new()
    }
}
