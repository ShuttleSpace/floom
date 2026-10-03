use std::rc::Rc;

use floom::context::EventCx;
use floom::event::EventPropagation;
use floom::style::CursorStyle;
use floom::style::Length;
use floom::views::{Decorators, Empty, Stack};
use floom::{
    prelude::*,
    reactive::RwSignal,
};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoSliderStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub thumb_color: Color,
    pub track_height: f32,
    pub thumb_size: f32,
    pub thumb_radius: f32,
}

pub struct CupertinoSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
    value: RwSignal<f32>,
    value_signal: Option<RwSignal<f32>>,
}

impl CupertinoSlider {
    pub fn new() -> Self {
        let props = SliderProps::default();
        let value = props.value;
        Self {
            props,
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            value: RwSignal::new(value),
            value_signal: None,
        }
    }
    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self.value.set(value);
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
    pub fn resolved_style(&self) -> CupertinoSliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        CupertinoSliderStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            thumb_color: theme.colors().primary,
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,
        }
    }

    /// Build a renderable, interactive Cupertino slider.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value_signal.unwrap_or(self.value);
        let on_change = Rc::new(self.props.on_change);
        let disabled = self.props.disabled;
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let dragging = RwSignal::new(false);

        let tokens = match theme.get().component(ComponentId::Slider) {
            ComponentTokens::Slider(t) => t,
            _ => unreachable!(),
        };
        let track_height = tokens.track_height;
        let thumb_size = tokens.thumb_size;
        let thumb_radius = tokens.thumb_radius;

        let ratio_of = move |v: f32| -> f32 {
            if max > min {
                ((v - min) / (max - min)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        };

        let on_change_down = on_change.clone();
        let on_change_move = on_change.clone();

        let fill = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ratio = ratio_of(value.get());
            s.absolute()
                .pointer_events_none()
                .inset_left(0.0)
                .inset_top(0.0)
                .height(track_height)
                .width_pct((ratio * 100.0) as f64)
                .background(t.colors().primary.to_peniko())
                .border_radius(track_height / 2.0)
        });

        let thumb = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ratio = ratio_of(value.get());
            s.absolute()
                .pointer_events_none()
                .size(thumb_size, thumb_size)
                .inset_top((track_height - thumb_size) / 2.0)
                .inset_left_pct((ratio * 100.0) as f64)
                .translate_x(Length::Pt(-(thumb_size as f64 / 2.0)))
                .background(t.colors().surface.to_peniko())
                .border_radius(thumb_radius)
                .box_shadow_blur(3.0)
                .box_shadow_spread(0.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 60).to_peniko())
                .apply_if(disabled, |s| s.set_disabled(true))
        });

        let track = Stack::vertical((fill, thumb))
            .style(move |s| {
                let t = theme.get();
                s.width_pct(100.0)
                    .height(track_height)
                    .background(t.colors().surface_variant.to_peniko())
                    .border_radius(track_height / 2.0)
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::PointerDown, move |cx, data| {
                if disabled {
                    return EventPropagation::Continue;
                }
                if let Some(pid) = data.pointer.pointer_id {
                    cx.request_pointer_capture(pid);
                }
                apply_pointer(cx, data.state.logical_point().x, value, &on_change_down, min, max, step);
                dragging.set(true);
                EventPropagation::Continue
            })
            .on_event(listener::PointerMove, move |cx, data| {
                if disabled || !dragging.get() {
                    return EventPropagation::Continue;
                }
                apply_pointer(
                    cx,
                    data.current.logical_point().x,
                    value,
                    &on_change_move,
                    min,
                    max,
                    step,
                );
                EventPropagation::Continue
            })
            .on_event(listener::PointerLeave, move |_, _| {
                dragging.set(false);
                EventPropagation::Continue
            });

        track
    }
}

/// Snap `raw` to the nearest `step` and clamp into `[min, max]`.
fn snap(raw: f32, min: f32, max: f32, step: f32) -> f32 {
    let val = if step <= 0.0 {
        raw
    } else {
        let steps = ((raw - min) / step).round();
        min + steps * step
    };
    val.clamp(min, max)
}

/// Map a pointer's local x position on the track to a stepped value.
fn apply_pointer(
    cx: &EventCx,
    x: f64,
    value: RwSignal<f32>,
    on_change: &Rc<Option<Box<dyn Fn(f32)>>>,
    min: f32,
    max: f32,
    step: f32,
) {
    let width = cx.target.owning_id().get_size().map(|s| s.width).unwrap_or(0.0);
    let ratio = if width > 0.0 {
        (x / width).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let raw = min + ratio as f32 * (max - min);
    let stepped = snap(raw, min, max, step);
    value.set(stepped);
    if let Some(cb) = on_change.as_ref() {
        cb(stepped);
    }
}

impl Default for CupertinoSlider {
    fn default() -> Self {
        Self::new()
    }
}
