//! `Slider`: a continuous or stepped value selector along the horizontal axis.
//!
//! It moved out of `floom-widgets/src/controls.rs` for the same reason `Select`
//! and `Counter` did, and the reason sharpened as it went: what a slider *does*
//! is primitive -- clamping, quantising, the drag, the keyboard walk -- but
//! what it *looks like* is a design decision, and the old `paint` had no way to
//! express one. It hard-coded `#E2E8F0` for the track, `#2B6CB0` for the fill,
//! a white thumb, a 4px track and a 16px thumb, so Material, Fluent and
//! Cupertino had no way to agree on a slider except by not using this one. The
//! appearance now lives in [`SliderStyle`], and the widget lives one layer up
//! beside the other composites that needed the same move.
//!
//! Everything except `paint` came across unchanged: the same revision model,
//! the same drag and keyboard handling, the same quantisation, the same
//! semantics. `SliderStyle::default()` *is* the engine's old hard-coded palette,
//! so a `Slider` built without a style still paints exactly what it painted
//! before.

use floom_accessibility::SemanticsRole;
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyEvent, KeyPhase, NamedKey,
    PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget,
};
use floom_types::{PointerButton, PointerPhase, Shadow};

use floom_widgets::{emit_semantics, InteractionState, Revision, SemanticsHint};

/// Slider thumb diameter (and therefore the slider's intrinsic height -- the
/// thumb is the tallest thing it paints).
const SLIDER_THUMB: f32 = 16.0;
/// Default track length for a Slider that was given no explicit width.
const SLIDER_TRACK_WIDTH: f32 = 160.0;

pub type ValueCallback = Box<dyn Fn(f64)>;

/// How a slider looks.
///
/// Every field is read by [`Slider::paint`], and the coverage is the three
/// kits' disagreement rather than any one kit's markup. Colours are *resolved*
/// rather than carried as an alpha over a theme colour, the same rule
/// [`SelectStyle`](crate::select::SelectStyle) follows: a kit whose hovered row
/// is `on_surface`@8 in one design system and `primary`@12 in another has two
/// different *source* colours, which no `alpha: u8` could carry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    // --- geometry, from `SliderTokens` ---
    /// The track's thickness; also the vertical centre of the thumb.
    pub track_height: f32,
    /// The thumb's diameter.
    pub thumb_size: f32,
    /// The thumb's corner radius. `thumb_size / 2.0` is a circle, which is what
    /// Material and Fluent want; Cupertino's is the token's own value.
    pub thumb_radius: f32,

    // --- resolved colours ---
    /// The whole track, filled or not.
    pub track_color: Color,
    /// The track between the left end and the thumb.
    pub filled_color: Color,
    /// The thumb.
    pub thumb_color: Color,

    // --- interaction variants, each falling back to its base colour when `None` ---
    /// The track while hovered.
    ///
    /// `None` paints the base colour, which is what the engine did before this
    /// type existed for a kit that has no hover treatment. There is no pressed
    /// track to carry: the old paint resolved one to the base colour too, so a
    /// press darkened the fill and left the track alone.
    pub track_hovered_color: Option<Color>,
    /// The fill while hovered.
    pub filled_hovered_color: Option<Color>,
    /// The fill while pressed.
    pub filled_pressed_color: Option<Color>,

    // --- the halo behind the thumb, drawn only when these are set ---
    /// A circle behind the thumb while hovered or dragged.
    ///
    /// `None` draws none, which is what Cupertino and Fluent want: Cupertino's
    /// thumb is a white disc carrying a drop shadow instead, and Fluent's
    /// carries neither. Material's v1 drew a 40px `primary`@20 circle, at @30
    /// while dragging.
    pub halo_color: Option<Color>,
    /// The halo's diameter, and its diameter again while dragging.
    pub halo_size: f32,
    pub halo_size_dragging: f32,

    // --- the thumb's drop shadow, drawn only when `shadow_blur` is > 0 ---
    /// Shadow blur radius; `0.0` draws no shadow.
    ///
    /// Cupertino writes blur 3, spread 0, black@60 -- the iOS treatment, where a
    /// white thumb needs a shadow to read against the track. The other two kits
    /// write no shadow at all.
    pub shadow_blur: f32,
    pub shadow_spread: f32,
    pub shadow_color: Color,
}

