//! `Scroll` -- the wheel, the clamp, and the edges.
//!
//! ## What is worth testing here rather than in the engine
//!
//! The engine's half is covered in `floom-app/tests/scroll.rs`: an offset
//! reaches absolute coordinates, composes down a tree, and moves hit regions.
//! What is left here is the widget's own arithmetic -- where a wheel delta ends
//! up, what happens at the ends, and what happens to an axis the container does
//! not scroll.
//!
//! The composites that use a scroll viewport -- tabs and accordion panels --
//! are covered in `floom-ui-composites/tests/scroll_surfaces.rs`, next to the
//! widgets themselves.

use std::cell::RefCell;
use std::rc::Rc;

use floom_app as Host;
use floom_core as Core;
use floom_core::{ScrollOffset, Size};
use floom_widgets::{Scroll, ScrollAxes};
/// The viewport every scroll test measures into.
const VIEWPORT: (f64, f64) = (200.0, 200.0);

/// A scroll already filled with content of the given extent.
fn scroll_with_content(content: (f64, f64)) -> Scroll {
    Scroll::new().with_content_extent(Size::new(content.0, content.1))
}

#[test]
fn an_unmeasured_scroll_does_not_move() {
    let s = Scroll::new();
    assert_eq!(s.content_extent(), None, "nothing measured yet");
    assert_eq!(
        s.offset_after_wheel((0.0, 100.0), VIEWPORT),
        ScrollOffset::ZERO
    );
}

/// A wheel delta moves the viewport by that much, when there is room.
#[test]
fn the_wheel_moves_the_viewport_by_the_delta() {
    let s = scroll_with_content((200.0, 800.0));
    assert_eq!(
        s.offset_after_wheel((0.0, 100.0), VIEWPORT).y,
        100.0,
        "400 of content below a 200 viewport leaves room for 100"
    );
}

/// Deltas accumulate. Two wheels of 50 are one wheel of 100, which is what makes
/// a trackpad feel continuous rather than stepped.
#[test]
fn successive_deltas_accumulate() {
    let s = scroll_with_content((200.0, 800.0));
    let first = s.offset_after_wheel((0.0, 50.0), VIEWPORT);
    let mut s2 = s;
    assert!(s2.scroll_to(first, VIEWPORT), "the first wheel moved it");
    let second = s2.offset_after_wheel((0.0, 50.0), VIEWPORT);
    assert_eq!(second.y, 100.0);
}

/// Negative deltas scroll back, and are clamped at the top. Clamping only the
/// far end is the usual shortcut and it lets a shrinking list scroll its own
/// first rows out of reach.
#[test]
fn scrolling_back_past_the_top_stops_at_zero() {
    let s = scroll_with_content((200.0, 800.0));
    assert_eq!(
        s.offset_after_wheel((0.0, -50.0), VIEWPORT),
        ScrollOffset::ZERO
    );
}

// --- the clamp -------------------------------------------------------------

/// The offset stops at `content - viewport`, not at the content's own extent.
#[test]
fn the_offset_stops_at_the_end_of_the_content() {
    let s = scroll_with_content((200.0, 800.0));
    assert_eq!(
        s.offset_after_wheel((0.0, 9999.0), VIEWPORT).y,
        600.0,
        "800 of content in a 200 viewport leaves 600 of travel"
    );
}

/// A wheel aimed past the end leaves the viewport exactly at the end, not short
/// of it and not past it. Both of those are the visible bug: short of it and the
/// last row cannot be brought into view, past it and the content has scrolled
/// into empty space.
#[test]
fn a_wheel_past_the_end_lands_exactly_on_it() {
    let s = scroll_with_content((200.0, 800.0));
    let at_end = s.offset_after_wheel((0.0, 9999.0), VIEWPORT);
    assert_eq!(at_end.y, 600.0);
    // And it stays there: another wheel changes nothing.
    let mut s2 = s;
    s2.scroll_to(at_end, VIEWPORT);
    assert_eq!(s2.offset_after_wheel((0.0, 50.0), VIEWPORT).y, 600.0);
    assert!(
        !s2.scroll_to(at_end, VIEWPORT),
        "and asking for the end again reports that nothing moved"
    );
}

