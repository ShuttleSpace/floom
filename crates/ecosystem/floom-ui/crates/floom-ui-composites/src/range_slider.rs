
use std::collections::HashMap;
use std::sync::Arc;

use floom_core::{
    Child, Color, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    Point, Rect, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::{CursorStyle, PointerButton, PointerPhase};
use floom_widgets::primitives::{emit_semantics, Revision, SemanticsHint};
use floom_widgets::{shared_map, SharedMap};

/// The two values, in the shared map, under these keys.
const LOW_KEY: &str = "low";
const HIGH_KEY: &str = "high";

/// Reads the two ends of the range.
fn read_range(range: &SharedMap) -> (f32, f32) {
    let guard = range
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let low = guard
        .get(LOW_KEY)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let high = guard
        .get(HIGH_KEY)
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0);
    (low, high)
}

/// Writes the two ends of the range.
fn write_range(range: &SharedMap, low: f32, high: f32) {
    let mut guard = range
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.insert(LOW_KEY.to_string(), low.to_string());
    guard.insert(HIGH_KEY.to_string(), high.to_string());
}

/// The range slider's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeSliderStyle {
    /// The track's fill.
    pub track_color: Color,
    /// The band between the two thumbs.
    pub fill_color: Color,
    /// The two thumbs.
    pub thumb_color: Color,
    /// The track's thickness.
    pub track_height: f32,
    /// The thumb's diameter.
    pub thumb_size: f32,
    /// The thumb's corner radius. 9999 is a circle whatever the size, and the
    /// renderers clamp a radius to half the box.
    pub thumb_radius: f32,
}

impl Default for RangeSliderStyle {
    fn default() -> Self {
        Self {
            track_color: Color::rgb(0xE7, 0xE0, 0xEC),
            fill_color: Color::rgb(0x67, 0x50, 0xA4),
            thumb_color: Color::rgb(0x67, 0x50, 0xA4),
            track_height: 4.0,
            thumb_size: 16.0,
            thumb_radius: 9999.0,
        }
    }
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

impl RangeSlider {
    pub fn new() -> Self {
        Self {
            style: RangeSliderStyle::default(),
            min: 0.0,
            max: 1.0,
            step: 0.0,
            disabled: false,
            range: shared_map(HashMap::new()),
            on_change: None,
            dragging: None,
            pressed: false,
            revision: Revision::default(),
        }
    }

    /// The range slider's appearance.
    pub fn style(mut self, style: RangeSliderStyle) -> Self {
        self.style = style;
        self
    }

    /// The values the slider runs between, and the step they quantise to.
    ///
    /// A step of zero means no quantisation, which is what v1 did with a step
    /// of zero.
    pub fn bounds(mut self, min: f32, max: f32, step: f32) -> Self {
        self.min = min;
        self.max = max;
        self.step = step;
        self
    }

    /// Whether the slider answers the pointer.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The shared map the two values live in.
    pub fn values(mut self, range: SharedMap) -> Self {
        self.range = range;
        self
    }

    /// Called with the two ends whenever they change.
    pub fn on_change(mut self, callback: impl Fn((f32, f32)) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }
}

pub struct RangeSlider {
    style: RangeSliderStyle,
    min: f32,
    max: f32,
    step: f32,
    disabled: bool,
    range: SharedMap,
    on_change: Option<Arc<dyn Fn((f32, f32)) + Send + Sync + 'static>>,
    /// Which thumb the pointer is holding: `0` is the low one, `1` the high one.
    dragging: Option<usize>,
    /// The pointer is down on the track.
    pressed: bool,
    revision: Revision,
}

impl RangeSlider {
    /// The low end, as it stands.
    pub fn low(&self) -> f32 {
        read_range(&self.range).0
    }

    /// The high end, as it stands.
    pub fn high(&self) -> f32 {
        read_range(&self.range).1
    }

    /// The shared map the two values live in.
    pub fn range(&self) -> SharedMap {
        self.range.clone()
    }

    /// Write the low end, clamped into the range.
    pub fn set_low(&mut self, low: f32) {
        let clamped = low.clamp(self.min, self.max);
        let (_, high) = read_range(&self.range);
        write_range(&self.range, clamped, high);
        self.revision.bump();
    }

    /// Write the high end, clamped into the range.
    pub fn set_high(&mut self, high: f32) {
        let clamped = high.clamp(self.min, self.max);
        let (low, _) = read_range(&self.range);
        write_range(&self.range, low, clamped);
        self.revision.bump();
    }

