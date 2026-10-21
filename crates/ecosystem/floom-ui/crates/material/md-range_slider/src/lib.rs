//! Material range slider, ported to the Floom 2.0 widget API.
//!
//! A range slider is a track with two thumbs: a low one and a high one. The
//! band between them is the selection, and dragging either thumb moves it --
//! the low thumb stops at the high one, and the high thumb stops at the low
//! one.
//!
//! ## What the port uses
//!
//! A kit-local widget. The engine's `Slider` is a *single*-valued control: it
//! paints a track, a fill that runs from the left end to its value, and one
//! thumb. A range slider needs a fill that starts where the low thumb is, and
//! `SliderStyle` has no way to say that -- its fill always starts at zero.
//! Two engine sliders side by side would be two tracks, two fills and two
//! drags, which is not this component.
//!
//! So the port carries the widget itself, on the engine's own contract: a
//! [`floom_core::Widget`] with a `layout_request`, a `paint`, and pointer
//! handling in `event`, reading the same `Event` / `EventContext` /
//! `PaintContext` vocabulary the engine's `Slider` reads.
//!
//! ## What the port uses
//!
//! The engine's [`RangeSlider`]: a track with two thumbs. The engine's own
//! `Slider` is a *single*-valued control -- it paints a fill that runs from the
//! left end to its value, and it has no way to say that the fill should start
//! where the low thumb is -- so two engine sliders side by side would be two
//! tracks, two fills and two drags, which is not this component. The engine
//! grew the two-valued one for this, and the three kits now say only what
//! they answer: the track's colour, the band's colour, the thumbs' colour and
//! the three sizes.
//!
//! ## What this kit answers
//!
//! * the track's thickness, the thumb's size and the thumb's radius: the
//!   tokens;
//! * the track's colour: `surface_variant`; the band and the two thumbs:
//!   `primary`;
//! * the disabled palette: `surface_variant` and `primary` at 120 alpha, which
//!   is what v1's three style closures composed.
//!
//! ## What v1 had that this port does not carry
//!
//! * **The style struct.** v1's range slider had none: no `resolved_style`,
//!   no `MdRangeSliderStyle`. The port *adds* one, and the fields it carries
//!   are the ones the paint uses.
//! * **The two signals.** v1 handed out `low_signal()` and `high_signal()`.
//!   A 2.0 slot closure is `Send + Sync` and an `RwSignal` is `Rc`-based, so
//!   the two values live in the engine's `SharedMap` instead, under the keys
//!   `low` and `high`. The map holds strings, so the two values travel as
//!   their decimal text; `f32`'s `Display` round-trips exactly, so nothing is
//!   lost -- but this is the engine's map shape, not a typed handle.
//! * **The pointer capture.** v1 asked for pointer capture on the way down,
//!   so a drag that left the track kept tracking. 2.0's `EventContext` has no
//!   capture request -- the engine's own `Slider` has the same gap -- so a
//!   drag that leaves the track ends at the edge.
//! * **The callback's shape.** v1's was `Box<dyn Fn((f32, f32))>`; the port's
//!   is `Arc<dyn Fn((f32, f32)) + Send + Sync>`, because a callback that
//!   reaches a 2.0 slot has to cross a thread boundary. The props' own
//!   `on_change` slot stays empty: the shared props type is
//!   `Box<dyn Fn((f32, f32))>` and cannot hold the `Send + Sync` handle the
//!   kit uses.
//! * **The step of zero.** v1 divided by `step` without looking at it, so a
//!   zero step produced `NaN`. The port quantises only for a positive step,
//!   which is the rule the engine's own `Slider` uses.
//!
//! ## The numbers v1 wrote as literals
//!
//! None: every number v1's range slider used came from the tokens.
//!
//! ## The theme
//!
//! Material reads the app-wide theme signal, so a change of theme re-themes
//! the slider along with everything else. Cupertino and Fluent take a snapshot
//! of their own design system instead -- which is what v1 did, and what the
//! other two kits in this family still do.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::{
    CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext, Rect,
    UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, CursorStyle, PointerButton, PointerPhase};
use floom_ui_composites::{RangeSlider, RangeSliderStyle};
use floom_widgets::{Revision, SharedMap, shared_map};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::range_slider::RangeSliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// The key the low value lives under in the shared map.
const LOW_KEY: &str = "low";
/// The key the high value lives under in the shared map.
const HIGH_KEY: &str = "high";

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Read the two values out of the shared map.
///
/// A poisoned mutex reads as "no value": the caller that poisoned it is as
/// stuck as the widget is, and panicking inside a paint pass over a number
/// nobody is looking at would take the frame down with it.
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

