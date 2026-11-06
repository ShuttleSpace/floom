use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Insets, LayoutRequest, PaintContext,
    UpdateContext, Widget, WidgetAny,
};
use floom_types::FontWeight;
use floom_widgets::{Align, Axis, Pressable, Spacer, Stack, Surface, Text};

use crate::color_picker::with_alpha;

/// The header buttons' padding. A v1 literal.
const BUTTON_PADDING: f64 = 4.0;

/// The header buttons' font size. A v1 literal.
const BUTTON_FONT_SIZE: f32 = 18.0;

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDate {
    /// The year.
    pub year: i32,
    /// The month, 1..=12.
    pub month: u32,
    /// The day of the month, 1..=31.
    pub day: u32,
}

impl Default for CalendarDate {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 1,
            day: 1,
        }
    }
}

/// The chosen date, shared with the caller.
pub type SharedDate = Arc<Mutex<CalendarDate>>;

/// The month on show, shared with the caller.
pub type SharedMonth = Arc<Mutex<(i32, u32)>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalendarStyle {
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

impl Default for CalendarStyle {
    fn default() -> Self {
        Self {
            surface: Color::rgb(0, 0, 0),
            on_surface: Color::rgb(0, 0, 0),
            on_surface_variant: Color::rgb(0, 0, 0),
            outline: Color::rgb(0, 0, 0),
            primary: Color::rgb(0, 0, 0),
            on_primary: Color::rgb(0, 0, 0),
            border_radius: 0.0,
            cell_size: 0.0,
            panel_width: 0.0,
            header_font_size: 0.0,
            day_font_size: 0.0,
            weekday_font_size: 0.0,
            weeknumber_width: 0.0,
        }
    }
}

/// The month on display, shared between the kit and the cells that draw it.

/// Read the selected date, treating a poisoned cell as the default.
pub fn read_value(value: &SharedDate) -> CalendarDate {
    *value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Write the selected date, ignoring a poisoned cell.
pub fn write_value(value: &SharedDate, date: CalendarDate) {
    if let Ok(mut guard) = value.lock() {
        *guard = date;
    }
}

/// Read the month on display, treating a poisoned cell as January 1970.
pub fn read_month(month: &SharedMonth) -> (i32, u32) {
    *month
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Write the month on display, ignoring a poisoned cell.
pub fn write_month(month: &SharedMonth, value: (i32, u32)) {
    if let Ok(mut guard) = month.lock() {
        *guard = value;
    }
}

#[derive(Clone)]
struct CalendarBody {
    /// The selected date.
    value: SharedDate,
    /// The month on display.
    view_month: SharedMonth,
    /// The kit's handle on the callback.
    on_change: Option<Arc<dyn Fn(CalendarDate) + Send + Sync + 'static>>,
    /// The calendar's appearance.
    style: CalendarStyle,
    /// Whether to draw the week-number column.
    show_week_numbers: bool,
    /// The state this widget last declared its children for.
    last: Option<(CalendarDate, (i32, u32))>,
}

impl Widget for CalendarBody {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let state = (read_value(&self.value), read_month(&self.view_month));
        if self.last != Some(state) {
            self.last = Some(state);
            // The day numbers and the selection are baked into the cells, so
            // the cells have to be rebuilt rather than repainted.
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let (year, month) = read_month(&self.view_month);
        vec![
            month_header(year, month, self.view_month.clone(), self.style),
            weekday_row(self.show_week_numbers, self.style),
            month_grid(
                year,
                month,
                self.value.clone(),
                self.on_change.clone(),
                self.style,
                self.show_week_numbers,
            ),
        ]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // v1's `Stack::vertical((header, weekday_row, grid)).style(|s| s.gap(6.0))`.
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .stack(Axis::Vertical, 6.0),
        )
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}
}

impl WidgetAny for CalendarBody {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// The header: the month and the year, and the two buttons that move it.
///
/// The key carries the month, so a month that moves remounts the header and
/// the title with it -- the title's text is a month and a year, and a `Text`
/// that is not rebuilt keeps the month it was built with.
fn month_header(year: i32, month: u32, view_month: SharedMonth, style: CalendarStyle) -> Child {
    let ink = style.on_surface;
    let title = format!("{} {}", MONTHS[(month as usize) - 1], year);
    let title_size = style.header_font_size;
    Child::new(
        ChildKey::keyed(month_key(year, month)),
        LayoutRequest::new().width_pct(100.0),
        move || {
            let mut header = Stack::new(Axis::Horizontal)
                // v1's `.justify_between().items_center()`.
                .align(Align::SpaceBetween, Align::Center)
                .request(LayoutRequest::new().width_pct(100.0));
            header = header.push({
                let view_month = view_month.clone();
                move || month_button("‹", view_month.clone(), ink)
            });
            header = header.push({
                // The inner closure owns its copy: a replayed closure cannot
                // move a value out of its own capture.
                let title = title.clone();
                move || {
                    Box::new(
                        Text::new(title.clone())
                            .size(title_size)
                            // v1's medium weight.
                            .weight(FontWeight::Medium)
                            .color(ink),
                    )
                }
            });
            header = header.push({
                let view_month = view_month.clone();
                move || month_button("›", view_month.clone(), ink)
            });
            Box::new(header)
        },
    )
}

/// One of the two header buttons: the previous month and the next month.
fn month_button(glyph: &'static str, view_month: SharedMonth, ink: Color) -> Box<dyn WidgetAny> {
    let forward = glyph == "›";
    Box::new(
        Pressable::new(Child::new(
            ChildKey::index(0),
            // v1's `.padding(4.0)` on the button.
            LayoutRequest::new().padding(Insets::uniform(BUTTON_PADDING)),
            move || Box::new(Text::new(glyph).size(BUTTON_FONT_SIZE).color(ink)),
        ))
        .on_click(move || {
            let (year, month) = read_month(&view_month);
            let next = if forward {
                next_month(year, month)
            } else {
                prev_month(year, month)
            };
            write_month(&view_month, next);
        }),
    )
}

/// The weekday row: the letters, and the head of the week-number column.
fn weekday_row(show_week_numbers: bool, style: CalendarStyle) -> Child {
    let ink = style.on_surface_variant;
    let size = style.weekday_font_size;
    let cell_size = style.cell_size;
    let weeknumber_width = style.weeknumber_width;
    Child::new(
        ChildKey::index(1),
        LayoutRequest::new().width_pct(100.0),
        move || {
            let mut row = Stack::new(Axis::Horizontal)
                // v1's `.justify_between()`.
                .align(Align::SpaceBetween, Align::Center)
                .request(LayoutRequest::new().width_pct(100.0));
            if show_week_numbers {
                row = row.push_layout(LayoutRequest::new().width(weeknumber_width), move || {
                    Box::new(Spacer::new(f64::from(weeknumber_width)).axis(Axis::Horizontal))
                });
            }
            for letter in ["S", "M", "T", "W", "T", "F", "S"] {
                let letter = letter.to_string();
                row = row.push_layout(
                    LayoutRequest::new().width(cell_size).height(cell_size),
                    move || {
                        // The inner closure owns its copy: a replayed closure
                        // cannot move a value out of its own capture.
                        let letter = letter.clone();
                        Box::new(
                            Stack::new(Axis::Horizontal)
                                // v1's `.items_center().justify_center()`.
                                .align(Align::Center, Align::Center)
                                .request(LayoutRequest::new().width(cell_size).height(cell_size))
                                .push(move || {
                                    Box::new(Text::new(letter.clone()).size(size).color(ink))
                                }),
                        )
                    },
                );
            }
            Box::new(row)
        },
    )
}

/// The grid: six weeks of days.
///
/// The key carries the month and the selection, because the day numbers and
/// the selected day's fill are baked into the cells.
fn month_grid(
    year: i32,
    month: u32,
    value: SharedDate,
    on_change: Option<Arc<dyn Fn(CalendarDate) + Send + Sync + 'static>>,
    style: CalendarStyle,
    show_week_numbers: bool,
) -> Child {
    let selected = read_value(&value);
    Child::new(
        ChildKey::keyed(grid_key(year, month, selected)),
        LayoutRequest::new().width_pct(100.0),
        move || {
            // v1's `list(rows).style(|s| s.gap(2.0))`.
            let mut grid = Stack::new(Axis::Vertical).gap(2.0);
            for week in 0..6 {
                let mut row = Stack::new(Axis::Horizontal)
                    // v1's `.justify_between()`.
                    .align(Align::SpaceBetween, Align::Center);
                if show_week_numbers {
                    let (wy, wm, wd) = abs_date_of_cell(week * 7, year, month);
                    let number = week_number(wy, wm, wd).to_string();
                    let ink = style.on_surface_variant;
                    let size = style.weekday_font_size;
                    let width = style.weeknumber_width;
                    let height = style.cell_size;
                    row = row.push_layout(
                        LayoutRequest::new().width(width).height(height),
                        move || {
                            // The inner closure owns its copy: a replayed
                            // closure cannot move a value out of its capture.
                            let number = number.clone();
                            Box::new(
                                Stack::new(Axis::Horizontal)
                                    .align(Align::Center, Align::Center)
                                    .request(LayoutRequest::new().width(width).height(height))
                                    .push(move || {
                                        Box::new(Text::new(number.clone()).size(size).color(ink))
                                    }),
                            )
                        },
                    );
                }
                for column in 0..7 {
                    let k = week * 7 + column;
                    // v1 drew all 42 cells and hid the ones outside the
                    // month; a hidden cell took no room, so the port does not
                    // mount it at all -- the same picture by a shorter road.
                    if day_at(k, year, month).is_none() {
                        continue;
                    }
                    let cell = day_cell(k, year, month, value.clone(), on_change.clone(), style);
                    row = row.child(cell);
                }
                grid = grid.push(move || Box::new(row.clone()));
            }
            Box::new(grid)
        },
    )
}

/// One day: the number in a round box, and the click that picks it.
fn day_cell(
    k: usize,
    year: i32,
    month: u32,
    value: SharedDate,
    on_change: Option<Arc<dyn Fn(CalendarDate) + Send + Sync + 'static>>,
    style: CalendarStyle,
) -> Child {
    let day = day_at(k, year, month);
    let selected = match day {
        Some(n) => {
            read_value(&value)
                == CalendarDate {
                    year,
                    month,
                    day: n,
                }
        }
        None => false,
    };
    let ink = if selected {
        style.on_primary
    } else {
        style.on_surface
    };
    let fill = if selected {
        style.primary
    } else {
        Color::rgba(0, 0, 0, 0)
    };
    let label = day.map(|n| n.to_string()).unwrap_or_default();
    let cell_size = style.cell_size;
    let font_size = style.day_font_size;

    Child::new(
        ChildKey::index(k),
        LayoutRequest::new().width(cell_size).height(cell_size),
        move || {
            // The inner closure owns its copy: a replayed closure cannot move
            // a value out of its own capture.
            let label = label.clone();
            let cell = Surface::new()
                .request(
                    LayoutRequest::new()
                        .width(cell_size)
                        .height(cell_size)
                        // v1's `.items_center().justify_center()`.
                        .main_align(Align::Center)
                        .cross_align(Align::Center),
                )
                .background(fill)
                // v1's `.border_radius(cell_size / 2.0)`.
                .radius(cell_size / 2.0)
                .push(move || Box::new(Text::new(label.clone()).size(font_size).color(ink)));
            let value = value.clone();
            let on_change = on_change.clone();
            Box::new(
                Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || Box::new(cell.clone()),
                ))
                .on_click(move || {
                    if let Some(day) = day {
                        let date = CalendarDate { year, month, day };
                        write_value(&value, date);
                        if let Some(on_change) = &on_change {
                            on_change(date);
                        }
                    }
                }),
            )
        },
    )
}

/// A key that changes when the month does.
fn month_key(year: i32, month: u32) -> u64 {
    ((year as u64) << 32) | (month as u64)
}

/// A key that changes when the month or the selection does.
fn grid_key(year: i32, month: u32, value: CalendarDate) -> u64 {
    (month_key(year, month) << 16) | (value.day as u64)
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
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The calendar: the month grid, and the date it shows.
pub struct Calendar {
    /// The kit's answer.
    style: CalendarStyle,
    /// The chosen date.
    value: SharedDate,
    /// The month on show.
    view_month: SharedMonth,
    /// Called with the chosen date whenever it changes.
    on_change: Option<Arc<dyn Fn(CalendarDate) + Send + Sync + 'static>>,
    /// Whether the week's numbers are shown.
    show_week_numbers: bool,
}

impl Calendar {
    pub fn new() -> Self {
        Self {
            style: CalendarStyle::default(),
            value: Arc::new(Mutex::new(CalendarDate::default())),
            view_month: Arc::new(Mutex::new((2026, 1))),
            on_change: None,
            show_week_numbers: false,
        }
    }

    /// The calendar's appearance.
    pub fn style(mut self, style: CalendarStyle) -> Self {
        self.style = style;
        self
    }

    /// The cell the chosen date lives in.
    pub fn value(mut self, value: SharedDate) -> Self {
        self.value = value;
        self
    }

    /// The cell the month on show lives in.
    pub fn view_month(mut self, view_month: SharedMonth) -> Self {
        self.view_month = view_month;
        self
    }

    /// Called with the chosen date whenever it changes.
    pub fn on_change(mut self, callback: impl Fn(CalendarDate) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// Whether the week's numbers are shown.
    pub fn show_week_numbers(mut self, show_week_numbers: bool) -> Self {
        self.show_week_numbers = show_week_numbers;
        self
    }

    /// The calendar, ready to mount.
    pub fn build(self) -> Surface {
        let style = self.style;
        let fill = style.surface;
        let outline = with_alpha(style.outline, 120);
        let body = CalendarBody {
            value: self.value.clone(),
            view_month: self.view_month.clone(),
            on_change: self.on_change.clone(),
            style,
            show_week_numbers: self.show_week_numbers,
            last: None,
        };

        Surface::new()
            .request(LayoutRequest::new().width(style.panel_width))
            // v1's `.padding(10.0)`.
            .padding(Insets::uniform(10.0))
            .background(fill)
            .radius(style.border_radius)
            // v1's `.border(1.0)` and `.border_color(outline at 120)`.
            .border(1.0, outline)
            .push(move || Box::new(body.clone()))
    }
}
