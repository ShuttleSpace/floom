//! `Ripple` — the Material-style press ripple.
//!
//! A press starts an ink circle expanding from the pointer; a release fades it
//! out while holding the expanded size. The ink is clipped to the wrapper's
//! rounded bounds, so the same widget works behind a button, a card, or a list
//! row.
//!
//! # Why this needed no engine change
//!
//! The ripple is a *two-phase, pointer-driven* animation, which looks like a
//! case a declarative animation contract could not express — a widget returns
//! one `AnimationSpec` and the host owns what is running. It can express it.
//! `WidgetHost::sync_declared_animation` compares each declaration with the
//! previous one and restarts on a difference, so a widget that switches spec
//! from `event()` gets its new phase. Two things make that reachable:
//!
//! * `AnimationSpec::new` is a tween that drives nothing by itself, meaning
//!   "the widget reads the value" via `cx.animation_value()`. The phases are
//!   *sequential*, so one scalar covers radius-while-expanding and
//!   alpha-while-fading.
//! * The update pass visits every node each frame, and dirtiness is reported
//!   *by* `update()` (via `Revision::sync`) rather than gating which nodes run.
//!   So bumping the revision inside `event()` is what gets the new declaration
//!   reconciled. It is the one requirement, and
//!   `floom-app/tests/event_animation.rs` is the control that pins it.
//!
//! This needs no layout-change listener to learn its own box:
//! `paint` is handed `cx.bounds()` and `event` is handed `cx.bounds()`, so the
//! ink diameter can be derived from the real laid-out box at press time.

use std::cell::Cell;

use floom_core::{
    Child, Color, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    PointerPhase, Rect, UpdateContext, Widget, WidgetAny,
};
use floom_types::{AnimationSpec, ClipShape, Easing};

use crate::primitives::Revision;

/// Which half of the interaction the ripple is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Not pressed: nothing is painted.
    Idle,
    /// Pressed: the ink grows from nothing to the full diameter.
    Expanding,
    /// Released: the ink holds its size and fades out.
    Fading,
}

/// A single-child wrapper that paints a press ripple over its child.
pub struct Ripple {
    child: Option<Child>,
    /// Corner radius the ink is clipped to.
    radius: f64,
    /// Ink colour at full opacity.
    ink: Color,
    /// Seconds the expansion takes.
    expand_secs: f64,
    /// Seconds the fade-out takes.
    fade_secs: f64,
    disabled: bool,
    phase: Phase,
    /// Pointer position in this element's own coordinates, set at press time.
    origin: Option<(f64, f64)>,
    /// Diameter of the circle that covers the whole box from `origin`.
    diameter: f64,
    /// Sampled animation value from the last paint, for tests and for the
    /// host to read back. A `Cell` because `paint` takes `&self`.
    last_value: Cell<f64>,
    revision: Revision,
}

impl std::fmt::Debug for Ripple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ripple")
            .field("phase", &self.phase)
            .field("radius", &self.radius)
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

impl Default for Ripple {
    fn default() -> Self {
        Self {
            child: None,
            radius: 0.0,
            ink: Color::rgba(0, 0, 0, 60),
            expand_secs: 0.4,
            fade_secs: 0.4,
            disabled: false,
            phase: Phase::Idle,
            origin: None,
            diameter: 0.0,
            last_value: Cell::new(0.0),
            revision: Revision::default(),
        }
    }
}

impl Ripple {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap `child`, so the ripple paints over it.
    pub fn child(
        mut self,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new("ripple-child", layout, build));
        self.revision.bump();
        self
    }

    /// Clip the ink to corners rounded by `radius`.
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self.revision.bump();
        self
    }

    /// The ink colour, at full opacity. Alpha here is the resting opacity.
    pub fn ink(mut self, ink: Color) -> Self {
        self.ink = ink;
        self.revision.bump();
        self
    }

    pub fn durations(mut self, expand_secs: f64, fade_secs: f64) -> Self {
        self.expand_secs = expand_secs;
        self.fade_secs = fade_secs;
        self.revision.bump();
        self
    }

    /// Suppress the ripple. Events are still passed through to the child, so a
    /// disabled ripple is inert rather than swallowing the press.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.revision.bump();
        self
    }

    /// Re-clip the ink to a new corner radius.
    ///
    /// The builder form takes a plain value because this widget is
    /// theme-agnostic, but a kit still has to follow a theme change: the radius
    /// function used to re-evaluate against the theme on every use. The tree is
    /// mounted once and mutated through `WidgetHost::widget_mut`, so the kit
    /// resolves the theme and pushes the new value in here. Same for
    /// [`Ripple::set_ink`] and [`Ripple::set_disabled`].
    pub fn set_radius(&mut self, radius: f64) {
        if self.radius != radius {
            self.radius = radius;
            self.revision.bump();
        }
    }

    /// Replace the ink colour (given at full opacity).
    pub fn set_ink(&mut self, ink: Color) {
        if self.ink != ink {
            self.ink = ink;
            self.revision.bump();
        }
    }

    /// Turn the ripple on or off without rebuilding it.
    pub fn set_disabled(&mut self, disabled: bool) {
        if self.disabled != disabled {
            self.disabled = disabled;
            self.revision.bump();
        }
    }

    /// The clip radius currently in force.
    /// The ink colour at full opacity.
    ///
    /// A reader to go with [`Self::ink`] and [`Self::set_ink`], for the same reason
    /// [`Self::clip_radius`] exists: a caller that composes a ripple over its own
    /// surface needs to know the two agree, and a test needs to read what the
    /// variant resolved to. Named `ink_color` because `ink` is the builder.
    pub fn ink_color(&self) -> Color {
        self.ink
    }

    pub fn clip_radius(&self) -> f64 {
        self.radius
    }

    /// Whether the ripple is suppressed.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Which phase the ripple is in.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The animation value read on the last paint.
    pub fn last_value(&self) -> f64 {
        self.last_value.get()
    }

    /// The circle's diameter for a press at `local` inside a box of `size`.
    ///
    /// The circle is centred on the pointer, so the diameter is twice the
    /// distance to the *farthest* corner — the smallest circle that covers the
    /// whole box, which is what makes the ink reach every edge rather than
    /// stopping short on the near side.
    fn diameter_at(origin: (f64, f64), size: (f64, f64)) -> f64 {
        let (lx, ly) = origin;
        let (w, h) = size;
        let dx = (w - lx).max(lx);
        let dy = (h - ly).max(ly);
        2.0 * (dx * dx + dy * dy).sqrt()
    }
}

