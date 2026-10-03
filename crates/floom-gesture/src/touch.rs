//! Touch gesture classifier — turn a raw touch stream into semantic gestures.
//!
//! Mobile platforms deliver a stream of low-level touch samples (finger down,
//! move, up/cancel). Floom widgets want higher-level intent: "this was a tap"
//! vs "this was a scroll/drag". This module classifies the two, mirroring the
//! `TouchState` machine in gpui-mobile's `ios/window.rs`.
//!
//! It is framework-agnostic: it consumes [`TouchSample`]s and emits neutral
//! [`GestureEvent`]s. The platform backend is responsible for mapping those to
//! Floom [`Event`]s (see [`GestureEvent::to_floom`]) and for feeding a fling
//! velocity into a [`MomentumScroller`](crate::momentum::MomentumScroller).
//!
//! # Usage
//!
//! ```ignore
//! use floom_gesture::{TouchRecognizer, TouchSample, TouchPhase};
//!
//! let mut rec = TouchRecognizer::new();
//! // On each native touch:
//! for g in rec.handle(TouchSample { phase, x, y, tap_count: 1 }) {
//!     for event in g.to_floom(modifiers) {
//!         window.dispatch(event);
//!     }
//!     if let GestureEvent::Fling { vx, vy } = g {
//!         scroller.fling(vx, vy, last_x, last_y);
//!     }
//! }
//! ```
//!
//! # Ported from
//!
//! Adapted from [`gpui-mobile`](https://github.com/itsbalamurali/gpui-mobile)
//! (`src/ios/window.rs`, the `TouchState` machine), used under its Apache-2.0
//! option. The physics/tracking were removed so the module lives in
//! `floom-gesture` and only depends on `floom-types` + the in-crate
//! `VelocityTracker`.

use crate::momentum::VelocityTracker;
use floom_types::{
    Event, Modifiers, PointerButton, PointerEvent, PointerKind, PointerPhase, ScrollEvent,
};

/// Distance (logical px) a finger must travel before a press is promoted from
/// "potential tap" to "scroll". Below this, lifting the finger is a tap; above
/// it, movement is scroll/drag. Matches gpui-mobile's `SCROLL_SLOP` (8.0).
pub const DEFAULT_SCROLL_SLOP: f32 = 8.0;

/// Lifecycle phase of a raw touch, independent of platform naming.
///
/// `Stationary` (a held-but-unmoved finger) is folded into `Move` here: it
/// carries no position change and therefore cannot promote or scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// Finger/pen touched down.
    Start,
    /// Finger/pen moved (or held still).
    Move,
    /// Finger/pen lifted.
    End,
    /// Gesture cancelled by the system (palm rejection, window lost focus).
    Cancel,
}

/// A single raw touch sample fed to the recognizer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchSample {
    pub phase: TouchPhase,
    /// Logical-x at sample time (logical px).
    pub x: f32,
    /// Logical-y at sample time (logical px).
    pub y: f32,
    /// Native tap count (1 = single tap, 2 = double, …). Pass `1` if the
    /// platform does not expose one.
    pub tap_count: u32,
}

/// A classified gesture produced by [`TouchRecognizer`].
///
/// Positions are the logical coordinates of the relevant pointer; deltas are
/// logical px moved since the previous sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureEvent {
    /// Finger lifted without exceeding the slop threshold → a tap/click,
    /// emitted at the original down position so hit-testing matches the press.
    Tap { x: f32, y: f32, tap_count: u32 },
    /// The press crossed the slop threshold; scrolling begins at this point.
    ScrollStart { x: f32, y: f32 },
    /// Incremental scroll delta since the previous sample while scrolling.
    ScrollDelta { dx: f32, dy: f32, x: f32, y: f32 },
    /// Scrolling finished (finger lifted after a scroll). The backend should
    /// stop accumulating raw deltas; it then handles the paired `Fling`.
    ScrollEnd,
    /// Finger lifted after scrolling with a release velocity (logical px/s).
    /// The backend feeds this into a momentum scroller for inertia. Emitted on
    /// every scroll-end (a near-zero velocity is harmless — the scroller
    /// ignores sub-threshold flings).
    Fling { vx: f32, vy: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,
    Pending { start_x: f32, start_y: f32 },
    Scrolling { prev_x: f32, prev_y: f32 },
}