/// Content that fits does not scroll at all, in either direction. A container
/// one pixel short of its viewport still does not scroll — the difference is
/// within rounding and scrolling it would leave a gap nobody asked for.
#[test]
fn content_that_fits_does_not_scroll() {
    let s = scroll_with_content((200.0, 200.0));
    assert_eq!(
        s.offset_after_wheel((0.0, 500.0), VIEWPORT),
        ScrollOffset::ZERO
    );
    let shorter = scroll_with_content((200.0, 100.0));
    assert_eq!(
        shorter.offset_after_wheel((0.0, 500.0), VIEWPORT),
        ScrollOffset::ZERO
    );
}

// --- axes ------------------------------------------------------------------

/// **A horizontal-only strip ignores a vertical wheel.** This is the whole
/// reason the flags exist: a row of chips inside a scrolling page must not
/// scroll the page when the wheel rolls over it and nothing is left to scroll
/// sideways.
#[test]
fn a_vertical_wheel_does_not_move_a_horizontal_strip() {
    let s = Scroll::horizontal().with_content_extent(Size::new(800.0_f64, 200.0_f64));
    let after = s.offset_after_wheel((0.0, 500.0), VIEWPORT);
    assert_eq!(after.y, 0.0, "the strip does not scroll vertically");
    assert_eq!(
        after.x, 0.0,
        "and a purely vertical wheel does not scroll it sideways either"
    );
}

/// The same strip scrolls sideways when the wheel does.
#[test]
fn a_horizontal_strip_scrolls_on_the_horizontal_axis() {
    let s = Scroll::horizontal().with_content_extent(Size::new(800.0_f64, 200.0_f64));
    assert_eq!(s.offset_after_wheel((100.0, 0.0), VIEWPORT).x, 100.0);
}

/// A vertical container ignores horizontal deltas — a trackpad swipe sideways
/// should not drift a list.
#[test]
fn a_vertical_container_ignores_horizontal_deltas() {
    let s = scroll_with_content((200.0, 800.0));
    assert_eq!(
        s.offset_after_wheel((500.0, 0.0), VIEWPORT),
        ScrollOffset::ZERO
    );
}

/// A two-axis container scrolls both, and each axis clamps against its own
/// extent. An axis whose content fits must not borrow travel from the other.
#[test]
fn a_two_axis_container_clamps_each_axis_independently() {
    let s = Scroll::both_axes().with_content_extent(Size::new(200.0_f64, 800.0_f64));
    let after = s.offset_after_wheel((500.0, 500.0), VIEWPORT);
    assert_eq!(
        after.x, 0.0,
        "no horizontal overflow, so no horizontal travel"
    );
    assert_eq!(
        after.y, 500.0,
        "a 500 wheel in a 600 of travel lands at 500"
    );
}

// --- reporting -------------------------------------------------------------

/// The reported extent is whatever the host last measured. The widget does not
/// invent one, and does not cache across a `None`.
#[test]
fn the_reported_extent_is_the_measured_one() {
    assert_eq!(Scroll::new().content_extent(), None);
    let s = scroll_with_content((120.0, 400.0));
    assert_eq!(s.content_extent(), Some((120.0, 400.0)));
}

/// The largest offset is reported in terms of the viewport it is given, since
/// that is what it is clamped against.
#[test]
fn the_max_offset_follows_the_viewport() {
    let s = scroll_with_content((200.0, 800.0));
    assert_eq!(s.max_offset(VIEWPORT), Some(ScrollOffset::new(0.0, 600.0)));
    assert_eq!(
        s.max_offset((200.0, 400.0)),
        Some(ScrollOffset::new(0.0, 400.0)),
        "a taller viewport leaves less to travel"
    );
    assert_eq!(
        s.max_offset(VIEWPORT),
        Some(ScrollOffset::new(0.0, 600.0)),
        "and it is unchanged by asking"
    );
}

/// A container is scrollable only when its content actually exceeds it on some
/// axis. A two-axis container whose content is tall and narrow is scrollable;
/// one whose content fits is not.
#[test]
fn scrollability_follows_the_content() {
    assert!(
        !Scroll::new().is_scrollable(),
        "unmeasured is not scrollable"
    );
    assert!(
        !scroll_with_content((200.0, 200.0)).is_scrollable(),
        "fitting content"
    );
    assert!(
        scroll_with_content((200.0, 800.0)).is_scrollable(),
        "taller content"
    );
    assert!(
        scroll_with_content((800.0, 200.0)).is_scrollable(),
        "wider content"
    );
}

