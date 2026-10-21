//! Material calendar, ported to the Floom 2.0 widget API.
//!
//! A calendar is a month of days: a header that says which month and moves it,
//! a row of weekday letters, and six weeks of day cells, one of which is the
//! selected date.
//!
//! ## What the port uses
//!
//! The engine's `Calendar`: the month grid, the header that moves between
//! months, and the day cells. The three floem-ui calendar kits each used to
//! carry their own copy of all of it.
//!
//! The engine's vocabulary: a [`Surface`] for the panel, a [`Stack`] for the
//! column and for the rows, a [`Text`] for the labels, and a [`Pressable`] for
//! the day cells and the two header buttons. The one thing the kit carries is
//! the **body** -- the header, the weekday row and the grid -- because the
//! state the calendar draws (the selected date, and the month on display)
//! changes after the tree is mounted, and a `Stack` built once cannot answer
//! that: the day numbers and the selection are baked into the widgets it
//! holds.
//!
//! ## What this kit answers
//!
//! * the day cell's size, the panel's width, and the three font sizes: the
//!   tokens;
//! * the panel's fill: `surface`; its outline: `outline` at 120;
//! * the selected day's fill: `primary`; its number: `on_primary`; every
//!   other day's number: `on_surface`; the weekday letters and the week
//!   numbers: `on_surface_variant`.
//!
//! ## The token that never showed
//!
//! `CalendarTokens::header_height` is declared and never read: v1's header was
//! a row of text, not a bar of a fixed height, and the port's header is the
//! same row of text. v1's style did not report it either, so the port's does
//! not.
//!
//! ## The numbers v1 wrote as literals
//!
//! The header buttons' size (18) and their padding (4).
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The day cells' hover.** v1 wrote
//!   `.hover(|s| s.background(with_alpha(primary, 16)))` on every cell. 2.0's
//!   `Pressable` owns the interaction state and paints no background of its
//!   own, and there is no hover background for a cell that is not one of the
//!   engine's own composites. The hover is named rather than faked.
//! * **The day cells' pointer cursor.** v1 asked for a pointer over a cell.
//!   2.0's `Pressable` does not report a cursor -- the engine's `Button` does,
//!   and a `Button` is a label rather than a cell. The cursor is named rather
//!   than faked.
//! * **The callback is not in the props.** `CalendarProps::on_change` is a
//!   `Box<dyn Fn(CalendarDate)>`, which is not `Send + Sync`, and 2.0's slots
//!   are. The kit keeps its own `Arc<dyn Fn(CalendarDate) + Send + Sync>` and
//!   the props' slot stays empty, so `props()` reports `None` where v1
//!   reported `Some`.
//! * **The accessors.** v1's Material calendar had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.
//!
//! ## The state v1 held in signals
//!
//! v1 held the selected date and the month on display in two `RwSignal`s.
//! `RwSignal` is `Rc`-based and cannot cross a 2.0 slot boundary, so the port
//! holds two shared cells instead, and the engine has no shared type for a
//! date -- the kit carries its own.
//!
//! ## The cells v1 hid
//!
//! v1 drew all 42 cells and *hid* the ones that fall outside the month. A
//! hidden cell took no room, so the month's days spread across the row. The
//! port does not mount those cells at all, which is the same picture by a
//! shorter road.
//!
//! ## Where a month picker would go
//!
//! v1's calendar is an inline panel: the month moves with the two header
//! buttons, and there is no picker to open. The port keeps it that way. The
//! engine's `Popup` and `Overlay` are how a *caller* would make this a layer
//! -- a date field that opens a calendar in a popup -- and the kit does not
//! use them, because v1's calendar had no layer of its own to build.

use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_ui_composites::{Calendar, CalendarStyle};
use floom_types::Color as PaintColor;
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::calendar::{CalendarDate as ThemeDate, CalendarProps};
use floom_core_theme::theme::{MultiTheme, Theme};
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

/// The calendar's appearance, in the terms v1 used.
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
    /// The kit's own handle on the callback.
    ///
    /// Not the props': `CalendarProps::on_change` is a `Box<dyn Fn(CalendarDate)>`,
    /// and 2.0's slots are `Send + Sync`.
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

    /// The selected date. v1 also moved the month on display to it.
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

/// The calendar's body: the header, the weekday row and the grid.
///
/// A widget of its own because the state it draws changes after the tree is
/// mounted. The keys its children carry hold that state, so the parts that
/// changed are remounted and the parts that did not are not.
/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