/// Classifies a raw touch stream into taps vs scrolls.
///
/// One recognizer tracks a single active contact. Multi-touch (pinch/rotate)
/// is out of scope — gpui-mobile itself only tracks one finger here.
pub struct TouchRecognizer {
    slop: f32,
    state: State,
    velocity: VelocityTracker,
}

impl TouchRecognizer {
    /// A recognizer with the default 8px slop threshold.
    pub fn new() -> Self {
        Self::with_slop(DEFAULT_SCROLL_SLOP)
    }

    /// A recognizer with a custom slop threshold (logical px).
    pub fn with_slop(slop: f32) -> Self {
        Self {
            slop,
            state: State::Idle,
            velocity: VelocityTracker::new(),
        }
    }

    /// Feed one touch sample. Returns the classified gesture(s) for this
    /// sample — usually zero or one, but the tap→scroll promotion emits both
    /// `ScrollStart` and the first `ScrollDelta`.
    pub fn handle(&mut self, t: TouchSample) -> Vec<GestureEvent> {
        match t.phase {
            TouchPhase::Start => {
                self.velocity.reset();
                self.state = State::Pending {
                    start_x: t.x,
                    start_y: t.y,
                };
                Vec::new()
            }

            TouchPhase::Move => {
                self.velocity.record(t.x, t.y);
                let mut out = Vec::new();
                match self.state {
                    State::Pending { start_x, start_y } => {
                        let dx = t.x - start_x;
                        let dy = t.y - start_y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > self.slop {
                            self.state = State::Scrolling {
                                prev_x: t.x,
                                prev_y: t.y,
                            };
                            out.push(GestureEvent::ScrollStart { x: t.x, y: t.y });
                            out.push(GestureEvent::ScrollDelta {
                                dx,
                                dy,
                                x: t.x,
                                y: t.y,
                            });
                        }
                        // Below the slop we stay Pending and emit nothing —
                        // deferring the down avoids accidental taps/navigation
                        // when a scroll starts near a button.
                    }
                    State::Scrolling { prev_x, prev_y } => {
                        let dx = t.x - prev_x;
                        let dy = t.y - prev_y;
                        self.state = State::Scrolling {
                            prev_x: t.x,
                            prev_y: t.y,
                        };
                        out.push(GestureEvent::ScrollDelta {
                            dx,
                            dy,
                            x: t.x,
                            y: t.y,
                        });
                    }
                    State::Idle => { /* spurious move, ignore */ }
                }
                out
            }

            TouchPhase::End | TouchPhase::Cancel => {
                let mut out = Vec::new();
                match self.state {
                    State::Pending { start_x, start_y } => {
                        if t.phase == TouchPhase::End {
                            // Lifted within slop → a tap at the original down
                            // position, carrying the native tap count.
                            out.push(GestureEvent::Tap {
                                x: start_x,
                                y: start_y,
                                tap_count: t.tap_count,
                            });
                        }
                        self.velocity.reset();
                        self.state = State::Idle;
                    }
                    State::Scrolling { .. } => {
                        // End the scroll gesture and hand the release velocity
                        // to the caller (via `Fling`) for inertia. Mirrors
                        // gpui-mobile's unconditional `momentum_scroller.fling`.
                        let (vx, vy) = self.velocity.velocity();
                        out.push(GestureEvent::ScrollEnd);
                        out.push(GestureEvent::Fling { vx, vy });
                        self.velocity.reset();
                        self.state = State::Idle;
                    }
                    State::Idle => { /* spurious up, ignore */ }
                }
                out
            }
        }
    }