/// Write the two values into the shared map.
fn write_range(range: &SharedMap, low: f32, high: f32) {
    let Ok(mut guard) = range.lock() else {
        return;
    };
    guard.insert(LOW_KEY.to_string(), format!("{low}"));
    guard.insert(HIGH_KEY.to_string(), format!("{high}"));
}

/// A value's place along the track, as a fraction of the range.
fn fraction(value: f32, min: f32, max: f32) -> f32 {
    let span = (max - min).max(f32::MIN_POSITIVE);
    ((value - min) / span).clamp(0.0, 1.0)
}

/// A pointer's place along the track, as a fraction of the track's width.
fn ratio_at(bounds: Rect, x: f64) -> f32 {
    if bounds.width() <= 0.0 {
        return 0.0;
    }
    (((x - bounds.x0) / bounds.width()) as f32).clamp(0.0, 1.0)
}

/// The range slider's appearance, in the terms v1 used.
///
/// v1 had no style struct at all, so every field here is one the port *adds*:
/// these are the numbers and the colours the paint reads, and a style that
/// does not say what the widget draws is not the widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialRangeSliderStyle {
    pub design_system: &'static str,
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

/// Material's range slider.
pub struct MaterialRangeSlider {
    props: RangeSliderProps,
    theme: RwSignal<MultiTheme>,
    /// The two values, shared with whoever drives the slider.
    range: SharedMap,
    /// The change callback, in the shape a 2.0 slot needs.
    ///
    /// Not the props' own slot: the shared props type is `Box<dyn Fn(..)>`, and
    /// a `Box<dyn Fn(..)>` cannot be held by a `Send + Sync` closure.
    on_change: Option<Arc<dyn Fn((f32, f32)) + Send + Sync + 'static>>,
}

impl MaterialRangeSlider {
    pub fn new() -> Self {
        let props = RangeSliderProps::default();
        let min = props.min;
        let max = props.max;
        let range = shared_map(HashMap::new());
        // v1 clamped the initial pair into the range, and so does this.
        write_range(
            &range,
            props.low.clamp(min, max),
            props.high.clamp(min, max),
        );
        Self {
            props,
            theme: theme_signal(),
            range,
            on_change: None,
        }
    }

    /// The low end of the selection.
    pub fn low(mut self, low: f32) -> Self {
        self.props.low = low;
        let clamped = low.clamp(self.props.min, self.props.max);
        let (_, high) = read_range(&self.range);
        write_range(&self.range, clamped, high);
        self
    }

    /// The high end of the selection.
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

    /// Fired whenever either thumb moves, with the pair.
    ///
    /// `Send + Sync`, where v1's was not: the callback reaches a 2.0 slot.
    pub fn on_change(
        mut self,
        callback: impl Fn((f32, f32)) + Send + Sync + 'static,
    ) -> Self {
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

    /// The shared map the two values live in. v1's `low_signal` and
    /// `high_signal`, in the shape a 2.0 caller can hold.
    pub fn range(&self) -> SharedMap {
        self.range.clone()
    }

    /// The low end, as it stands.
    pub fn low_value(&self) -> f32 {
        read_range(&self.range).0
    }

    /// The high end, as it stands.
    pub fn high_value(&self) -> f32 {
        read_range(&self.range).1
    }

    pub fn resolved_style(&self) -> MaterialRangeSliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::RangeSlider(tokens) = theme.component(ComponentId::RangeSlider) else {
            unreachable!("ComponentId::RangeSlider must resolve to RangeSliderTokens")
        };
        let colors = theme.colors();
        // v1's disabled palette, composed in its three style closures: the
        // track and the band at 120 alpha.
        let (track, fill, thumb) = if self.props.disabled {
            (
                with_alpha(colors.surface_variant, 120),
                with_alpha(colors.primary, 120),
                with_alpha(colors.primary, 120),
            )
        } else {
            (colors.surface_variant, colors.primary, colors.primary)
        };
        MaterialRangeSliderStyle {
            design_system: "material",
            track_color: track,
            fill_color: fill,
            thumb_color: thumb,
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,
        }
    }

    /// The slider, ready to mount.
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

impl Default for MaterialRangeSlider {
    fn default() -> Self {
        Self::new()
    }
}