impl Widget for Ripple {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) => vec![child.clone()],
            None => Vec::new(),
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        None
    }

    /// The declaration. One spec per phase, and the phases differ, so the host
    /// sees a change and restarts.
    fn animation(&self) -> Option<AnimationSpec> {
        let mut spec = match self.phase {
            // Present but inert, so a resting ripple still has a declaration
            // for the next one to differ from.
            Phase::Idle => AnimationSpec::new(0.0, 0.0, 0.0),
            Phase::Expanding => AnimationSpec::new(0.0, 1.0, self.expand_secs),
            Phase::Fading => AnimationSpec::new(0.0, 1.0, self.fade_secs),
        };
        // Distinct keys so `matches` cannot call two phases equal on the
        // numbers alone (a 0-duration idle vs a 0-duration fade, say).
        spec.key = match self.phase {
            Phase::Idle => 0,
            Phase::Expanding => 1,
            Phase::Fading => 2,
        };
        spec.easing = Easing::Linear;
        Some(spec)
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        if self.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        let bounds = cx.bounds();
        // `cursor_position` is in the same space as `bounds`, so the ink is
        // anchored where the press actually landed inside this element.
        let Some(at) = cx.cursor_position() else {
            return false;
        };
        let local = (at.x - bounds.x0, at.y - bounds.y0);

        match pointer.phase {
            PointerPhase::Down => {
                self.origin = Some(local);
                self.diameter = Self::diameter_at(local, (bounds.width(), bounds.height()));
                self.phase = Phase::Expanding;
            }
            // Only a release that follows a press starts the fade; a release
            // with no press in flight must not conjure ink.
            PointerPhase::Up if self.phase == Phase::Expanding => {
                self.phase = Phase::Fading;
            }
            _ => return false,
        }
        // The one line the whole mechanism rests on.
        self.revision.bump();
        // Never consume: the child underneath still has to see this press, or
        // wrapping a button in a ripple would disable the button.
        false
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let t = cx.animation_value().unwrap_or(0.0);
        self.last_value.set(t);
        if self.phase == Phase::Idle {
            return;
        }
        let Some((ox, oy)) = self.origin else {
            return;
        };
        let (radius, alpha) = match self.phase {
            Phase::Idle => return,
            // Grows with the animation; fully opaque throughout.
            Phase::Expanding => (t.clamp(0.0, 1.0) * self.diameter, 1.0),
            // Holds the expanded size and fades.
            Phase::Fading => (self.diameter, 1.0 - t.clamp(0.0, 1.0)),
        };
        if radius <= 0.0 || alpha <= 0.0 {
            return;
        }
        let bounds = cx.bounds();
        // Centred on the pointer, so it expands symmetrically outward from it.
        let rect = Rect {
            x0: bounds.x0 + ox - radius / 2.0,
            y0: bounds.y0 + oy - radius / 2.0,
            x1: bounds.x0 + ox + radius / 2.0,
            y1: bounds.y0 + oy + radius / 2.0,
        };
        cx.fill_rounded_rect(
            rect,
            (radius / 2.0) as f32,
            Color {
                a: (f32::from(self.ink.a) * alpha as f32)
                    .round()
                    .clamp(0.0, 255.0) as u8,
                ..self.ink
            },
        );
    }

    /// Clip the ink to this element's rounded box. Without this the circle
    /// would spill past the corners it is meant to be tucked into.
    fn clip(&self) -> Option<ClipShape> {
        Some(ClipShape::RoundedBox {
            radius: self.radius,
        })
    }
}

/// Convenience: wrap a child in a ripple.
pub fn ripple(
    layout: LayoutRequest,
    build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
) -> Ripple {
    Ripple::new().child(layout, build)
}