    /// Whether the recognizer is currently in a scroll/drag.
    pub fn is_scrolling(&self) -> bool {
        matches!(self.state, State::Scrolling { .. })
    }
}

impl Default for TouchRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureEvent {
    /// Map a classified gesture to zero or more Floom [`Event`]s.
    ///
    /// `ScrollEnd` and `Fling` carry no direct Floom event — `ScrollEnd` tells
    /// the backend to stop accumulating raw deltas and `Fling` is the cue to
    /// start inertia. They are returned via [`TouchRecognizer::handle`] so the
    /// backend can act on them; this method emits the events Floom understands
    /// natively (pointer taps, scroll deltas).
    pub fn to_floom(self, modifiers: Modifiers) -> Vec<Event> {
        match self {
            GestureEvent::Tap { x, y, tap_count } => {
                let pos = floom_types::Point::new(x as f64, y as f64);
                vec![
                    Event::Pointer(PointerEvent {
                        phase: PointerPhase::Down,
                        kind: PointerKind::Touch,
                        button: Some(PointerButton::Primary),
                        position: pos,
                        modifiers,
                        click_count: tap_count as u16,
                    }),
                    Event::Pointer(PointerEvent {
                        phase: PointerPhase::Up,
                        kind: PointerKind::Touch,
                        button: Some(PointerButton::Primary),
                        position: pos,
                        modifiers,
                        click_count: tap_count as u16,
                    }),
                ]
            }
            GestureEvent::ScrollStart { x, y } => {
                vec![Event::Scroll(ScrollEvent {
                    delta_x: 0.0,
                    delta_y: 0.0,
                    position: floom_types::Point::new(x as f64, y as f64),
                    modifiers,
                })]
            }
            GestureEvent::ScrollDelta { dx, dy, x, y } => {
                vec![Event::Scroll(ScrollEvent {
                    delta_x: dx as f64,
                    delta_y: dy as f64,
                    position: floom_types::Point::new(x as f64, y as f64),
                    modifiers,
                })]
            }
            GestureEvent::ScrollEnd | GestureEvent::Fling { .. } => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(x: f32, y: f32) -> TouchSample {
        TouchSample {
            phase: TouchPhase::Start,
            x,
            y,
            tap_count: 1,
        }
    }
    fn mv(x: f32, y: f32) -> TouchSample {
        TouchSample {
            phase: TouchPhase::Move,
            x,
            y,
            tap_count: 1,
        }
    }
    fn end(x: f32, y: f32) -> TouchSample {
        TouchSample {
            phase: TouchPhase::End,
            x,
            y,
            tap_count: 1,
        }
    }

    #[test]
    fn start_is_inert() {
        let mut r = TouchRecognizer::new();
        assert!(r.handle(start(0.0, 0.0)).is_empty());
        assert!(!r.is_scrolling());
    }

    #[test]
    fn tap_within_slop_emits_tap() {
        let mut r = TouchRecognizer::new();
        r.handle(start(10.0, 10.0));
        // Small jiggle within slop must NOT promote to scroll.
        r.handle(mv(12.0, 11.0));
        let events = r.handle(end(11.0, 10.0));
        assert_eq!(events.len(), 1);
        match &events[0] {
            GestureEvent::Tap { x, y, tap_count } => {
                // Tap is emitted at the *down* position, not the lift position.
                assert_eq!(*x, 10.0);
                assert_eq!(*y, 10.0);
                assert_eq!(*tap_count, 1);
            }
            other => panic!("expected Tap, got {other:?}"),
        }
    }

    #[test]
    fn slop_promotes_to_scroll() {
        let mut r = TouchRecognizer::new();
        r.handle(start(0.0, 0.0));
        let events = r.handle(mv(20.0, 0.0)); // 20px > 8px slop
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], GestureEvent::ScrollStart { .. }));
        match events[1] {
            GestureEvent::ScrollDelta { dx, dy, .. } => {
                assert_eq!(dx, 20.0);
                assert_eq!(dy, 0.0);
            }
            other => panic!("expected ScrollDelta, got {other:?}"),
        }
        assert!(r.is_scrolling());
    }

    #[test]
    fn scroll_emits_deltas_and_ends() {
        let mut r = TouchRecognizer::new();
        r.handle(start(0.0, 0.0));
        r.handle(mv(20.0, 0.0)); // promote
        let mid = r.handle(mv(35.0, 0.0)); // +15
        assert_eq!(mid.len(), 1);
        assert!(matches!(
            mid[0],
            GestureEvent::ScrollDelta {
                dx: 15.0,
                dy: 0.0,
                ..
            }
        ));
        let end_events = r.handle(end(35.0, 0.0));
        // Synchronous timing → velocity (0,0), but Fling is still emitted.
        assert!(end_events.contains(&GestureEvent::ScrollEnd));
        assert!(
            end_events
                .iter()
                .any(|e| matches!(e, GestureEvent::Fling { vx, vy } if *vx == 0.0 && *vy == 0.0))
        );
        assert!(!r.is_scrolling());
    }

    #[test]
    fn cancel_while_pending_is_not_a_tap() {
        let mut r = TouchRecognizer::new();
        r.handle(start(5.0, 5.0));
        r.handle(mv(6.0, 6.0));
        let events = r.handle(TouchSample {
            phase: TouchPhase::Cancel,
            x: 6.0,
            y: 6.0,
            tap_count: 1,
        });
        assert!(events.is_empty());
    }

    #[test]
    fn double_tap_count_survives() {
        let mut r = TouchRecognizer::new();
        r.handle(TouchSample {
            phase: TouchPhase::Start,
            x: 1.0,
            y: 1.0,
            tap_count: 2,
        });
        let events = r.handle(TouchSample {
            phase: TouchPhase::End,
            x: 1.0,
            y: 1.0,
            tap_count: 2,
        });
        match &events[0] {
            GestureEvent::Tap { tap_count, .. } => assert_eq!(*tap_count, 2),
            other => panic!("expected Tap, got {other:?}"),
        }
    }

    #[test]
    fn to_floom_tap_is_down_then_up_at_same_point() {
        let g = GestureEvent::Tap {
            x: 4.0,
            y: 7.0,
            tap_count: 1,
        };
        let events = g.to_floom(Modifiers::NONE);
        assert_eq!(events.len(), 2);
        match (&events[0], &events[1]) {
            (Event::Pointer(d), Event::Pointer(u)) => {
                assert_eq!(d.phase, PointerPhase::Down);
                assert_eq!(u.phase, PointerPhase::Up);
                assert_eq!(d.position, floom_types::Point::new(4.0, 7.0));
                assert_eq!(d.position, u.position);
                assert_eq!(d.kind, PointerKind::Touch);
                assert_eq!(d.click_count, 1);
            }
            other => panic!("expected two pointer events, got {other:?}"),
        }
    }

    #[test]
    fn to_floom_scroll_delta_uses_f64_position() {
        let g = GestureEvent::ScrollDelta {
            dx: -3.0,
            dy: 12.0,
            x: 100.0,
            y: 200.0,
        };
        let events = g.to_floom(Modifiers::NONE);
        match &events[0] {
            Event::Scroll(s) => {
                assert_eq!(s.delta_x, -3.0);
                assert_eq!(s.delta_y, 12.0);
                assert_eq!(s.position, floom_types::Point::new(100.0, 200.0));
            }
            other => panic!("expected Scroll, got {other:?}"),
        }
    }

    #[test]
    fn custom_slop_changes_threshold() {
        let mut r = TouchRecognizer::with_slop(2.0);
        r.handle(start(0.0, 0.0));
        let events = r.handle(mv(5.0, 0.0)); // 5px > 2px slop
        assert!(matches!(events[0], GestureEvent::ScrollStart { .. }));
    }
}
