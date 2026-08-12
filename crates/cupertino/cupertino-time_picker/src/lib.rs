use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::time_picker::{TimePickerProps, TimeValue};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTimePickerStyle {
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
}

fn set_time(
    value: RwSignal<Option<TimeValue>>,
    on_change: &Rc<Option<Box<dyn Fn(TimeValue)>>>,
    hour: u8,
    minute: u8,
) {
    let nv = TimeValue { hour, minute };
    value.set(Some(nv));
    if let Some(cb) = on_change.as_ref() {
        cb(nv);
    }
}

pub struct CupertinoTimePicker {
    props: TimePickerProps,
    value: RwSignal<Option<TimeValue>>,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTimePicker {
    pub fn new() -> Self {
        let props = TimePickerProps::default();
        Self {
            value: RwSignal::new(props.value),
            open: RwSignal::new(false),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
        }
    }

    pub fn value(mut self, value: TimeValue) -> Self {
        self.props.value = Some(value);
        self.value.set(Some(value));
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

    pub fn on_change(mut self, callback: impl Fn(TimeValue) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> CupertinoTimePickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::TimePicker(tokens) = theme.component(ComponentId::TimePicker) else {
            unreachable!("ComponentId::TimePicker must resolve to TimePickerTokens")
        };
        CupertinoTimePickerStyle {
            design_system: "cupertino",
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
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let value = self.value;
        let open = self.open;
        let on_change: Rc<Option<Box<dyn Fn(TimeValue)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::TimePicker) {
            ComponentTokens::TimePicker(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let radius = tokens.border_radius;
        let min_width = tokens.min_width;
        let panel_width = tokens.panel_width;
        let cell_height = tokens.cell_height;

        let stepper_btn = |label_text: &'static str, theme: RwSignal<MultiTheme>| {
            text(label_text).style(move |s| {
                let t = theme.get();
                s.width(32.0)
                    .height(32.0)
                    .items_center()
                    .justify_center()
                    .color(t.colors().on_surface.to_peniko())
                    .background(with_alpha(t.colors().on_surface, 8).to_peniko())
                    .border_radius(16.0)
                    .font_size(18.0)
                    .cursor(CursorStyle::Pointer)
            })
        };

        let hour_dec = stepper_btn("−", theme)
            .on_event(listener::Click, {
                let value = value;
                let on_change = on_change.clone();
                move |_, _| {
                    let cur = value.get().unwrap_or(TimeValue { hour: 0, minute: 0 });
                    let h = if cur.hour == 0 { 23 } else { cur.hour - 1 };
                    set_time(value, &on_change, h, cur.minute);
                    EventPropagation::Continue
                }
            });
        let hour_inc = stepper_btn("+", theme)
            .on_event(listener::Click, {
                let value = value;
                let on_change = on_change.clone();
                move |_, _| {
                    let cur = value.get().unwrap_or(TimeValue { hour: 0, minute: 0 });
                    let h = (cur.hour + 1) % 24;
                    set_time(value, &on_change, h, cur.minute);
                    EventPropagation::Continue
                }
            });
        let minute_dec = stepper_btn("−", theme)
            .on_event(listener::Click, {
                let value = value;
                let on_change = on_change.clone();
                move |_, _| {
                    let cur = value.get().unwrap_or(TimeValue { hour: 0, minute: 0 });
                    let m = if cur.minute == 0 { 59 } else { cur.minute - 1 };
                    set_time(value, &on_change, cur.hour, m);
                    EventPropagation::Continue
                }
            });
        let minute_inc = stepper_btn("+", theme)
            .on_event(listener::Click, {
                let value = value;
                let on_change = on_change.clone();
                move |_, _| {
                    let cur = value.get().unwrap_or(TimeValue { hour: 0, minute: 0 });
                    let m = (cur.minute + 1) % 60;
                    set_time(value, &on_change, cur.hour, m);
                    EventPropagation::Continue
                }
            });

        let hour_val = label(move || {
            format!("{:02}", value.get().map(|t| t.hour).unwrap_or(0))
        })
        .style(move |s| {
            let t = theme.get();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(24.0)
                .font_weight(FontWeight::Medium.to_floem())
                .width(48.0)
                .items_center()
                .justify_center()
        });
        let minute_val = label(move || {
            format!("{:02}", value.get().map(|t| t.minute).unwrap_or(0))
        })
        .style(move |s| {
            let t = theme.get();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(24.0)
                .font_weight(FontWeight::Medium.to_floem())
                .width(48.0)
                .items_center()
                .justify_center()
        });

        let hour_row = h_stack((hour_dec, hour_val, hour_inc))
            .style(move |s| s.items_center().justify_center().gap(8.0).height(cell_height));
        let minute_row = h_stack((minute_dec, minute_val, minute_inc))
            .style(move |s| s.items_center().justify_center().gap(8.0).height(cell_height));

        let done = text("Done")
            .style(move |s| {
                let t = theme.get();
                s.padding_horiz(12.0)
                    .height(32.0)
                    .items_center()
                    .justify_center()
                    .color(t.colors().primary.to_peniko())
                    .font_size(t.typography().body.medium.font_size)
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                open.set(false);
                EventPropagation::Continue
            });

        let panel = Container::new(
            v_stack((hour_row, minute_row, done))
                .style(|s| s.items_center().gap(8.0)),
        )
        .style(move |s| {
            let t = theme.get();
            s.absolute()
                .inset_top_pct(100.0)
                .inset_left_pct(0.0)
                .width(panel_width)
                .margin_top(4.0)
                .padding(12.0)
                .background(t.colors().surface.to_peniko())
                .border_radius(radius)
                .border(1.0)
                .border_color(with_alpha(t.colors().outline, 120).to_peniko())
                .box_shadow_blur(12.0)
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 24).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        });

        let trigger = Container::new(
            label(move || match value.get() {
                Some(tv) => format!("{:02}:{:02}", tv.hour, tv.minute),
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
                open.set(!open.get());
            }
            EventPropagation::Continue
        });

        Stack::new((trigger, panel))
    }
}

impl Default for CupertinoTimePicker {
    fn default() -> Self {
        Self::new()
    }
}
