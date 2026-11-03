//! Per-element repaint requests that travel outside the frame loop.
//!
//! A widget painted by the host cannot hold the host, so it cannot call
//! `frame()` or flip the host's dirty flags directly. What it *can* do is say
//! "this element of mine needs paint again" into a channel the host owns the
//! other end of: [`request_repaint`] enqueues an [`ElementId`], and the host
//! drains the queue at the top of its next [`WidgetHost::frame`] — marking the
//! element dirty through exactly the same path a widget's own `update` uses
//! (`cx.dirty`), so the request costs a repaint and nothing else: no relayout,
//! no semantics rebuild beyond the element's own upsert.
//!
//! This is the repaint request channel: the host is widget-agnostic and knows
//! nothing about signals, so the channel is deliberately dumb — a queue of ids.
//! The reactive
//! half lives with the caller: `Canvas::repaint_on`, for instance, runs a
//! reactive effect that turns a signal ping into a request for its own
//! element, which is how "a signal changed, so redraw my chart" works without
//! the host ever seeing a signal.
//!
//! Two documented properties, both load-bearing:
//!
//! * **Drain-or-nothing.** A request sits in the queue until a host drains
//!   it. An embedder that never calls `frame()` again never repaints — the
//!   channel schedules work for the next frame, it does not conjure one.
//!   [`has_repaint_requests`] exists so a runner keying off
//!   `WidgetHost::needs_frame` can still wake for it.
//! * **Unknown ids vanish.** Requests name elements, and only the host that
//!   minted an id knows whether it is still mounted. A request for an
//!   unmounted element is silently dropped at drain time; a channel that
//!   could fail loudly per id would need the host in the loop that enqueues,
//!   which is the coupling this exists to avoid.

use crate::ElementId;
use std::cell::RefCell;

thread_local! {
    static REQUESTS: RefCell<Vec<ElementId>> = const { RefCell::new(Vec::new()) };
}

/// Asks the host to repaint `id` on its next frame.
///
/// The request is paint-only by construction: the host folds it into the
/// element's dirty flag exactly as if the widget's own `update` had reported
/// dirty, which drives a re-paint and (at most) that element's semantics
/// upsert — never a relayout. Requests for elements the host does not know
/// (unmounted, or minted by a different host) are dropped.
pub fn request_repaint(id: ElementId) {
    REQUESTS.with(|q| q.borrow_mut().push(id));
}

/// Takes every pending request, oldest first. The host calls this once per
/// frame; anything taken is consumed — a request repaints at most one frame.
pub fn take_repaint_requests() -> Vec<ElementId> {
    REQUESTS.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

/// Whether any request is waiting. `false` after a drain — this is what lets
/// a frame loop that polls it settle instead of spinning.
pub fn has_repaint_requests() -> bool {
    REQUESTS.with(|q| !q.borrow().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The queue is process-global, so each test works on a drained channel.
    fn drained<R>(f: impl FnOnce() -> R) -> R {
        take_repaint_requests();
        let r = f();
        take_repaint_requests();
        r
    }

    #[test]
    fn a_request_is_visible_until_drained_and_gone_after() {
        drained(|| {
            assert!(!has_repaint_requests());
            let id = ElementId::new(7, 1);
            request_repaint(id);
            assert!(has_repaint_requests());
            assert_eq!(take_repaint_requests(), vec![id]);
            assert!(!has_repaint_requests());
            // Draining twice yields nothing: a request repaints once.
            assert!(take_repaint_requests().is_empty());
        });
    }

    #[test]
    fn requests_keep_enqueue_order() {
        drained(|| {
            let ids: Vec<ElementId> = (1u32..=4).map(|i| ElementId::new(i, 2)).collect();
            for &id in &ids {
                request_repaint(id);
            }
            assert_eq!(take_repaint_requests(), ids);
        });
    }
}