/// The axes a container answers to are part of what it is, and survive being
/// passed around by value.
#[test]
fn the_axes_are_preserved() {
    let s = Scroll::new().axes(ScrollAxes {
        horizontal: true,
        vertical: true,
    });
    assert_eq!(
        s.scroll_axes(),
        ScrollAxes {
            horizontal: true,
            vertical: true
        }
    );
    assert_eq!(
        Scroll::new().scroll_axes(),
        ScrollAxes::default(),
        "vertical only by default"
    );
}

// --- in a host -------------------------------------------------------------

/// A scroll container that counts the wheels it consumed and the ones it
/// passed on, so "declines at the end" is observable rather than inferred.
struct ScrollProbe {
    inner: Rc<RefCell<Vec<ProbeEvent>>>,
    /// The probe is a widget, so its offset is a field — a real `Scroll` holds
    /// the same state, and a probe that recomputed from zero every time could
    /// never reach an end and so could never test what happens at one.
    offset: ScrollOffset,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ProbeEvent {
    Consumed(f64),
    PassedOn,
}

impl floom_core::WidgetAny for ScrollProbe {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl floom_core::Widget for ScrollProbe {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn floom_core::UpdateContext, _id: floom_core::ElementId) {}

    fn needs_content_size(&self) -> bool {
        true
    }

    fn clip(&self) -> Option<floom_types::ClipShape> {
        Some(floom_types::ClipShape::Box)
    }

    fn event(&mut self, cx: &mut dyn floom_core::EventContext, id: floom_core::ElementId) -> bool {
        let floom_core::Event::Scroll(delta) = cx.event() else {
            return false;
        };
        let Some(content) = cx.content_size(id) else {
            return false;
        };
        let bounds = cx.bounds();
        let viewport = (bounds.width(), bounds.height());
        // From the current position, not from zero: the wheel adds to where the
        // viewport already is. This is what makes successive wheels accumulate
        // into travel rather than each one resetting to the top.
        let mut scroller = Scroll::new().with_content_extent(content);
        scroller.scroll_to(self.offset, viewport);
        let next = scroller.offset_after_wheel((delta.delta_x, delta.delta_y), viewport);
        let moved = next != self.offset;
        self.offset = next;
        if !moved {
            // At an end: hand the wheel on so a nested scroller can take it.
            self.inner.borrow_mut().push(ProbeEvent::PassedOn);
            return false;
        }
        cx.request_scroll(next);
        self.inner.borrow_mut().push(ProbeEvent::Consumed(next.y));
        true
    }

    fn children(&self) -> Vec<floom_core::Child> {
        vec![floom_core::Child::new(
            0,
            floom_core::LayoutRequest::new().width(200.0).height(900.0),
            || Box::new(Filler) as Box<dyn floom_core::WidgetAny>,
        )]
    }