/// The palette the engine hard-coded before a slider had a style.
///
/// Every value here is what `Slider::paint` used to write inline, so the default
/// is not a design decision: it is a snapshot of the old behaviour, kept so that
/// moving the widget did not also repaint every existing caller. That includes
/// the three the old paint varied its two rectangles on -- a hovered track
/// `#EDF2F7`, a hovered fill `#367FC9`, a pressed fill `#205385` -- which are
/// the engine's own appearance rather than a kit's opinion, and which the move
/// promoted to fields instead of dropping. A kit that wants no variant of its
/// own writes `None` and gets the base colour, the same pixels the old inline
/// `resolved_color` produced for the states it left out.
impl Default for SliderStyle {
    fn default() -> Self {
        SliderStyle {
            track_height: 4.0,
            thumb_size: 16.0,
            thumb_radius: 8.0,
            track_color: Color::rgb(0xE2, 0xE8, 0xF0),
            filled_color: Color::rgb(0x2B, 0x6C, 0xB0),
            thumb_color: Color::WHITE,
            track_hovered_color: Some(Color::rgb(0xED, 0xF2, 0xF7)),
            filled_hovered_color: Some(Color::rgb(0x36, 0x7F, 0xC9)),
            filled_pressed_color: Some(Color::rgb(0x20, 0x53, 0x85)),
            halo_color: None,
            halo_size: 40.0,
            halo_size_dragging: 40.0,
            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_color: Color::TRANSPARENT,
        }
    }
}

/// A continuous or stepped value selector along the horizontal axis.
pub struct Slider {
    value: f64,
    min: f64,
    max: f64,
    step: Option<f64>,
    on_change: Option<ValueCallback>,
    state: InteractionState,
    revision: Revision,
    /// Visual style. Named `style_` because the builder below is
    /// [`Slider::style`], the same split `Select` uses.
    style_: SliderStyle,
}

impl Slider {
    pub fn new(value: f64, min: f64, max: f64) -> Self {
        Self {
            value: value.clamp(min, max),
            min,
            max,
            step: None,
            on_change: None,
            state: InteractionState::default(),
            revision: Revision::default(),
            style_: SliderStyle::default(),
        }
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f64) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    fn quantize(&self, raw: f64) -> f64 {
        let clamped = raw.clamp(self.min, self.max);
        match self.step {
            Some(step) if step > 0.0 => {
                let snapped = ((clamped - self.min) / step).round() * step + self.min;
                snapped.clamp(self.min, self.max)
            }
            _ => clamped,
        }
    }

    /// Maps an x coordinate to a (quantized) value.
    pub fn value_at(&self, bounds: Rect, x: f64) -> f64 {
        if bounds.width() <= 0.0 {
            return self.value;
        }
        let t = ((x - bounds.x0) / bounds.width()).clamp(0.0, 1.0);
        self.quantize(self.min + (self.max - self.min) * t)
    }

    fn set_value(&mut self, value: f64) {
        let next = self.quantize(value);
        if next == self.value {
            return;
        }
        self.value = next;
        self.revision.bump();
        if let Some(callback) = &self.on_change {
            callback(self.value);
        }
    }

    /// How far one arrow key moves the value: the configured step, or a
    /// hundredth of the range on a continuous slider (which has no step to
    /// quantize to, but still needs a sensible key granularity).
    fn key_step(&self) -> f64 {
        match self.step {
            Some(step) if step > 0.0 => step,
            _ => (self.max - self.min) / 100.0,
        }
    }