    /// Whether the slider answers the pointer.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// v1's `update(which, ratio)`: quantise, clamp into the range, keep the
    /// two thumbs from crossing, write, and notify.
    fn move_thumb(&mut self, which: usize, ratio: f32) {
        let (mut low, mut high) = read_range(&self.range);
        let span = (self.max - self.min).max(f32::MIN_POSITIVE);
        let raw = self.min + ratio * span;
        let mut value = if self.step > 0.0 {
            let steps = ((raw - self.min) / self.step).round();
            (self.min + steps * self.step).clamp(self.min, self.max)
        } else {
            raw.clamp(self.min, self.max)
        };
        match which {
            0 => {
                if value > high {
                    value = high;
                }
                low = value;
            }
            _ => {
                if value < low {
                    value = low;
                }
                high = value;
            }
        }
        write_range(&self.range, low, high);
        self.revision.bump();
        if let Some(on_change) = &self.on_change {
            on_change((low, high));
        }
    }
}

impl Widget for RangeSlider {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// The track is the box: v1's track was `height(track_height)` and
    /// `width_full`, and the thumbs were painted over it.
    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(self.style.track_height),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        if self.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                if self.pressed {
                    let which = self.dragging.unwrap_or(0);
                    let ratio = ratio_at(cx.bounds(), pointer.position.x);
                    self.move_thumb(which, ratio);
                    return true;
                }
                false
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                self.pressed = false;
                self.dragging = None;
                false
            }
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                let ratio = ratio_at(cx.bounds(), pointer.position.x);
                let (low, high) = read_range(&self.range);
                // v1 picked the *nearer* thumb and dragged that one.
                let which = if (ratio - fraction(low, self.min, self.max)).abs()
                    <= (ratio - fraction(high, self.min, self.max)).abs()
                {
                    0
                } else {
                    1
                };
                self.dragging = Some(which);
                self.pressed = true;
                cx.request_focus(id);
                self.move_thumb(which, ratio);
                true
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                let was_pressed = self.pressed;
                self.pressed = false;
                self.dragging = None;
                was_pressed
            }
            _ => false,
        }
    }

    /// The track, then the band between the two thumbs, then the thumbs.
    ///
    /// That order is v1's: the band and the thumbs were absolutely positioned
    /// children of the track, and the track's own fill was underneath them.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let (low, high) = read_range(&self.range);
        let low_ratio = f64::from(fraction(low, self.min, self.max));
        let high_ratio = f64::from(fraction(high, self.min, self.max));

        // The track: a pill, centred in the box.
        let height = f64::from(self.style.track_height).min(bounds.height());
        let track = Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - height) / 2.0,
            x1: bounds.x1,
            y1: bounds.y0 + (bounds.height() + height) / 2.0,
        };
        cx.fill_rounded_rect(track, (height / 2.0) as f32, self.style.track_color);

        // The band between the two thumbs. v1's band was a plain rectangle
        // with no radius, drawn over the rounded track.
        let band = Rect {
            x0: track.x0 + track.width() * low_ratio,
            x1: track.x0 + track.width() * high_ratio,
            y0: track.y0,
            y1: track.y1,
        };
        cx.fill_rect(band, self.style.fill_color);

        // The two thumbs, centred on their values.
        let diameter = f64::from(self.style.thumb_size);
        for ratio in [low_ratio, high_ratio] {
            let centre = track.x0 + track.width() * ratio;
            let thumb = Rect {
                x0: centre - diameter / 2.0,
                x1: centre + diameter / 2.0,
                y0: track.y0 + height / 2.0 - diameter / 2.0,
                y1: track.y0 + height / 2.0 + diameter / 2.0,
            };
            cx.fill_rounded_rect(thumb, self.style.thumb_radius, self.style.thumb_color);
        }
    }

    fn cursor(&self) -> CursorStyle {
        // v1 set `CursorStyle::Pointer` on the track.
        if self.disabled {
            CursorStyle::Default
        } else {
            CursorStyle::Pointer
        }
    }
}

impl WidgetAny for RangeSlider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The pair, as a label.
    ///
    /// An addition rather than a translation: v1's range slider published no
    /// accessibility description at all, and a control nobody can read is a
    /// control nobody can use.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let (low, high) = read_range(&self.range);
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Slider,
            format!("{low} to {high}"),
        ))
    }
}
