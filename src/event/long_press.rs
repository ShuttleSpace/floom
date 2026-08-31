//! Touch long-press gesture tracking for opening the in-UI context menu.
//!
//! On touch devices there is no secondary (right) button, so the
//! [`crate::event::dispatch`] context-menu path (which only fires on
//! `PointerButton::Secondary`) never triggers. This module implements the
//! Android-friendly long-press gesture: a touch pointer held stationary for a
//! short time opens the context menu of the view under the press.
//!
//! The tracker follows the same "pending → fire" shape as
//! [`crate::event::drag_state::DragTracker`]: it records the press, schedules a
//! timer, and cancels it if the pointer moves (a scroll/drag) or lifts early.

use std::{rc::Rc, time::Duration};

use peniko::kurbo::Point;
use ui_events::pointer::{
    PointerButton, PointerButtonEvent, PointerId, PointerInfo, PointerType, PointerUpdate,
};

use crate::{
    ElementId,
    action::{TimerToken, exec_after, show_context_menu_internal},
    context::MenuCallback,
};

/// How long a touch pointer must remain (roughly) stationary before the
/// long-press context menu is triggered. 500ms matches the Android framework
/// default for a long-press.
const LONG_PRESS_DURATION_MS: u64 = 500;

/// Maximum movement (logical px) allowed before a pending long-press is
/// cancelled and treated as a scroll/drag instead.
const LONG_PRESS_MOVE_THRESHOLD: f64 = 10.0;

/// Per-window long-press gesture state. Only one long-press can be pending at a
/// time per window (the most recent touch press wins).
pub(crate) struct LongPressState {
    /// Pointer currently being tracked for a long-press, if any.
    pointer_id: Option<PointerId>,
    /// Logical position where the press began (used for the move threshold and
    /// as the menu anchor if the view has no explicit position).
    start: Point,
    /// Hit element at press time whose context menu we intend to show.
    element: Option<ElementId>,
    /// Scheduled long-press timer; cancelled if the gesture ends early.
    timer: Option<TimerToken>,
}

impl Default for LongPressState {
    fn default() -> Self {
        Self::new()
    }
}

impl LongPressState {
    pub fn new() -> Self {
        Self {
            pointer_id: None,
            start: Point::ZERO,
            element: None,
            timer: None,
        }
    }

    /// Begin tracking a potential long-press for a touch primary press.
    ///
    /// Does nothing unless `pbe` is a touch primary press over a view that
    /// actually has a context menu (`menu_cb.is_some()`). Any previously pending
    /// long-press is cancelled first, so a new press always wins.
    pub fn on_pointer_down(
        &mut self,
        pbe: &PointerButtonEvent,
        element: Option<ElementId>,
        menu_cb: Option<Rc<MenuCallback>>,
    ) {
        // A new press supersedes any pending long-press.
        self.cancel();

        // Only touch primary presses trigger a long-press menu. Mouse/pen
        // secondary presses already open the menu via the Secondary-button path.
        if pbe.pointer.pointer_type != PointerType::Touch {
            return;
        }
        if pbe.button != Some(PointerButton::Primary) {
            return;
        }
        let Some(pointer_id) = pbe.pointer.pointer_id else {
            return;
        };
        let Some(menu_cb) = menu_cb else {
            return;
        };

        let start = pbe.state.logical_point();

        // Capture what we need to open the menu when the timer fires. We clone
        // the menu callback (an `Rc`) rather than re-resolving the view state
        // later, so the menu is stable even if the tree changes before firing.
        let pos = start;
        let token = exec_after(Duration::from_millis(LONG_PRESS_DURATION_MS), move |_| {
            show_context_menu_internal(menu_cb(), Some(pos), true);
        });

        self.pointer_id = Some(pointer_id);
        self.start = start;
        self.element = element;
        self.timer = Some(token);
    }

    /// Cancel a pending long-press if the pointer moves beyond the threshold
    /// (treated as a scroll/drag). Only the tracked pointer is considered.
    pub fn on_pointer_move(&mut self, pu: &PointerUpdate) {
        let Some(_timer) = self.timer else {
            return;
        };
        if pu.pointer.pointer_id != self.pointer_id {
            return;
        }
        let current = pu.current.logical_point();
        if (current - self.start).length() > LONG_PRESS_MOVE_THRESHOLD {
            self.cancel();
        }
    }

    /// Cancel a pending long-press when the tracked pointer lifts. A finger that
    /// lifts before the timeout should not open a menu.
    pub fn on_pointer_up(&mut self, pe: &PointerButtonEvent) {
        if pe.pointer.pointer_id == self.pointer_id {
            self.cancel();
        }
    }

    /// Cancel a pending long-press when the tracked pointer is cancelled.
    pub fn on_pointer_cancel(&mut self, pi: PointerInfo) {
        if pi.pointer_id == self.pointer_id {
            self.cancel();
        }
    }

    /// Cancel any pending long-press and release its timer.
    pub fn cancel(&mut self) {
        if let Some(token) = self.timer.take() {
            token.cancel();
        }
        self.pointer_id = None;
        self.start = Point::ZERO;
        self.element = None;
    }
}