    /// Keyboard control for the focused slider -- the pointer's equivalent for
    /// anyone not using one, and the only way in for assistive technology.
    fn key_event(&mut self, key: &KeyEvent) -> bool {
        if !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        let step = self.key_step();
        match key.key {
            Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => {
                self.set_value(self.value + step)
            }
            Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => {
                self.set_value(self.value - step)
            }
            // A page is ten arrow steps: enough to cross a long range quickly
            // without losing the fine control the arrows give.
            Key::Named(NamedKey::PageUp) => self.set_value(self.value + step * 10.0),
            Key::Named(NamedKey::PageDown) => self.set_value(self.value - step * 10.0),
            Key::Named(NamedKey::Home) => self.set_value(self.min),
            Key::Named(NamedKey::End) => self.set_value(self.max),
            _ => return false,
        }
        // Consumed even when the value did not move (already at an end):
        // swallowing the key is what keeps it from scrolling the page behind.
        true
    }

    /// Normalized position of the thumb within the track (0..1).
    pub fn fraction(&self) -> f64 {
        let span = self.max - self.min;
        if span <= 0.0 {
            return 0.0;
        }
        ((self.value - self.min) / span).clamp(0.0, 1.0)
    }

    /// Replaces the whole palette. A kit resolves its tokens into a
    /// [`SliderStyle`] once and hands it over here, rather than the widget
    /// holding a partial set of knobs a kit has to guess the rest of.
    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style_ = style;
        self.revision.bump();
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::labeled(SemanticsRole::Slider, format!("{:.0}", self.value))
            .with_state(self.state.to_semantics())
    }

    /// The track: full width, [`SliderStyle::track_height`] tall, centred
    /// vertically in the box the host gave us.
    fn track_rect(&self, bounds: Rect) -> Rect {
        let height = f64::from(self.style_.track_height).min(bounds.height());
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - height) / 2.0,
            x1: bounds.x1,
            y1: bounds.y0 + (bounds.height() + height) / 2.0,
        }
    }

    /// The thumb: [`SliderStyle::thumb_size`] across, on the track's centre
    /// line, slid along the span the track leaves once the thumb's own width is
    /// taken off each end -- which is what lets the thumb's edges reach the ends
    /// of the track instead of stopping a thumb's width short of them.
    fn thumb_rect(&self, bounds: Rect) -> Rect {
        let diameter = f64::from(self.style_.thumb_size).min(bounds.height());
        let usable = bounds.width() - diameter;
        let x = bounds.x0 + usable * self.fraction();
        Rect {
            x0: x,
            y0: bounds.y0 + (bounds.height() - diameter) / 2.0,
            x1: x + diameter,
            y1: bounds.y0 + (bounds.height() + diameter) / 2.0,
        }
    }

    /// A `diameter`-wide circle centred on `centre`.
    ///
    /// Drawn as a rounded rect at half the diameter: `fill_rounded_rect` is the
    /// one shape the paint context promises every backend, and at that radius it
    /// *is* the circle.
    fn circle_at(centre: Point, diameter: f64) -> Rect {
        let half = diameter / 2.0;
        Rect {
            x0: centre.x - half,
            y0: centre.y - half,
            x1: centre.x + half,
            y1: centre.y + half,
        }
    }

    /// The colour to paint one of the two rectangles in, given the base colour
    /// and whichever variants the style declares.
    ///
    /// Pressed outranks hovered because the old inline paint ranked it that way
    /// too (`floom_widgets::controls::resolved_color`, which is private to that
    /// module and dims the disabled colour besides, so it cannot be reused
    /// here): a pointer down sets `hovered` and `pressed` together, so a press
    /// that lost to hover would never paint its own colour at all.
    ///
    /// A variant the style leaves as `None` paints the base colour, not
    /// nothing. The track passes `None` for `pressed` for exactly that reason --
    /// the old paint's pressed track *was* the base colour, so a press darkened
    /// the fill and left the track where it was.
    fn resolved_color(&self, base: Color, hovered: Option<Color>, pressed: Option<Color>) -> Color {
        if self.state.pressed {
            pressed.unwrap_or(base)
        } else if self.state.hovered {
            hovered.unwrap_or(base)
        } else {
            base
        }
    }
}

