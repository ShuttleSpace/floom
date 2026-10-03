use std::rc::Rc;

use floom::context::{LayoutChanged, LayoutChangedListener};
use floom::event::EventPropagation;
use floom::prelude::*;
use floom::style::CursorStyle;
use floom::views::{Decorators, Empty, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentSliderStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub thumb_color: Color,
    pub track_height: f32,
    pub thumb_size: f32,
    pub thumb_radius: f32,
}

pub struct FluentSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
    value: RwSignal<f32>,
    value_signal: Option<RwSignal<f32>>,
    width: RwSignal<f32>,
    dragging: RwSignal<bool>,
}

/// Map a local pointer x-coordinate (relative to the track's left edge) to a
/// stepped slider value, clamped to `[min, max]`.
fn value_from_local_x(
    local_x: f64,
    width: f32,
    min: f32,
    max: f32,
    step: f32,
    thumb_size: f32,
) -> f32 {
    if max <= min {
        return min;
    }
    let w = (width as f64).max(1.0);
    let radius = (thumb_size as f64) / 2.0;
    let clamped = local_x.clamp(radius, w - radius);
    let ratio = ((clamped - radius) / (w - 2.0 * radius)).clamp(0.0, 1.0);
    let raw = min as f64 + ratio * (max as f64 - min as f64);
    let step_f = step as f64;
    let steps = ((raw - min as f64) / step_f).round();
    let v = min as f64 + steps * step_f;
    (v as f32).clamp(min, max)
}

impl FluentSlider {
    pub fn new() -> Self {
        let props = SliderProps::default();
        let v = props.value.clamp(props.min, props.max);
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            props,
            value: RwSignal::new(v),
            value_signal: None,
            width: RwSignal::new(200.0),
            dragging: RwSignal::new(false),
        }
    }
    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self.value.set(value.clamp(self.props.min, self.props.max));
        self
    }
    pub fn min(mut self, min: f32) -> Self {
        self.props.min = min;
        self
    }
    pub fn max(mut self, max: f32) -> Self {
        self.props.max = max;
        self
    }
    pub fn step(mut self, step: f32) -> Self {
        self.props.step = step;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn value_signal(mut self, value: RwSignal<f32>) -> Self {
        self.value_signal = Some(value);
        self
    }
    pub fn props(&self) -> &SliderProps {
        &self.props
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentSliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tk) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        FluentSliderStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            thumb_color: theme.colors().primary,
            track_height: tk.track_height,
            thumb_size: tk.thumb_size,
            thumb_radius: tk.thumb_radius,
        }
    }

    /// Build a renderable, interactive Fluent slider.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value_signal.unwrap_or(self.value);
        let width = self.width;
        let dragging = self.dragging;
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let disabled = self.props.disabled;
        let on_change = Rc::new(self.props.on_change);
        let on_change_down = on_change.clone();
        let on_change_move = on_change.clone();

        let track_line = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Slider(tk) = t.component(ComponentId::Slider) else {
                unreachable!()
            };
            s.width_full()
                .height(tk.track_height)
                .background(t.colors().surface_variant.to_peniko())
                .border_radius(t.radius().sm)
        });

        // Filled (selected) track segment — primary, from the left edge to the
        // current value, matching md/cupertino sliders and fluent-range_slider.
        let fill = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Slider(tk) = t.component(ComponentId::Slider) else {
                unreachable!()
            };
            let ratio = if max > min {
                (value.get() - min) / (max - min)
            } else {
                0.0
            };
            s.absolute()
                .inset_top(0.0)
                .inset_left(0.0)
                .height(tk.track_height)
                .width_pct((ratio * 100.0) as f64)
                .background(t.colors().primary.to_peniko())
                .border_radius(t.radius().sm)
                .apply_if(disabled, |s| s.set_disabled(true))
        });

        let thumb = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Slider(tk) = t.component(ComponentId::Slider) else {
                unreachable!()
            };
            let ratio = if max > min {
                (value.get() - min) / (max - min)
            } else {
                0.0
            };
            let w = width.get().max(1.0);
            let offset = ratio * (w - tk.thumb_size);
            s.absolute()
                .inset_top(0.0)
                .inset_left(offset)
                .width(tk.thumb_size)
                .height(tk.thumb_size)
                .background(t.colors().primary.to_peniko())
                .border_radius(tk.thumb_radius)
                .apply_if(disabled, |s| s.set_disabled(true))
        });

        Stack::horizontal((track_line, fill, thumb))
            .style(move |s| {
                let ComponentTokens::Slider(tk) = theme.get().component(ComponentId::Slider) else {
                    unreachable!()
                };
                s.width_full()
                    .height(tk.thumb_size)
                    .items_center()
                    .cursor(if disabled {
                        CursorStyle::Default
                    } else {
                        CursorStyle::Pointer
                    })
                    .apply_if(disabled, |s| s.set_disabled(true))
            })
            .on_event(LayoutChangedListener, move |_cx, layout: &LayoutChanged| {
                width.set(layout.new_box.size().width as f32);
                EventPropagation::Continue
            })
            .on_event(listener::PointerDown, move |cx, e| {
                if disabled {
                    return EventPropagation::Continue;
                }
                if let Some(pid) = e.pointer.pointer_id {
                    cx.request_pointer_capture(pid);
                }
                let ComponentTokens::Slider(tk) =
                    theme.get().component(ComponentId::Slider) else {
                        unreachable!()
                    };
                let v = value_from_local_x(
                    e.state.logical_point().x,
                    width.get(),
                    min,
                    max,
                    step,
                    tk.thumb_size,
                );
                value.set(v);
                if let Some(cb) = on_change_down.as_ref() {
                    cb(v);
                }
                dragging.set(true);
                EventPropagation::Continue
            })
            .on_event(listener::PointerMove, move |_cx, e| {
                if disabled || !dragging.get() {
                    return EventPropagation::Continue;
                }
                let ComponentTokens::Slider(tk) =
                    theme.get().component(ComponentId::Slider) else {
                        unreachable!()
                    };
                let v = value_from_local_x(
                    e.current.logical_point().x,
                    width.get(),
                    min,
                    max,
                    step,
                    tk.thumb_size,
                );
                value.set(v);
                if let Some(cb) = on_change_move.as_ref() {
                    cb(v);
                }
                EventPropagation::Continue
            })
            .on_event(listener::PointerUp, move |_cx, _e| {
                dragging.set(false);
                EventPropagation::Continue
            })
    }
}

impl Default for FluentSlider {
    fn default() -> Self {
        Self::new()
    }
}
