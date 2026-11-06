use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Calendar, CalendarStyle};
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::calendar::{CalendarDate as ThemeDate, CalendarProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// The selected date, shared between the kit and the cells that draw it.
/// The cell the chosen date lives in.
pub type SharedDate = floom_ui_composites::SharedDate;
/// The cell the month on show lives in.
pub type SharedMonth = floom_ui_composites::SharedMonth;
/// The engine's calendar date.
pub use floom_ui_composites::CalendarDate;

/// The theme's calendar date -> the engine's.
fn to_engine_date(value: ThemeDate) -> floom_ui_composites::CalendarDate {
    floom_ui_composites::CalendarDate {
        year: value.year,
        month: value.month,
        day: value.day,
    }
}

/// The engine's calendar date -> the theme's.
fn to_theme_date(value: floom_ui_composites::CalendarDate) -> ThemeDate {
    ThemeDate {
        year: value.year,
        month: value.month,
        day: value.day,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialCalendarStyle {
    pub design_system: &'static str,
    /// The panel's fill.
    pub surface: Color,
    /// The day numbers' and the header's ink.
    pub on_surface: Color,
    /// The weekday letters' and the week numbers' ink.
    pub on_surface_variant: Color,
    /// The panel's outline.
    pub outline: Color,
    /// The selected day's fill.
    pub primary: Color,
    /// The selected day's number.
    pub on_primary: Color,
    /// The panel's corner radius.
    pub border_radius: f32,
    /// A day cell's size.
    pub cell_size: f32,
    /// The panel's width.
    pub panel_width: f32,
    /// The month and year's size.
    pub header_font_size: f32,
    /// The day numbers' size.
    pub day_font_size: f32,
    /// The weekday letters' size.
    pub weekday_font_size: f32,
    /// The week-number column's width.
    pub weeknumber_width: f32,
}

/// Material's calendar.
pub struct MaterialCalendar {
    props: CalendarProps,
    theme: RwSignal<MultiTheme>,
    /// The selected date, shared with the cells.
    value: SharedDate,
    /// The month on display, shared with the cells.
    view_month: SharedMonth,
    on_change: Option<Arc<dyn Fn(CalendarDate) + Send + Sync + 'static>>,
}

impl MaterialCalendar {
    pub fn new() -> Self {
        let props = CalendarProps::default();
        Self {
            value: Arc::new(Mutex::new(to_engine_date(props.value))),
            view_month: Arc::new(Mutex::new((props.value.year, props.value.month))),
            theme: theme_signal(),
            props,
            on_change: None,
        }
    }

    pub fn value(mut self, value: CalendarDate) -> Self {
        self.props.value = to_theme_date(value);
        floom_ui_composites::write_value(&self.value, value);
        floom_ui_composites::write_month(&self.view_month, (value.year, value.month));
        self
    }

    /// Whether to draw the week-number column.
    pub fn show_week_numbers(mut self, show: bool) -> Self {
        self.props.show_week_numbers = show;
        self
    }

    /// Called with the newly picked date when a day is clicked.
    pub fn on_change(mut self, callback: impl Fn(CalendarDate) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &CalendarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialCalendarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Calendar(tokens) = theme.component(ComponentId::Calendar) else {
            unreachable!("ComponentId::Calendar must resolve to CalendarTokens")
        };
        MaterialCalendarStyle {
            design_system: "material",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            primary: theme.colors().primary,
            on_primary: theme.colors().on_primary,
            border_radius: tokens.border_radius,
            cell_size: tokens.cell_size,
            panel_width: tokens.panel_width,
            header_font_size: tokens.header_font_size,
            day_font_size: tokens.day_font_size,
            weekday_font_size: tokens.weekday_font_size,
            weeknumber_width: tokens.weeknumber_width,
        }
    }

    /// The panel: the calendar, ready to mount.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let mut calendar = Calendar::new()
            .style(CalendarStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                outline: to_paint(style.outline),
                primary: to_paint(style.primary),
                on_primary: to_paint(style.on_primary),
                border_radius: style.border_radius,
                cell_size: style.cell_size,
                panel_width: style.panel_width,
                header_font_size: style.header_font_size,
                day_font_size: style.day_font_size,
                weekday_font_size: style.weekday_font_size,
                weeknumber_width: style.weeknumber_width,
            })
            .value(self.value.clone())
            .view_month(self.view_month.clone())
            .show_week_numbers(self.props.show_week_numbers);
        if let Some(on_change) = self.on_change.clone() {
            calendar = calendar.on_change(move |date| on_change(date));
        }
        calendar.build()
    }
}

impl Default for MaterialCalendar {
    fn default() -> Self {
        Self::new()
    }
}
