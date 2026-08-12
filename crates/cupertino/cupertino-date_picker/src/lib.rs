use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::h_stack_from_iter;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::date_picker::{DatePickerProps, DateValue};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoDatePickerStyle {
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

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December",
];

pub struct CupertinoDatePicker {
    props: DatePickerProps,
    value: RwSignal<Option<DateValue>>,
    open: RwSignal<bool>,
    view_month: RwSignal<(i32, u32)>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoDatePicker {
    pub fn new() -> Self {
        let props = DatePickerProps::default();
        let vm = props
            .value
            .map(|d| (d.year, d.month))
            .unwrap_or((2026, 8));
        Self {
            value: RwSignal::new(props.value),
            open: RwSignal::new(false),
            view_month: RwSignal::new(vm),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
        }
    }

    pub fn value(mut self, value: DateValue) -> Self {
        self.props.value = Some(value);
        self.value.set(Some(value));
        self.view_month.set((value.year, value.month));
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

    pub fn on_change(mut self, callback: impl Fn(DateValue) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> CupertinoDatePickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::DatePicker(tokens) = theme.component(ComponentId::DatePicker) else {
            unreachable!("ComponentId::DatePicker must resolve to DatePickerTokens")
        };
        CupertinoDatePickerStyle {
            design_system: "cupertino",
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
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let value = self.value;
        let open = self.open;
        let view_month = self.view_month;
        let on_change: Rc<Option<Box<dyn Fn(DateValue)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::DatePicker) {
            ComponentTokens::DatePicker(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let radius = tokens.border_radius;
        let min_width = tokens.min_width;
        let panel_width = tokens.panel_width;
        let header_font = tokens.header_font_size;
        let cell_size = tokens.cell_size;

        let trigger = Container::new(
            label(move || match value.get() {
                Some(d) => format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
                None => placeholder.clone(),
            })
            .style(move |s| {
                let t = theme.get();
                s.width_pct(100.0)
                    .color(
                        if value.get().is_some() {
                            t.colors().on_surface
                        } else {
                            t.colors().on_surface_variant
                        }
                        .to_peniko(),
                    )
                    .font_size(t.typography().body.medium.font_size)
            }),
        )
        .style(move |s| {
            let t = theme.get();
            let border = if disabled {
                with_alpha(t.colors().outline, 97)
            } else if open.get() {
                t.colors().primary
            } else {
                t.colors().outline
            };
            s.height(height)
                .min_width(min_width)
                .padding_horiz(12.0)
                .border(1.0)
                .border_color(border.to_peniko())
                .border_radius(radius)
                .background(t.colors().surface.to_peniko())
                .items_center()
                .cursor(CursorStyle::Pointer)
                .set_disabled(disabled)
        })
        .on_event(listener::Click, move |_, _| {
            if !disabled {
                if open.get() {
                    open.set(false);
                } else {
                    if let Some(d) = value.get() {
                        view_month.set((d.year, d.month));
                    }
                    open.set(true);
                }
            }
            EventPropagation::Continue
        });

        let prev_btn = text("‹")
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

        let next_btn = text("›")
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

        let title = label(move || {
            let (y, m) = view_month.get();
            format!("{} {}", MONTHS[(m as usize) - 1], y)
        })
        .style(move |s| {
            let t = theme.get();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(header_font)
                .font_weight(FontWeight::Medium.to_floem())
        });

        let header = h_stack((prev_btn, title, next_btn))
            .style(|s| s.justify_between().items_center().width_pct(100.0));

        let weekday_row = h_stack((
            text("S"),
            text("M"),
            text("T"),
            text("W"),
            text("T"),
            text("F"),
            text("S"),
        ))
        .style(move |s| {
            s.justify_between()
                .width_pct(100.0)
                .color(theme.get().colors().on_surface_variant.to_peniko())
                .font_size(11.0)
        });

        // 6 weeks × 7 days of fixed cells; each reads `view_month` reactively so prev/next
        // navigation updates labels, highlight, and click target without rebuilding the tree.
        let rows: Vec<AnyView> = (0..6)
            .map(|r| {
                let week: Vec<AnyView> = (0..7)
                    .map(|c| {
                        let k = (r * 7 + c) as usize;
                        let value = value;
                        let view_month = view_month;
                        let open = open;
                        let on_change = on_change.clone();
                        let theme = theme;
                        label(move || {
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
                                    value.get() == Some(DateValue {
                                        year: yy,
                                        month: mm,
                                        day: n,
                                    })
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
                                .font_size(theme.get().typography().body.medium.font_size)
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
                                let dv = DateValue {
                                    year: yy,
                                    month: mm,
                                    day: n,
                                };
                                value.set(Some(dv));
                                open.set(false);
                                if let Some(cb) = on_change.as_ref() {
                                    cb(dv);
                                }
                            }
                            EventPropagation::Continue
                        })
                        .into_any()
                    })
                    .collect();
                h_stack_from_iter(week)
                    .style(|s| s.justify_between())
                    .into_any()
            })
            .collect();

        let grid = list(rows).style(|s| s.gap(2.0));

        let panel = Container::new(
            v_stack((header, weekday_row, grid)).style(|s| s.gap(6.0)),
        )
        .style(move |s| {
            let t = theme.get();
            s.absolute()
                .inset_top_pct(100.0)
                .inset_left_pct(0.0)
                .width(panel_width)
                .margin_top(4.0)
                .padding(8.0)
                .background(t.colors().surface.to_peniko())
                .border_radius(radius)
                .border(1.0)
                .border_color(with_alpha(t.colors().outline, 120).to_peniko())
                .box_shadow_blur(12.0)
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 24).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        });

        Stack::new((trigger, panel))
    }
}

impl Default for CupertinoDatePicker {
    fn default() -> Self {
        Self::new()
    }
}