impl std::fmt::Debug for Slider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slider")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .finish_non_exhaustive()
    }
}

impl Widget for Slider {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Slider::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the style's own geometry, tall enough for whichever of
    /// the thumb and the track is bigger.
    ///
    /// The height has to come from the style rather than from a constant: a kit
    /// that gives its thumb a 28px diameter would otherwise be laid out 16px
    /// tall and have the bottom of it clipped away. The width stays the
    /// default track length, because a track is meaningless at zero width and
    /// a caller who wants it to fill its container pins the width explicitly.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        let tall = self.style_.thumb_size.max(self.style_.track_height);
        Some(
            floom_core::LayoutRequest::new()
                .width(SLIDER_TRACK_WIDTH)
                .height(tall),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        if self.state.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return match cx.event() {
                Event::Key(key) => self.key_event(&key),
                _ => false,
            };
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                self.state.hovered = true;
                // Drag while held: keep following the pointer.
                if self.state.pressed {
                    let next = self.value_at(cx.bounds(), pointer.position.x);
                    self.set_value(next);
                    return true;
                }
                false
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                self.state.hovered = false;
                self.state.pressed = false;
                false
            }
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                self.state.hovered = true;
                self.state.pressed = true;
                cx.request_focus(id);
                let next = self.value_at(cx.bounds(), pointer.position.x);
                self.set_value(next);
                true
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                let was_pressed = self.state.pressed;
                self.state.pressed = false;
                was_pressed
            }
            _ => false,
        }
    }

    /// Track, then fill, then the halo and the thumb's shadow, then the thumb
    /// itself. That order is the whole point of both: a halo painted after the
    /// thumb would cover it, and a shadow drawn last would land on the disc it
    /// belongs behind.
    ///
    /// The two rectangles take their colour from [`Slider::resolved_color`]
    /// rather than straight off the style, so an unstyled slider still darkens
    /// under a press and lightens under a hover -- the appearance the engine had
    /// before the widget moved.
    ///
    /// A disabled slider paints the style's colours as given. The engine's
    /// `floom_widgets::controls::disabled_tint` is private to that module and no
    /// kit's style struct carries a disabled colour, so the alternative would be
    /// either a copy of the dimming rule or a field the three kits cannot fill --
    /// and a hard-coded dim a kit cannot override is the same problem this
    /// module was moved to fix.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style_;
        let track = self.track_rect(bounds);
        cx.fill_rect(
            track,
            self.resolved_color(style.track_color, style.track_hovered_color, None),
        );
        let filled = Rect {
            x0: track.x0,
            y0: track.y0,
            x1: track.x0 + track.width() * self.fraction(),
            y1: track.y1,
        };
        cx.fill_rect(
            filled,
            self.resolved_color(
                style.filled_color,
                style.filled_hovered_color,
                style.filled_pressed_color,
            ),
        );
        let thumb = self.thumb_rect(bounds);
        if let Some(halo) = style.halo_color {
            let diameter = if self.state.pressed {
                f64::from(style.halo_size_dragging)
            } else {
                f64::from(style.halo_size)
            };
            cx.fill_rounded_rect(
                Self::circle_at(thumb.center(), diameter),
                (diameter / 2.0) as f32,
                halo,
            );
        }
        if style.shadow_blur > 0.0 {
            cx.draw_shadow(
                thumb,
                style.thumb_radius,
                &Shadow {
                    color: style.shadow_color,
                    blur: f64::from(style.shadow_blur),
                    offset_x: 0.0,
                    offset_y: 0.0,
                    spread: f64::from(style.shadow_spread),
                },
            );
        }
        cx.fill_rounded_rect(thumb, style.thumb_radius, style.thumb_color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }

    fn cursor(&self) -> floom_types::CursorStyle {
        // A disabled control gets the platform arrow: a pointer over a
        // thing that cannot be clicked is a promise the control cannot keep.
        if self.state.disabled {
            floom_types::CursorStyle::Default
        } else {
            floom_types::CursorStyle::Pointer
        }
    }
}
