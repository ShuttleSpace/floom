//! A1 — the scroll offset reaches absolute coordinates, bounds and hit regions.
//!
//! ## What is asserted here and why it is a separate suite
//!
//! A scroll offset is a post-layout transform on absolute coordinates. It is not
//! a margin (the viewport would grow by the scroll distance), not a relative
//! position (flex is free to recompute it away), and not a paint-time clip
//! adjustment (hit-testing would then disagree with what is drawn). So the whole
//! mechanism lives in one pass of `floe-app` — the absolute-position
//! accumulation — and the observables are:
//!
//! * the scrolled subtree moves by exactly the offset;
//! * the scroll container's own box does **not** move;
//! * hit regions move with the pixels, because a pointer must land where the
//!   thing is drawn;
//! * nested containers compose rather than overwrite each other.
//!
//! The container-without-a-widget at the root stands in for what a `Scroll`
//! widget will be; nothing here depends on that widget existing yet, so the
//! engine can be verified before the widget is written on top of it.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_app::WidgetHost;
use floom_core::{ScrollOffset, Widget as _};
use floom_layout::LayoutStyle;
use floom_types::{Length, Point, Rect, Size};
use floom_widgets::Container;

/// Settle the host: run frames until it stops asking for more.
fn settle(host: &mut WidgetHost) {
    host.frame();
    for _ in 0..12 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert!(!host.needs_frame(), "the host did not settle");
}

fn host_with_children(count: usize) -> (WidgetHost, floom_app::NodeId, Vec<floom_app::NodeId>) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(200.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let children: Vec<_> = (0..count)
        .map(|_| {
            host.mount(
                root,
                LayoutStyle {
                    width: Some(Length::Px(40.0)),
                    height: Some(Length::Px(30.0)),
                    ..LayoutStyle::default()
                },
                Container::new(),
            )
        })
        .collect();
    settle(&mut host);
    (host, root, children)
}

// --- the mechanism ---------------------------------------------------------

/// The control for every test below: an unscrolled child sits where layout put
/// it. Without this, "the child moved by exactly 50" would also be satisfied by
/// a child that started 50 away and never moved.
#[test]
fn an_unscrolled_child_sits_where_layout_put_it() {
    let (host, _, children) = host_with_children(1);
    assert_eq!(
        host.bounds(children[0]),
        Rect::from_origin_size(Point::ZERO, Size::new(40.0, 30.0))
    );
    assert_eq!(host.scroll_offset(host.root().unwrap()), ScrollOffset::ZERO);
}

/// A scroll offset moves the subtree by exactly that offset.
#[test]
fn a_scroll_offset_moves_the_subtree_exactly() {
    let (mut host, root, children) = host_with_children(1);
    let before = host.bounds(children[0]);

    host.set_scroll_offset(root, ScrollOffset::new(0.0, 50.0));
    settle(&mut host);

    let after = host.bounds(children[0]);
    assert_eq!(after.y0 - before.y0, 50.0, "moved down by exactly the offset");
    assert_eq!(after.x0, before.x0, "an axis with no offset does not move");
}

/// **The offset applies to descendants, not to the scroll container itself.** A
/// container whose own box moved with its content would slide out from under
/// the pointer and stop reading as a window.
#[test]
fn the_scroll_container_itself_does_not_move() {
    let (mut host, root, _) = host_with_children(1);
    let before = host.bounds(root);

    host.set_scroll_offset(root, ScrollOffset::new(10.0, 10.0));
    settle(&mut host);

    assert_eq!(host.bounds(root), before, "the frame stays put");
}

