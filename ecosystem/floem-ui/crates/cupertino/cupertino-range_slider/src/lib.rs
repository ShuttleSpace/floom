use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::view::AnyView;
use floem::style::CursorStyle;
use floem::views::{Decorators, Empty};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::range_slider::RangeSliderProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color_ext::*;

/// Material 3 range slider with two draggable thumbs selecting a (low, high)
/// interval. The track handles all pointer interaction; thumbs are visual.
pub struct CupertinoRangeSlider {
    props: RangeSliderProps,
    theme: RwSignal<MultiTheme>,
    low: RwSignal<f32>,
    high: RwSignal<f32>,
}

impl CupertinoRangeSlider {
    pub fn new() -> Self {
        let props = RangeSliderProps::default();
        let min = props.min;
        let max = props.max;
        Self {
            low: RwSignal::new(props.low.clamp(min, max)),
            high: RwSignal::new(props.high.clamp(min, max)),
            props,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn low(mut self, low: f32) -> Self {
        self.props.low = low;
        self.low = RwSignal::new(low.clamp(self.props.min, self.props.max));
        self
    }

    pub fn high(mut self, high: f32) -> Self {
        self.props.high = high;
        self.high = RwSignal::new(high.clamp(self.props.min, self.props.max));
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

    pub fn on_change(mut self, callback: impl Fn((f32, f32)) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &RangeSliderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn low_signal(&self) -> RwSignal<f32> {
        self.low
    }

    pub fn high_signal(&self) -> RwSignal<f32> {
        self.high
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let on_change: Option<Rc<Box<dyn Fn((f32, f32))>>> = self.props.on_change.map(Rc::new);
        let disabled = RwSignal::new(self.props.disabled);
        let low = self.low;
        let high = self.high;
        let dragging: RwSignal<Option<usize>> = RwSignal::new(None);

        let tokens = match theme.get().component(ComponentId::RangeSlider) {
            ComponentTokens::RangeSlider(t) => t,
            _ => unreachable!(),
        };
        let track_height = tokens.track_height;
        let thumb_size = tokens.thumb_size;

        let update = Rc::new({
            let low = low;
            let high = high;
            let on_change = on_change.clone();
            move |which: usize, ratio: f32| {
                let span = (max - min).max(f32::MIN_POSITIVE);
                let raw = min + ratio * span;
                let steps = ((raw - min) / step).round();
                let mut v = (min + steps * step).clamp(min, max);
                match which {
                    0 => {
                        let hi = high.get();
                        if v > hi {
                            v = hi;
                        }
                        low.set(v);
                    }
                    _ => {
                        let lo = low.get();
                        if v < lo {
                            v = lo;
                        }
                        high.set(v);
                    }
                }
                if let Some(cb) = on_change.as_ref() {
                    cb((low.get(), high.get()));
                }
            }
        });

        let ratio_of = move |v: f32| -> f64 {
            let span = (max - min).max(f32::MIN_POSITIVE);
            ((v - min) / span).clamp(0.0, 1.0) as f64
        };

        let filled: AnyView = Container::new(Empty::new())
            .style(move |s| {
                let t = theme.get();
                let lo_r = ratio_of(low.get());
                let hi_r = ratio_of(high.get());
                let c = if disabled.get() {
                    with_alpha(t.colors().primary, 120)
                } else {
                    t.colors().primary
                };
                s.absolute()
                    .inset_left_pct(lo_r * 100.0)
                    .width_pct((hi_r - lo_r) * 100.0)
                    .height_pct(100.0)
                    .background(c.to_peniko())
            })
            .into_any();

        let thumb_low: AnyView = Container::new(Empty::new())
            .style(move |s| {
                let t = theme.get();
                let r = ratio_of(low.get());
                let c = if disabled.get() {
                    with_alpha(t.colors().primary, 120)
                } else {
                    t.colors().primary
                };
                s.absolute()
                    .size(thumb_size, thumb_size)
                    .inset_left_pct(r * 100.0)
                    .translate_x(-(thumb_size as f64 / 2.0))
                    .border_radius(tokens.thumb_radius)
                    .background(c.to_peniko())
            })
            .into_any();

        let thumb_high: AnyView = Container::new(Empty::new())
            .style(move |s| {
                let t = theme.get();
                let r = ratio_of(high.get());
                let c = if disabled.get() {
                    with_alpha(t.colors().primary, 120)
                } else {
                    t.colors().primary
                };
                s.absolute()
                    .size(thumb_size, thumb_size)
                    .inset_left_pct(r * 100.0)
                    .translate_x(-(thumb_size as f64 / 2.0))
                    .border_radius(tokens.thumb_radius)
                    .background(c.to_peniko())
            })
            .into_any();

        let mut track = list(vec![filled, thumb_low, thumb_high]).style(move |s| {
            let t = theme.get();
            let c = if disabled.get() {
                with_alpha(t.colors().surface_variant, 120)
            } else {
                t.colors().surface_variant
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
                let low = low;
                let high = high;
                let disabled = disabled;
                move |cx, pbe| {
                    if !disabled.get() {
                        if let Some(sz) = track_id.get_size() {
                            let w = sz.width as f32;
                            if w > 0.0 {
                                let p = pbe.state.logical_point();
                                let ratio = ((p.x as f32) / w).clamp(0.0, 1.0);
                                let lo_r = ratio_of(low.get());
                                let hi_r = ratio_of(high.get());
                                let which =
                                    if ((ratio as f64) - lo_r).abs() <= ((ratio as f64) - hi_r).abs() { 0 } else { 1 };
                                dragging.set(Some(which));
                                update(which, ratio);
                            }
                        }
                        if let Some(pid) = pbe.pointer.pointer_id {
                            cx.request_pointer_capture(pid);
                        }
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
                    if !disabled.get() {
                        if let Some(which) = dragging.get() {
                            if let Some(sz) = track_id.get_size() {
                                let w = sz.width as f32;
                                if w > 0.0 {
                                    let p = pu.current.logical_point();
                                    let ratio = ((p.x as f32) / w).clamp(0.0, 1.0);
                                    update(which, ratio);
                                }
                            }
                        }
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerUp, {
                let dragging = dragging;
                move |_, _| {
                    dragging.set(None);
                    EventPropagation::Continue
                }
            });

        track
    }
}

impl Default for CupertinoRangeSlider {
    fn default() -> Self {
        Self::new()
    }
}
