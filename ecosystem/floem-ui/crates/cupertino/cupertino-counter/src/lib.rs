use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::views::{Container, Decorators, Stack, Label};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::counter::CounterProps;
use floem_core_theme::theme::{MultiTheme, Theme};

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

pub struct CupertinoCounter {
    props: CounterProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoCounter {
    pub fn new() -> Self {
        Self {
            props: CounterProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn value(mut self, v: i64) -> Self {
        self.props.value = v;
        self
    }

    pub fn min(mut self, m: i64) -> Self {
        self.props.min = Some(m);
        self
    }

    pub fn max(mut self, m: i64) -> Self {
        self.props.max = Some(m);
        self
    }

    pub fn step(mut self, s: i64) -> Self {
        self.props.step = s;
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
        let on_change = Rc::new(self.props.on_change);
        let value: RwSignal<i64> = RwSignal::new(clamp_value(self.props.value, min, max));

        let tokens = match theme.get().component(ComponentId::Counter) {
            ComponentTokens::Counter(t) => t,
            _ => unreachable!(),
        };
        let tk_dec = tokens.clone();
        let tk_inc = tokens.clone();
        let tk_out = tokens;

        let dec = Container::new(Label::new("-"))
            .style(move |s| {
                let t = theme.get();
                s.size(tk_dec.button_size, tk_dec.button_size)
                    .border_radius(tk_dec.radius)
                    .background(t.colors().surface_variant.to_peniko())
                    .color(t.colors().on_surface.to_peniko())
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, {
                let value = value;
                let on_change = on_change.clone();
                move |_, _| {
                    let nv = clamp_value(value.get() - step, min, max);
                    value.set(nv);
                    if let Some(cb) = on_change.as_ref() {
                        cb(nv);
                    }
                    EventPropagation::Continue
                }
            });

        let val = Label::derived(move || value.get().to_string()).style(move |s| {
            let t = theme.get();
            s.font_size(16.0).color(t.colors().on_surface.to_peniko())
        });

        let inc = Container::new(Label::new("+"))
            .style(move |s| {
                let t = theme.get();
                s.size(tk_inc.button_size, tk_inc.button_size)
                    .border_radius(tk_inc.radius)
                    .background(t.colors().surface_variant.to_peniko())
                    .color(t.colors().on_surface.to_peniko())
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, {
                let value = value;
                let on_change = on_change.clone();
                move |_, _| {
                    let nv = clamp_value(value.get() + step, min, max);
                    value.set(nv);
                    if let Some(cb) = on_change.as_ref() {
                        cb(nv);
                    }
                    EventPropagation::Continue
                }
            });

        Stack::horizontal((dec, val, inc)).style(move |s| {
            let t = theme.get();
            s.height(tk_out.height)
                .width(tk_out.width)
                .gap(tk_out.gap)
                .items_center()
                .justify_center()
                .border(tk_out.border)
                .border_color(t.colors().outline.to_peniko())
                .border_radius(tk_out.radius)
        })
    }
}

impl Default for CupertinoCounter {
    fn default() -> Self {
        Self::new()
    }
}