/// Hit regions follow the pixels. A pointer that lands where the thing is drawn
/// is not a nicety: without this, a scrolled list would be clickable at the
/// wrong rows while looking correct.
#[test]
fn hit_regions_follow_the_scrolled_pixels() {
    use floom_testkit::TestApp;

    let hit = Arc::new(AtomicUsize::new(0));
    let mut app = TestApp::new(
        Size::new(200.0, 200.0),
        ProbeRoot {
            hit: hit.clone(),
            scroll: Rc::new(Cell::new(ScrollOffset::ZERO)),
        },
    );
    app.frame();

    // The child occupies the top-left, so a click at the origin reaches it.
    app.press(Point::new(10.0, 10.0));
    app.release(Point::new(10.0, 10.0));
    assert_eq!(hit.load(Ordering::SeqCst), 1, "the child is at the origin before scrolling");

    // Scroll it 50px down. A hit region left behind by the scroll would make
    // the list clickable at the wrong rows while looking perfectly correct, so
    // the click has to follow the pixels.
    {
        // Push the offset into the host, which is what a scroll widget does when
        // its own state changes. Storing it in the widget alone would move
        // nothing: the host owns the tree and is the only party that can apply it.
        let node = app.find_widget::<ProbeRoot>().expect("root widget");
        let scroll = app.host_mut().widget::<ProbeRoot>(node).unwrap().scroll.clone();
        scroll.set(ScrollOffset::new(0.0, 50.0));
        app.host_mut().set_scroll_offset(node, ScrollOffset::new(0.0, 50.0));
    }
    app.frame();

    app.press(Point::new(10.0, 10.0));
    app.release(Point::new(10.0, 10.0));
    assert_eq!(
        hit.load(Ordering::SeqCst),
        1,
        "nothing is at the old position any more; child bounds are {:?}",
        app.bounds(app.find_widget::<ClickProbe>().unwrap())
    );

    app.press(Point::new(10.0, 60.0));
    app.release(Point::new(10.0, 60.0));
    assert_eq!(hit.load(Ordering::SeqCst), 2, "and the child is hit where it is drawn");
}

/// A root that owns one clickable child and a scroll offset the test can set.
///
/// The offset is pushed on every frame rather than once: a scroll widget does
/// not know when its content changed size, so re-asserting the offset each
/// frame is the shape the real widget has to take too.
struct ProbeRoot {
    /// The counter the child reports into, cloned into each rebuild.
    hit: Arc<AtomicUsize>,
    scroll: Rc<Cell<ScrollOffset>>,
}

impl ::floom_core::WidgetAny for ProbeRoot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<::floom_core::SemanticsHint> {
        None
    }
}

impl ::floom_core::Widget for ProbeRoot {
    fn create(&self, cx: &mut dyn ::floom_core::CreateContext) -> ::floom_core::ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn ::floom_core::UpdateContext, _id: ::floom_core::ElementId) {}

    fn children(&self) -> Vec<::floom_core::Child> {
        // `children()` returns builders that must be `Fn`, so the child is
        // rebuilt from the shared counter rather than moved out of the root —
        // which is also why the counter is an `Arc` and not a borrow.
        let hit = self.hit.clone();
        vec![::floom_core::Child::new(
            0,
            ::floom_core::LayoutRequest::new().width(40.0).height(30.0),
            move || Box::new(ClickProbe(hit.clone())) as Box<dyn ::floom_core::WidgetAny>,
        )]
    }

    fn paint(&self, _cx: &mut dyn ::floom_core::PaintContext, _id: ::floom_core::ElementId) {}
}

/// A leaf that counts its clicks, so hit-testing can be observed end to end
/// instead of by inspecting the router.
struct ClickProbe(Arc<AtomicUsize>);

impl ::floom_core::Widget for ClickProbe {
    fn create(&self, _cx: &mut dyn ::floom_core::CreateContext) -> ::floom_core::ElementId {
        ::floom_core::ElementId::new(0, 0)
    }

    fn update(&mut self, _cx: &mut dyn ::floom_core::UpdateContext, _id: ::floom_core::ElementId) {}

    fn event(
        &mut self,
        cx: &mut dyn ::floom_core::EventContext,
        _id: ::floom_core::ElementId,
    ) -> bool {
        if matches!(
            cx.event(),
            ::floom_types::Event::Pointer(::floom_types::PointerEvent {
                phase: ::floom_types::PointerPhase::Up,
                ..
            })
        ) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return true;
        }
        false
    }

    fn paint(&self, _cx: &mut dyn ::floom_core::PaintContext, _id: ::floom_core::ElementId) {}
}

