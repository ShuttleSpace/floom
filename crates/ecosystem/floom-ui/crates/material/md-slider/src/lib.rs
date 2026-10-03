use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::view::AnyView;
use floom::style::CursorStyle;
use floom::views::{Decorators, Empty};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSliderStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub thumb_color: Color,
    pub track_height: f32,
    pub thumb_size: f32,
    pub thumb_radius: f32,
}

pub struct MdSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
}

impl MdSlider {
    pub fn new() -> Self {
        Self {
            props: SliderProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
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

    /// Bind the slider to an external signal. The slider reads its displayed
    /// value from `value` and writes back to it on every drag, so the host does
    /// not need to rebuild the slider to track changes (prevents the drag
    /// "flash-back" seen when the host rebuilds on every `value` change).
    pub fn value_signal(mut self, value: RwSignal<f32>) -> Self {
        self.props.value_signal = Some(value);
        self
    }

    pub fn props(&self) -> &SliderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdSliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        MdSliderStyle {
            design_system: "material",
            track_color: with_alpha(theme.colors().on_surface_variant, 97),
            thumb_color: theme.colors().primary,
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,
        }
    }

    /// Build a renderable, interactive Material 3 slider.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let on_change: Option<Rc<Box<dyn Fn(f32)>>> = self.props.on_change.map(Rc::new);
        let disabled = RwSignal::new(self.props.disabled);
        let live = self
            .props
            .value_signal
            .unwrap_or_else(|| RwSignal::new(self.props.value.clamp(min, max)));
        let dragging = RwSignal::new(false);
        let hover = RwSignal::new(false);

        let tokens = match theme.get().component(ComponentId::Slider) {
            ComponentTokens::Slider(t) => t,
            _ => unreachable!(),
        };
        let track_height = tokens.track_height;
        let thumb_size = tokens.thumb_size;

        // Shared value updater: maps a 0..=1 ratio to the nearest stepped value.
        let update = Rc::new({
            let live = live;
            let on_change = on_change.clone();
            move |ratio: f32| {
                let span = (max - min).max(f32::MIN_POSITIVE);
                let raw = min + ratio * span;
                let steps = ((raw - min) / step).round();
                let mut v = min + steps * step;
                v = v.clamp(min, max);
                live.set(v);
                if let Some(cb) = on_change.as_ref() {
                    cb(v);
                }
            }
        });

        let filled: AnyView = Container::new(Empty::new())
            .style(move |s| {
                let t = theme.get();
                let span = (max - min).max(f32::MIN_POSITIVE);
                let ratio: f64 = ((live.get() - min) / span).clamp(0.0, 1.0) as f64;
                let c = if disabled.get() {
                    with_alpha(t.colors().primary, 120)
                } else {
                    t.colors().primary
                };
                s.absolute()
                    .width_pct(ratio * 100.0)
                    .height_pct(100.0)
                    .background(c.to_peniko())
            })
            .into_any();

        let thumb: AnyView = Container::new(Empty::new())
            .style(move |s| {
                let t = theme.get();
                let span = (max - min).max(f32::MIN_POSITIVE);
                let ratio: f64 = ((live.get() - min) / span).clamp(0.0, 1.0) as f64;
                let c = if disabled.get() {
                    with_alpha(t.colors().primary, 120)
                } else {
                    t.colors().primary
                };
                s.absolute()
                    .size(thumb_size, thumb_size)
                    .inset_top_pct(50.0)
                    .inset_left_pct(ratio * 100.0)
                    .translate_x(-(thumb_size as f64 / 2.0))
                    .translate_y(-(thumb_size as f64 / 2.0))
                    .border_radius(tokens.thumb_radius)
                    .background(c.to_peniko())
            })
            .into_any();

        let halo: AnyView = Container::new(Empty::new())
            .style(move |s| {
                let t = theme.get();
                let span = (max - min).max(f32::MIN_POSITIVE);
                let ratio: f64 = ((live.get() - min) / span).clamp(0.0, 1.0) as f64;
                let active = hover.get() || dragging.get();
                let alpha = if dragging.get() { 30 } else { 20 };
                let dy = -((40.0 - track_height as f64) / 2.0);
                s.absolute()
                    .size(40.0, 40.0)
                    .inset_left_pct(ratio * 100.0)
                    .translate_x(-20.0)
                    .translate_y(dy)
                    .border_radius(9999.0)
                    .background(with_alpha(t.colors().primary, alpha).to_peniko())
                    .apply_if(!active, |s| s.hide())
            })
            .into_any();

        let mut track = list(vec![filled, halo, thumb]).style(move |s| {
            let t = theme.get();
            let c = if disabled.get() {
                with_alpha(t.colors().on_surface_variant, 60)
            } else {
                with_alpha(t.colors().on_surface_variant, 97)
            };
            s.width_full()
                .height(track_height)
                .border_radius(track_height / 2.0)
                .background(c.to_peniko())
                .cursor(CursorStyle::Pointer)
                .set_disabled(disabled.get())
        });

        let track_id = track.id();

        track = track
            .on_event(listener::PointerDown, {
                let update = update.clone();
                let track_id = track_id;
                let dragging = dragging;
                let disabled = disabled;
                move |cx, pbe| {
                    if !disabled.get() {
                        if let Some(sz) = track_id.get_size() {
                            let w = sz.width as f32;
                            if w > 0.0 {
                                let p = pbe.state.logical_point();
                                let ratio = ((p.x as f32) / w).clamp(0.0, 1.0);
                                update(ratio);
                            }
                        }
                        if let Some(pid) = pbe.pointer.pointer_id {
                            cx.request_pointer_capture(pid);
                        }
                        dragging.set(true);
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerMove, {
                let update = update.clone();
                let track_id = track_id;
                let dragging = dragging;
                let disabled = disabled;
                move |_, pu| {
                    if !disabled.get() && dragging.get() {
                        if let Some(sz) = track_id.get_size() {
                            let w = sz.width as f32;
                            if w > 0.0 {
                                let p = pu.current.logical_point();
                                let ratio = ((p.x as f32) / w).clamp(0.0, 1.0);
                                update(ratio);
                            }
                        }
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerUp, {
                let dragging = dragging;
                move |_, _| {
                    dragging.set(false);
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerEnter, move |_, _| {
                hover.set(true);
                EventPropagation::Continue
            })
            .on_event(listener::PointerLeave, move |_, _| {
                hover.set(false);
                EventPropagation::Continue
            });

        track
    }
}

impl Default for MdSlider {
    fn default() -> Self {
        Self::new()
    }
}
