//! Fluent range slider, ported to the Floom 2.0 widget API.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::{
    CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext, Rect,
    UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, CursorStyle, PointerButton, PointerPhase};
use floom_ui_composites::{RangeSlider, RangeSliderStyle};
use floom_widgets::{shared_map, Revision, SharedMap};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::range_slider::RangeSliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const LOW_KEY: &str = "low";
const HIGH_KEY: &str = "high";

fn read_range(range: &SharedMap) -> (f32, f32) {
    let Ok(guard) = range.lock() else {
        return (0.0, 0.0);
    };
    let low = guard
        .get(LOW_KEY)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    let high = guard
        .get(HIGH_KEY)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    (low, high)
}

fn write_range(range: &SharedMap, low: f32, high: f32) {
    let Ok(mut guard) = range.lock() else {
        return;
    };
    guard.insert(LOW_KEY.to_string(), format!("{low}"));
    guard.insert(HIGH_KEY.to_string(), format!("{high}"));
}

fn fraction(value: f32, min: f32, max: f32) -> f32 {
    let span = (max - min).max(f32::MIN_POSITIVE);
    ((value - min) / span).clamp(0.0, 1.0)
}

fn ratio_at(bounds: Rect, x: f64) -> f32 {
    if bounds.width() <= 0.0 {
        return 0.0;
    }
    (((x - bounds.x0) / bounds.width()) as f32).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentRangeSliderStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub fill_color: Color,
    pub thumb_color: Color,
    pub track_height: f32,
    pub thumb_size: f32,
    pub thumb_radius: f32,
}

pub struct FluentRangeSlider {
    props: RangeSliderProps,
    theme: RwSignal<MultiTheme>,
    range: SharedMap,
    on_change: Option<Arc<dyn Fn((f32, f32)) + Send + Sync + 'static>>,
}

impl FluentRangeSlider {
    pub fn new() -> Self {
        let props = RangeSliderProps::default();
        let min = props.min;
        let max = props.max;
        let range = shared_map(HashMap::new());
        write_range(
            &range,
            props.low.clamp(min, max),
            props.high.clamp(min, max),
        );
        Self {
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            range,
            on_change: None,
        }
    }

    pub fn low(mut self, low: f32) -> Self {
        self.props.low = low;
        let clamped = low.clamp(self.props.min, self.props.max);
        let (_, high) = read_range(&self.range);
        write_range(&self.range, clamped, high);
        self
    }

    pub fn high(mut self, high: f32) -> Self {
        self.props.high = high;
        let clamped = high.clamp(self.props.min, self.props.max);
        let (low, _) = read_range(&self.range);
        write_range(&self.range, low, clamped);
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

    pub fn on_change(mut self, callback: impl Fn((f32, f32)) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &RangeSliderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn range(&self) -> SharedMap {
        self.range.clone()
    }

    pub fn low_value(&self) -> f32 {
        read_range(&self.range).0
    }

    pub fn high_value(&self) -> f32 {
        read_range(&self.range).1
    }

    pub fn resolved_style(&self) -> FluentRangeSliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::RangeSlider(tokens) = theme.component(ComponentId::RangeSlider) else {
            unreachable!("ComponentId::RangeSlider must resolve to RangeSliderTokens")
        };
        let colors = theme.colors();
        let (track, fill, thumb) = if self.props.disabled {
            (
                with_alpha(colors.surface_variant, 120),
                with_alpha(colors.primary, 120),
                with_alpha(colors.primary, 120),
            )
        } else {
            (colors.surface_variant, colors.primary, colors.primary)
        };
        FluentRangeSliderStyle {
            design_system: "fluent",
            track_color: track,
            fill_color: fill,
            thumb_color: thumb,
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,
        }
    }

    pub fn view(self) -> RangeSlider {
        let style = self.resolved_style();
        let mut slider = RangeSlider::new()
            .style(RangeSliderStyle {
                track_color: to_paint(style.track_color),
                fill_color: to_paint(style.fill_color),
                thumb_color: to_paint(style.thumb_color),
                track_height: style.track_height,
                thumb_size: style.thumb_size,
                thumb_radius: style.thumb_radius,
            })
            .bounds(self.props.min, self.props.max, self.props.step)
            .disabled(self.props.disabled)
            .values(self.range.clone());
        if let Some(on_change) = self.on_change.clone() {
            slider = slider.on_change(move |value| on_change(value));
        }
        slider
    }
}

impl Default for FluentRangeSlider {
    fn default() -> Self {
        Self::new()
    }
}