impl ::floom_core::WidgetAny for ClickProbe {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<::floom_core::SemanticsHint> {
        None
    }
}

/// Clearing the offset puts everything back. A scroll container that can scroll
/// but not un-scroll is a one-way door.
#[test]
fn clearing_the_offset_restores_the_original_position() {
    let (mut host, root, children) = host_with_children(1);
    let before = host.bounds(children[0]);

    host.set_scroll_offset(root, ScrollOffset::new(0.0, 50.0));
    settle(&mut host);
    assert_ne!(host.bounds(children[0]), before);

    host.set_scroll_offset(root, ScrollOffset::ZERO);
    settle(&mut host);
    assert_eq!(host.bounds(children[0]), before);
}

/// A negative offset scrolls the other way. The sign convention has to hold
/// both ways or a backend cannot simply add its delta.
#[test]
fn a_negative_offset_scrolls_the_other_way() {
    let (mut host, root, children) = host_with_children(1);
    let before = host.bounds(children[0]);

    host.set_scroll_offset(root, ScrollOffset::new(0.0, -20.0));
    settle(&mut host);

    assert_eq!(host.bounds(children[0]).y0 - before.y0, -20.0);
}

/// Two independent scroll containers must not affect each other. This is the
/// property that a single global offset field would break immediately.
#[test]
fn sibling_scroll_containers_are_independent() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(200.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let left = host.mount(
        root,
        LayoutStyle {
            width: Some(Length::Px(80.0)),
            height: Some(Length::Px(200.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let right = host.mount(
        root,
        LayoutStyle {
            width: Some(Length::Px(80.0)),
            height: Some(Length::Px(200.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let in_left = host.mount(
        left,
        LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(30.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let in_right = host.mount(
        right,
        LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(30.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    settle(&mut host);

    let right_before = host.bounds(in_right);
    host.set_scroll_offset(left, ScrollOffset::new(0.0, 40.0));
    settle(&mut host);

    assert_eq!(host.bounds(in_left).y0, 40.0, "the left container scrolled");
    assert_eq!(
        host.bounds(in_right),
        right_before,
        "the right container did not, even though it is a sibling under the same root"
    );
}

/// Nested containers compose: an inner scroll moves its content by both its own
/// offset and its ancestor's. Anything else and the inner container's content
/// would jump out from under its ancestors when the outer one scrolls.
#[test]
fn nested_scroll_containers_compose() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 200.0));
    let outer = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(200.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let inner = host.mount(
        outer,
        LayoutStyle {
            width: Some(Length::Px(80.0)),
            height: Some(Length::Px(100.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let leaf = host.mount(
        inner,
        LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(30.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    settle(&mut host);
    let base = host.bounds(leaf);

    host.set_scroll_offset(outer, ScrollOffset::new(0.0, 10.0));
    host.set_scroll_offset(inner, ScrollOffset::new(0.0, 5.0));
    settle(&mut host);

    assert_eq!(
        host.bounds(leaf).y0 - base.y0,
        15.0,
        "10 from the outer plus 5 from the inner, not one or the other"
    );
}

/// Setting the same offset twice changes nothing — a widget can push its offset
/// every frame without guarding the call.
#[test]
fn setting_the_same_offset_twice_is_idempotent() {
    let (mut host, root, children) = host_with_children(1);
    host.set_scroll_offset(root, ScrollOffset::new(0.0, 30.0));
    settle(&mut host);
    let after_first = host.bounds(children[0]);

    host.set_scroll_offset(root, ScrollOffset::new(0.0, 30.0));
    settle(&mut host);
    assert_eq!(host.bounds(children[0]), after_first);
}

/// The offset a node reports is the one that was set, and reading it back is how
/// a widget confirms its own state survived a reconcile.
#[test]
fn the_offset_can_be_read_back() {
    let (mut host, root, _) = host_with_children(1);
    assert_eq!(host.scroll_offset(root), ScrollOffset::ZERO);
    host.set_scroll_offset(root, ScrollOffset::new(3.0, 4.0));
    assert_eq!(host.scroll_offset(root), ScrollOffset::new(3.0, 4.0));
}