    fn paint(&self, _cx: &mut dyn floom_core::PaintContext, _id: floom_core::ElementId) {}
}

struct Filler;

impl floom_core::WidgetAny for Filler {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl floom_core::Widget for Filler {
    fn create(&self, _cx: &mut dyn floom_core::CreateContext) -> floom_core::ElementId {
        floom_core::ElementId::new(0, 0)
    }
    fn update(&mut self, _cx: &mut dyn floom_core::UpdateContext, _id: floom_core::ElementId) {}
    fn paint(&self, _cx: &mut dyn floom_core::PaintContext, _id: floom_core::ElementId) {}
}

fn wheel(y: f64) -> floom_core::Event {
    floom_core::Event::Scroll(floom_core::ScrollEvent {
        delta_x: 0.0,
        delta_y: y,
        position: floom_core::Point::new(50.0, 50.0),
        modifiers: floom_core::Modifiers::default(),
    })
}

fn probe_host(viewport: (f64, f64)) -> (Host::WidgetHost, Rc<RefCell<Vec<ProbeEvent>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = Host::WidgetHost::new();
    host.set_size(Size::new(viewport.0, viewport.1));
    // No fixed size on the root, so resizing the viewport resizes the scroll
    // area rather than leaving it pinned at its mount-time geometry.
    host.mount_root(
        floom_layout::LayoutStyle::default(),
        ScrollProbe {
            inner: log.clone(),
            offset: ScrollOffset::ZERO,
        },
    );
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    (host, log)
}

/// A wheel over the viewport scrolls it, and the content really moves.
#[test]
fn a_wheel_scrolls_the_content_in_a_host() {
    let (mut host, log) = probe_host((200.0, 200.0));
    let root = host.root().unwrap();
    let child = host.child_at(root, 0).unwrap();
    let before = host.bounds(child);

    host.dispatch(wheel(120.0));
    host.frame();

    assert_eq!(
        host.bounds(child).y0 - before.y0,
        120.0,
        "the content moved down by the wheel's amount"
    );
    assert_eq!(log.borrow().first(), Some(&ProbeEvent::Consumed(120.0)));
}

/// **The edge.** A container that has run out of travel must let the wheel
/// through, so a scroll container nested inside it can still be reached. This is
/// the case the naive implementation gets wrong: swallowing the wheel at the end
/// makes everything below the end permanently unscrollable.
#[test]
fn a_wheel_at_the_end_is_passed_on_not_swallowed() {
    let (mut host, log) = probe_host((200.0, 200.0));

    // Far more travel than the content has.
    host.dispatch(wheel(99_999.0));
    host.frame();
    assert!(
        matches!(log.borrow().last(), Some(ProbeEvent::Consumed(_))),
        "the first wheel was consumed"
    );

    // Now at the end. Another wheel must not be eaten.
    log.borrow_mut().clear();
    host.dispatch(wheel(120.0));
    host.frame();
    assert_eq!(
        log.borrow().last(),
        Some(&ProbeEvent::PassedOn),
        "at the end the wheel goes to whatever is nested below"
    );
}

/// Content that fits never consumes a wheel at all, so a page behind it scrolls.
#[test]
fn content_that_fits_passes_every_wheel_on() {
    // A viewport taller than the 900-tall child, so there is nowhere to go.
    let (mut host, log) = probe_host((200.0, 2000.0));
    let root = host.root().unwrap();
    assert!(
        host.bounds(root).height() > 900.0,
        "the control: the viewport really is taller than the content"
    );

    host.dispatch(wheel(120.0));
    host.frame();
    assert_eq!(
        log.borrow().last(),
        Some(&ProbeEvent::PassedOn),
        "a 900-tall child in a 2000-tall viewport has nowhere to go"
    );
}

/// A scrolling container clips its content. Without the clip the rows paint
/// over whatever is beside the viewport, which is the one thing a scroll area
/// exists to prevent.
#[test]
fn a_scroll_container_clips_its_content() {
    let scroll = Scroll::new();
    assert_eq!(
        floom_core::Widget::clip(&scroll),
        Some(floom_types::ClipShape::Box),
        "the default container clips to its own box"
    );
}

// --- the real widget in a host -------------------------------------------

/// The same shape as [`ScrollProbe`], but the actual [`Scroll`] rather than a
/// stand-in. The probe proved the engine's offsets work; this proves the widget
/// consumes them, which is a different claim — `Scroll::event` is the first
/// consumer of `EventContext::content_size`, and a widget that never runs is a
/// widget whose arithmetic is never exercised in situ.
fn real_scroll_host(viewport: (f64, f64)) -> (floom_app::WidgetHost, Rc<RefCell<Vec<ProbeEvent>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut host = floom_app::WidgetHost::new();
    host.set_size(Size::new(viewport.0, viewport.1));
    host.mount_root(
        floom_layout::LayoutStyle::default(),
        // The real widget, mounted directly. An earlier version wrapped it in a
        // host widget that re-implemented `needs_content_size` and forwarded
        // `event`; mutations to the real widget survived that, because a wrapper
        // that duplicates the behaviour means the test never exercises it.
        Scroll::new().child(floom_core::Child::new(
            0,
            floom_core::LayoutRequest::new().width(200.0).height(900.0),
            || Box::new(Filler) as Box<dyn floom_core::WidgetAny>,
        )),
    );
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    (host, log)
}

/// **The widget itself, end to end.** A wheel over a real `Scroll` moves its
/// content, which exercises the whole chain at once: `Event::Scroll` routed by
/// position, `EventContext::content_size`, the clamp, and `request_scroll`.
#[test]
fn the_real_scroll_widget_scrolls_its_content() {
    let (mut host, _log) = real_scroll_host((200.0, 200.0));
    let root = host.root().unwrap();
    let child = host.child_at(root, 0).unwrap();
    let before = host.bounds(child);

    host.dispatch(wheel(120.0));
    host.frame();

    assert_eq!(
        host.bounds(child).y0 - before.y0,
        120.0,
        "the content moved by the wheel's amount"
    );
}

/// **The end, on the real widget.** Two wheels that together exceed the
/// available travel leave the viewport exactly at the end, and a third changes
/// nothing — which is the state the widget must report by declining the event.
#[test]
fn the_real_scroll_widget_stops_at_the_end() {
    let (mut host, _log) = real_scroll_host((200.0, 200.0));
    let root = host.root().unwrap();
    let child = host.child_at(root, 0).unwrap();

    host.dispatch(wheel(99_999.0));
    host.frame();
    // 900 of content in a 200 viewport.
    assert_eq!(
        host.bounds(child).y0,
        700.0,
        "exactly at the end, not past it"
    );

    host.dispatch(wheel(120.0));
    host.frame();
    assert_eq!(host.bounds(child).y0, 700.0, "and it stays there");
}

/// The widget refuses a wheel it has nothing to scroll — which is what lets a
/// container inside a non-scrolling page scroll instead of it.
#[test]
fn the_real_scroll_widget_refuses_when_there_is_nothing_to_scroll() {
    // A viewport taller than the 900-tall content.
    let (mut host, _log) = real_scroll_host((200.0, 2000.0));
    let root = host.root().unwrap();
    let child = host.child_at(root, 0).unwrap();
    let before = host.bounds(child);

    host.dispatch(wheel(120.0));
    host.frame();

    assert_eq!(host.bounds(child), before, "nothing moved");
}

/// A widget below the scroll area that counts the wheels that reached it.
///
/// The end case is not observable from bounds: a container at the end consumes
/// a wheel with the *same* offset it already had, so the pixels do not move
/// whether it swallows the gesture or declines it. What differs is what happens
/// next — whether the gesture reaches whatever is underneath. So the assertion
/// needs something underneath to receive it.
#[allow(dead_code)]
struct Downstream {
    wheels: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl floom_core::WidgetAny for Downstream {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl floom_core::Widget for Downstream {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn floom_core::UpdateContext, _id: floom_core::ElementId) {}

    /// The scroll area is this widget's **child**, which is what puts it on the
    /// same event route. Events route along the ancestor chain — capture,
    /// target, bubble — so a sibling would never see a wheel this subtree
    /// declines, and a test built on a sibling would assert nothing.
    fn children(&self) -> Vec<Core::Child> {
        vec![Core::Child::new(
            0,
            floom_core::LayoutRequest::new().width(200.0).height(200.0),
            || {
                Box::new(Scroll::new().child(Core::Child::new(
                    0,
                    floom_core::LayoutRequest::new().width(200.0).height(900.0),
                    || Box::new(Filler) as Box<dyn floom_core::WidgetAny>,
                ))) as Box<dyn floom_core::WidgetAny>
            },
        )]
    }

    /// Counts the wheels that reached it, which is only possible if the scroll
    /// area below declined them.
    fn event(&mut self, cx: &mut dyn floom_core::EventContext, _id: floom_core::ElementId) -> bool {
        if matches!(cx.event(), floom_core::Event::Scroll(_)) {
            self.wheels
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return true;
        }
        false
    }

    fn paint(&self, _cx: &mut dyn floom_core::PaintContext, _id: floom_core::ElementId) {}
}

/// **The end, observed as what it actually affects.** A scroll area at the end
/// of its travel must let the wheel past, so that a scroll container nested
/// inside it — or a page behind it — can still be scrolled.
///
/// This is the case that cannot be tested by looking at pixels: swallowing a
/// wheel at the end re-requests the offset the container already has, so the
/// bounds are identical either way. The difference is entirely in whether the
/// gesture reached anything else.
#[test]
fn a_wheel_at_the_end_is_not_consumed() {
    let mut host = Host::WidgetHost::new();
    host.set_size(Size::new(200.0, 200.0));
    host.mount_root(
        floom_layout::LayoutStyle::default(),
        Scroll::new().child(floom_core::Child::new(
            0,
            floom_core::LayoutRequest::new().width(200.0).height(900.0),
            || Box::new(Filler) as Box<dyn floom_core::WidgetAny>,
        )),
    );
    host.frame();
    for _ in 0..8 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }

    // While there is travel left the container takes the wheel, and the host
    // reports the event as handled.
    assert!(
        host.dispatch(wheel(120.0)),
        "the scroll area consumed a wheel it could act on"
    );
    host.frame();

    // Past the end it has nothing to do, so it must decline rather than
    // swallow. `dispatch` reports whether anything on the route handled the
    // event, and with nothing else in the tree that is exactly the question:
    // a container that returns "handled" at its end is a wall, and everything
    // scrolled inside it — a pane within a page — becomes unreachable.
    host.dispatch(wheel(99_999.0));
    host.frame();
    assert!(
        !host.dispatch(wheel(120.0)),
        "at the end the wheel reached nothing, so whatever is below can scroll"
    );
}
