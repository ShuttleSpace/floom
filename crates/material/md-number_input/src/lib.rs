use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate, create_effect};
use floem::style::CursorStyle;
use floem::views::{Container, Decorators, Empty, TextInput, v_stack, h_stack, text, Stack, Label};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::number_input::NumberInputProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

fn clamp_value(v: i64, min: Option<i64>, max: Option<i64>) -> i64 {
    let mut r = v;
    if let Some(m) = min {
        r = r.max(m);
    }
    if let Some(m) = max {
        r = r.min(m);
    }
    r
}

/// Material 3 numeric input field with stepper buttons (Vuetify `VNumberInput`).
pub struct MdNumberInput {
    props: NumberInputProps,
    theme: RwSignal<MultiTheme>,
}

impl MdNumberInput {
    pub fn new() -> Self {
        Self {
            props: NumberInputProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn value(mut self, v: i64) -> Self {
        self.props.value = v;
        self
    }

    /// Inclusive lower bound (clamped). Wrap in `Some`.
    pub fn min(mut self, m: i64) -> Self {
        self.props.min = Some(m);
        self
    }

    /// Inclusive upper bound (clamped). Wrap in `Some`.
    pub fn max(mut self, m: i64) -> Self {
        self.props.max = Some(m);
        self
    }

    pub fn step(mut self, s: i64) -> Self {
        self.props.step = s;
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.props.label = Some(l.into());
        self
    }

    pub fn disabled(mut self, d: bool) -> Self {
        self.props.disabled = d;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(i64) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let disabled = self.props.disabled;
        let label = self.props.label.clone();
        let on_change = Rc::new(self.props.on_change);

        let value: RwSignal<i64> = RwSignal::new(clamp_value(self.props.value, min, max));
        let buf: RwSignal<String> = RwSignal::new(value.get().to_string());

        // Typed text -> numeric value (clamped), firing on_change.
        create_effect({
            let value = value;
            let on_change = on_change.clone();
            move |_| {
                if let Ok(p) = buf.get().trim().parse::<i64>() {
                    let c = clamp_value(p, min, max);
                    if c != value.get() {
                        value.set(c);
                        if let Some(cb) = on_change.as_ref() {
                            cb(c);
                        }
                    }
                }
            }
        });

        let tokens = match theme.get().component(ComponentId::NumberInput) {
            ComponentTokens::NumberInput(t) => t,
            _ => unreachable!(),
        };
        let tk = tokens;

        let dec = {
            let value = value;
            let buf = buf;
            let on_change = on_change.clone();
            Container::new(Label::new("-"))
                .style(move |s| {
                    let t = theme.get();
                    s.size(tk.button_size, tk.button_size)
                        .border_radius(tk.radius)
                        .background(t.colors().surface_variant.to_peniko())
                        .color(t.colors().on_surface.to_peniko())
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::Pointer)
                })
                .on_event(listener::Click, move |_, _| {
                    let nv = clamp_value(value.get() - step, min, max);
                    value.set(nv);
                    buf.set(nv.to_string());
                    if let Some(cb) = on_change.as_ref() {
                        cb(nv);
                    }
                    EventPropagation::Continue
                })
        };

        let inc = {
            let value = value;
            let buf = buf;
            let on_change = on_change.clone();
            Container::new(Label::new("+"))
                .style(move |s| {
                    let t = theme.get();
                    s.size(tk.button_size, tk.button_size)
                        .border_radius(tk.radius)
                        .background(t.colors().surface_variant.to_peniko())
                        .color(t.colors().on_surface.to_peniko())
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::Pointer)
                })
                .on_event(listener::Click, move |_, _| {
                    let nv = clamp_value(value.get() + step, min, max);
                    value.set(nv);
                    buf.set(nv.to_string());
                    if let Some(cb) = on_change.as_ref() {
                        cb(nv);
                    }
                    EventPropagation::Continue
                })
        };

        let input = TextInput::new(buf)
            .placeholder(&label.clone().unwrap_or_default())
            .style(move |s| {
                let t = theme.get();
                let c = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().on_surface
                };
                s.color(c.to_peniko())
                    .font_size(tk.font_size)
                    .width_pct(100.0)
            })
            .on_event(listener::FocusGained, move |_, _| EventPropagation::Continue)
            .on_event(listener::FocusLost, move |_, _| EventPropagation::Continue)
            .style(move |s| s.set_disabled(disabled));

        let field = Container::new(Stack::horizontal((dec, input, inc)))
            .style(move |s| {
                let t = theme.get();
                let border = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().outline
                };
                let bg = if disabled {
                    with_alpha(t.colors().on_surface, 4)
                } else {
                    t.colors().surface
                };
                s.height(tk.height)
                    .width(tk.width)
                    .padding_horiz(tk.padding_horizontal)
                    .border(tk.border)
                    .border_color(border.to_peniko())
                    .border_radius(tk.radius)
                    .background(bg.to_peniko())
                    .items_center()
                    .cursor(CursorStyle::Text)
            })
            .style(move |s| s.set_disabled(disabled));

        match label {
            Some(l) => Stack::vertical((
                Label::new(l).style(move |s| {
                    let t = theme.get();
                    s.font_size(12.0).color(t.colors().on_surface_variant.to_peniko())
                }),
                field,
            ))
            .style(|s| s.gap(4.0))
            .into_any(),
            None => field.into_any(),
        }
    }
}

impl Default for MdNumberInput {
    fn default() -> Self {
        Self::new()
    }
}
