use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::calendar::{CalendarDate, CalendarProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::typography::FontWeight;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdCalendarStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub border_radius: f32,
    pub cell_size: f32,
    pub panel_width: f32,
    pub header_font_size: f32,
    pub day_font_size: f32,
    pub weekday_font_size: f32,
    pub weeknumber_width: f32,
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Weekday of the 1st of the given month: 0 = Sunday … 6 = Saturday.
fn weekday_of_first(y: i32, m: u32) -> u32 {
    let mut days: i64 = 0;
    let mut yy = 1970;
    while yy < y {
        days += if is_leap(yy) { 366 } else { 365 };
        yy += 1;
    }
    let mut mm = 1u32;
    while mm < m {
        days += days_in_month(y, mm) as i64;
        mm += 1;
    }
    (((4 + days) % 7 + 7) % 7) as u32
}

/// Day number shown in the fixed cell `k` (0..42) for the displayed month, or `None` for a
/// leading/trailing blank cell.
fn day_at(k: usize, y: i32, m: u32) -> Option<u32> {
    let offset = weekday_of_first(y, m) as i32;
    let n = k as i32 - offset + 1;
    if n >= 1 && n <= days_in_month(y, m) as i32 {
        Some(n as u32)
    } else {
        None
    }
}

fn prev_month(y: i32, m: u32) -> (i32, u32) {
    if m == 1 {
        (y - 1, 12)
    } else {
        (y, m - 1)
    }
}

fn next_month(y: i32, m: u32) -> (i32, u32) {
    if m == 12 {
        (y + 1, 1)
    } else {
        (y, m + 1)
    }
}

fn day_of_year(y: i32, m: u32, d: u32) -> u32 {
    let mut doy = d;
    let mut mm = 1u32;
    while mm < m {
        doy += days_in_month(y, mm);
        mm += 1;
    }
    doy
}

/// Approximate ISO week number for display in the optional week-number column.
fn week_number(y: i32, m: u32, d: u32) -> u32 {
    let dow = weekday_of_first(y, m); // 0=Sun..6=Sat for day 1
    let weekday_monday = (dow + 6) % 7; // 0=Mon..6=Sun
    let doy = day_of_year(y, m, d);
    ((doy as i64 + 6 - weekday_monday as i64) / 7 + 1) as u32
}

/// Absolute (possibly out-of-month) date of grid cell `k`, used for week numbers.
fn abs_date_of_cell(k: usize, y: i32, m: u32) -> (i32, u32, u32) {
    let offset = weekday_of_first(y, m) as i32;
    let day = (k as i32) - offset + 1;
    if day >= 1 && day <= days_in_month(y, m) as i32 {
        (y, m, day as u32)
    } else if day < 1 {
        let (py, pm) = prev_month(y, m);
        let pd = days_in_month(py, pm) as i32 + day;
        (py, pm, pd as u32)
    } else {
        let (ny, nm) = next_month(y, m);
        (ny, nm, (day - days_in_month(y, m) as i32) as u32)
    }
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

pub struct MdCalendar {
    props: CalendarProps,
    value: RwSignal<CalendarDate>,
    view_month: RwSignal<(i32, u32)>,
    theme: RwSignal<MultiTheme>,
}

impl MdCalendar {
    pub fn new() -> Self {
        let props = CalendarProps::default();
        let vm = (props.value.year, props.value.month);
        Self {
            value: RwSignal::new(props.value),
            view_month: RwSignal::new(vm),
            theme: theme_signal(),
            props,
        }
    }

    pub fn value(mut self, value: CalendarDate) -> Self {
        self.props.value = value;
        self.value.set(value);
        self.view_month.set((value.year, value.month));
        self
    }

    pub fn show_week_numbers(mut self, show: bool) -> Self {
        self.props.show_week_numbers = show;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(CalendarDate) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdCalendarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Calendar(tokens) = theme.component(ComponentId::Calendar) else {
            unreachable!("ComponentId::Calendar must resolve to CalendarTokens")
        };
        MdCalendarStyle {
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

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value;
        let view_month = self.view_month;
        let on_change: Rc<Option<Box<dyn Fn(CalendarDate)>>> = Rc::new(self.props.on_change);
        let show_week_numbers = self.props.show_week_numbers;

        let tokens = match theme.get().component(ComponentId::Calendar) {
            ComponentTokens::Calendar(t) => t,
            _ => unreachable!(),
        };
        let header_font = tokens.header_font_size;
        let day_font = tokens.day_font_size;
        let weekday_font = tokens.weekday_font_size;
        let cell_size = tokens.cell_size;
        let border_radius = tokens.border_radius;
        let panel_width = tokens.panel_width;
        let weeknumber_width = tokens.weeknumber_width;

        let prev_btn = Label::new("‹")
            .style(move |s| {
                let t = theme.get();
                s.color(t.colors().on_surface.to_peniko())
                    .font_size(18.0)
                    .padding(4.0)
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                let (y, m) = view_month.get();
                view_month.set(prev_month(y, m));
                EventPropagation::Continue
            });

        let next_btn = Label::new("›")
            .style(move |s| {
                let t = theme.get();
                s.color(t.colors().on_surface.to_peniko())
                    .font_size(18.0)
                    .padding(4.0)
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                let (y, m) = view_month.get();
                view_month.set(next_month(y, m));
                EventPropagation::Continue
            });

        let title = Label::derived(move || {
            let (y, m) = view_month.get();
            format!("{} {}", MONTHS[(m as usize) - 1], y)
        })
        .style(move |s| {
            s.color(theme.get().colors().on_surface.to_peniko())
                .font_size(header_font)
                .font_weight(FontWeight::Medium.to_floom())
        });

        let header = Stack::horizontal((prev_btn, title, next_btn))
            .style(|s| s.justify_between().items_center().width_pct(100.0));

        let mut weekday_cells: Vec<AnyView> = Vec::new();
        if show_week_numbers {
            weekday_cells.push(
                Empty::new()
                    .style(move |s| s.width(weeknumber_width))
                    .into_any(),
            );
        }
        for w in ["S", "M", "T", "W", "T", "F", "S"] {
            weekday_cells.push(
                Label::new(w)
                    .style(move |s| {
                        s.width(cell_size)
                            .height(cell_size)
                            .items_center()
                            .justify_center()
                            .color(theme.get().colors().on_surface_variant.to_peniko())
                            .font_size(weekday_font)
                    })
                    .into_any(),
            );
        }
        let weekday_row = Stack::horizontal_from_iter(weekday_cells)
            .style(|s| s.justify_between().width_pct(100.0));

        let rows: Vec<AnyView> = (0..6)
            .map(|r| {
                let week: Vec<AnyView> = (0..7)
                    .map(|c| {
                        let k = (r * 7 + c) as usize;
                        let value = value;
                        let view_month = view_month;
                        let on_change = on_change.clone();
                        let theme = theme;
                        Label::derived(move || {
                            let (yy, mm) = view_month.get();
                            match day_at(k, yy, mm) {
                                Some(n) => n.to_string(),
                                None => String::new(),
                            }
                        })
                        .style(move |s| {
                            let (yy, mm) = view_month.get();
                            let day = day_at(k, yy, mm);
                            let vis = day.is_some();
                            let sel = day
                                .map(|n| {
                                    value.get()
                                        == CalendarDate {
                                            year: yy,
                                            month: mm,
                                            day: n,
                                        }
                                })
                                .unwrap_or(false);
                            s.width(cell_size)
                                .height(cell_size)
                                .items_center()
                                .justify_center()
                                .color(
                                    if sel {
                                        theme.get().colors().on_primary
                                    } else {
                                        theme.get().colors().on_surface
                                    }
                                    .to_peniko(),
                                )
                                .font_size(day_font)
                                .background(
                                    if sel {
                                        theme.get().colors().primary.to_peniko()
                                    } else {
                                        Color::rgba(0, 0, 0, 0).to_peniko()
                                    },
                                )
                                .border_radius(cell_size / 2.0)
                                .cursor(CursorStyle::Pointer)
                                .hover(move |s| {
                                    s.background(
                                        with_alpha(theme.get().colors().primary, 16).to_peniko(),
                                    )
                                })
                                .apply_if(!vis, |s| s.hide())
                        })
                        .on_event(listener::Click, move |_, _| {
                            let (yy, mm) = view_month.get();
                            if let Some(n) = day_at(k, yy, mm) {
                                let cd = CalendarDate {
                                    year: yy,
                                    month: mm,
                                    day: n,
                                };
                                value.set(cd);
                                if let Some(cb) = on_change.as_ref() {
                                    cb(cd);
                                }
                            }
                            EventPropagation::Continue
                        })
                        .into_any()
                    })
                    .collect();
                let week_row: Vec<AnyView> = if show_week_numbers {
                    let (yy, mm) = view_month.get();
                    let (wy, wm, wd) = abs_date_of_cell(r * 7, yy, mm);
                    let wnum = week_number(wy, wm, wd);
                    let wnum_view = Label::new(wnum.to_string())
                        .style(move |s| {
                            s.width(weeknumber_width)
                                .height(cell_size)
                                .items_center()
                                .justify_center()
                                .color(theme.get().colors().on_surface_variant.to_peniko())
                                .font_size(weekday_font)
                        })
                        .into_any();
                    let mut row = vec![wnum_view];
                    row.extend(week);
                    row
                } else {
                    week
                };
                Stack::horizontal_from_iter(week_row)
                    .style(|s| s.justify_between())
                    .into_any()
            })
            .collect();

        let grid = list(rows).style(|s| s.gap(2.0));

        let panel = Container::new(Stack::vertical((header, weekday_row, grid)).style(|s| s.gap(6.0)))
            .style(move |s| {
                let t = theme.get();
                s.width(panel_width)
                    .padding(10.0)
                    .background(t.colors().surface.to_peniko())
                    .border_radius(border_radius)
                    .border(1.0)
                    .border_color(with_alpha(t.colors().outline, 120).to_peniko())
            });

        panel
    }
}

impl Default for MdCalendar {
    fn default() -> Self {
        Self::new()
    }
}
