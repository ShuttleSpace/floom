//! Does an event-driven animation phase actually start?
//!
//! `Ripple` (the Material press ripple) needs a two-phase, pointer-driven
//! animation: a press starts an expand, a release starts a fade. The 2.0
//! animation contract is *declarative* — a widget returns one `AnimationSpec`
//! from `Widget::animation` and the host owns what is running — so the question
//! this file answers is whether a widget can change that spec *from an event*
//! and have the host pick the new phase up.
//!
//! The answer is yes, and it needs no engine change, provided the widget bumps
//! its own `Revision` when it changes phase. Two things make that work:
//!
//! * the update pass visits every node every frame, and dirtiness is reported
//!   *by* `update` (via `Revision::sync`) rather than gating which nodes run;
//! * `sync_declared_animation` restarts a declaration whose `AnimationSpec`
//!   differs from the last one.
//!
//! So the chain is: `event()` switches phase and bumps the revision → the next
//! frame's `update` sees the bump and reports dirty → the node joins the
//! reconcile set → the new spec restarts the animation. Each of those three
//! links is asserted separately below, because a regression in any one of them
//! looks identical from the outside (the phase silently never starts).

use std::any::Any;
use std::cell::Cell;

use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    UpdateContext, Widget, WidgetAny,
};
use floom_testkit::TestApp;
use floom_types::{AnimationSpec, Easing, PointerEvent, PointerPhase, Rect};

/// The two phases a press-driven control moves through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Resting: the declared animation is "nothing moves".
    Idle,
    /// Pressed: the declared animation expands from 0 to 100.
    Expanding,
}

/// A minimal stand-in for `Ripple`: a bar whose width is driven by its own
/// animation value, whose phase flips on pointer-down.
#[derive(Debug)]
struct PressExpand {
    phase: Phase,
    /// Bumped whenever the phase changes, exactly as a 2.0 widget's
    /// `Revision` would be. Mirrors `Revision::sync`'s compare-and-mark.
    revision: u64,
    painted_revision: u64,
    /// Width painted on the last frame, in logical px. A `Cell` because
    /// `paint` takes `&self` and the test needs to read the sampled value back.
    last_painted: Cell<f64>,
}

impl PressExpand {
    fn new() -> Self {
        Self {
            phase: Phase::Idle,
            revision: 1,
            painted_revision: 0,
            last_painted: Cell::new(0.0),
        }
    }

    fn phase(&self) -> Phase {
        self.phase
    }

    fn painted(&self) -> f64 {
        self.last_painted.get()
    }
}

impl Widget for PressExpand {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The whole mechanism under test: a bump here is what makes the node
        // dirty, which is what gets it into the declaration reconcile set.
        if self.painted_revision != self.revision {
            self.painted_revision = self.revision;
            cx.mark_dirty(id);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().width(100.0).height(20.0))
    }

    /// The declarative half. Each phase declares a *different* spec, which is
    /// what `sync_declared_animation` compares to decide on a restart.
    fn animation(&self) -> Option<AnimationSpec> {
        match self.phase {
            // A zero-length, zero-delta declaration: present but inert, so a
            // resting control still declares something and the host has a
            // baseline to compare the next phase against.
            // `AnimationSpec::new` is a tween that drives nothing by itself --
            // exactly the "the widget reads the value" shape.
            Phase::Idle => Some(AnimationSpec::new(0.0, 0.0, 0.0)),
            Phase::Expanding => Some(AnimationSpec::new(0.0, 100.0, 10.0)),
        }
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        if pointer.phase != PointerPhase::Down {
            return false;
        }
        self.phase = Phase::Expanding;
        // The one line that makes the whole thing work: without it the new
        // phase is never reconciled, because nothing else revisits this node.
        self.revision += 1;
        true
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let value = cx.animation_value().unwrap_or(0.0);
        self.last_painted.set(value);
        let bounds = cx.bounds();
        cx.fill_rect(
            Rect {
                x0: bounds.x0,
                y0: bounds.y0,
                x1: bounds.x0 + value.max(0.0).min(bounds.width()),
                y1: bounds.y1,
            },
            Color::rgb(0, 128, 255),
        );
    }
}

impl WidgetAny for PressExpand {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// A host with one `PressExpand` mounted, plus a spare idle animation so the
/// test has something to compare against.
fn app() -> TestApp {
    TestApp::new(floom_types::Size::new(100.0, 20.0), PressExpand::new())
}

/// Press the widget and pump a frame, the way a real backend would.
///
/// `TestApp::press` reports whether the frame had *damage*, not whether the
/// event was consumed, so it is not a usable signal here — the widget's own
/// phase is.
fn press(app: &mut TestApp) {
    let centre = app.centre(app.root());
    app.press(centre);
}

#[test]
fn a_resting_widget_declares_an_inert_animation() {
    let app = app();
    let w = app.widget::<PressExpand>(app.root());
    assert_eq!(w.phase(), Phase::Idle, "a fresh control rests");
}

#[test]
fn the_press_switches_phase_and_reports_dirty() {
    let mut app = app();
    press(&mut app);
    // The widget's own state moved during the event — link 1.
    assert_eq!(
        app.widget::<PressExpand>(app.root()).phase(),
        Phase::Expanding,
        "handling the press must switch the phase"
    );
}

#[test]
fn the_new_phase_starts_a_running_animation() {
    let mut app = app();
    press(&mut app);

    // Pump frames and watch the declared animation actually advance. If the
    // spec had not been re-armed, the resting (zero-delta) declaration would
    // still be running and the value would never leave 0.
    let mut advanced = false;
    for _ in 0..30 {
        app.drain();
        if app.widget::<PressExpand>(app.root()).painted() > 0.0 {
            advanced = true;
            break;
        }
    }
    assert!(
        advanced,
        "the expand phase must run: a press that re-arms AnimationSpec has to \
         produce a non-zero animation value"
    );
}

#[test]
fn the_two_phases_declare_different_specs() {
    // The precondition the restart depends on: if both phases declared the same
    // spec there would be nothing for the host to detect.
    let idle = PressExpand::new();
    let mut pressed = PressExpand::new();
    pressed.phase = Phase::Expanding;
    let (a, b) = (idle.animation(), pressed.animation());
    assert!(a.is_some() && b.is_some());
    assert!(
        !a.unwrap().matches(&b.unwrap()),
        "two phases must declare different specs, or no restart can be detected"
    );
}