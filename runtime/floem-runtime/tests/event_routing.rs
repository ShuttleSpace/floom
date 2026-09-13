use floem_runtime::{EventPhase, EventRouter, HitRegion, PointerCapture};
use floem_types::{ElementId, NativeViewId, Point, Rect};

fn eid(i: u32) -> ElementId {
    ElementId::new(i, 1)
}

#[test]
fn hit_test_picks_frontmost_z() {
    let mut router = EventRouter::new();
    router.hits.set_region(
        eid(1),
        HitRegion {
            bounds: Rect::from_origin_size(Point::ZERO, floem_types::Size::new(100.0, 100.0)),
            native: None,
            z: 0,
        },
    );
    router.hits.set_region(
        eid(2),
        HitRegion {
            bounds: Rect::from_origin_size(
                Point::new(10.0, 10.0),
                floem_types::Size::new(20.0, 20.0),
            ),
            native: None,
            z: 1,
        },
    );
    let hit = router.hits.hit_test(Point::new(15.0, 15.0)).unwrap();
    assert_eq!(hit.target, eid(2));
}

#[test]
fn native_region_wins_same_z() {
    let mut router = EventRouter::new();
    router.hits.set_region(
        eid(1),
        HitRegion {
            bounds: Rect::from_origin_size(Point::ZERO, floem_types::Size::new(50.0, 50.0)),
            native: None,
            z: 5,
        },
    );
    router.hits.set_region(
        eid(2),
        HitRegion {
            bounds: Rect::from_origin_size(Point::ZERO, floem_types::Size::new(50.0, 50.0)),
            native: Some(NativeViewId::new(1, 1)),
            z: 5,
        },
    );
    let hit = router.hits.hit_test(Point::new(10.0, 10.0)).unwrap();
    assert_eq!(hit.target, eid(2));
    assert!(hit.native.is_some());
}

#[test]
fn capture_bubble_route_order() {
    let mut router = EventRouter::new();
    // root(1) → mid(2) → leaf(3)
    router.set_parent(eid(2), eid(1));
    router.set_parent(eid(3), eid(2));
    let route = router.route(eid(3));
    let phases: Vec<_> = route.iter().map(|s| (s.id.index(), s.phase)).collect();
    assert_eq!(
        phases,
        vec![
            (1, EventPhase::Capture),
            (2, EventPhase::Capture),
            (3, EventPhase::Target),
            (2, EventPhase::Bubble),
            (1, EventPhase::Bubble),
        ]
    );
}

#[test]
fn pointer_capture_overrides_hit() {
    let mut router = EventRouter::new();
    router.hits.set_region(
        eid(1),
        HitRegion {
            bounds: Rect::from_origin_size(Point::ZERO, floem_types::Size::new(10.0, 10.0)),
            native: None,
            z: 0,
        },
    );
    router.hits.set_region(
        eid(9),
        HitRegion {
            bounds: Rect::from_origin_size(
                Point::new(100.0, 100.0),
                floem_types::Size::new(10.0, 10.0),
            ),
            native: None,
            z: 99,
        },
    );
    router.capture.capture(eid(1));
    let hit = router.resolve_target(Point::new(105.0, 105.0)).unwrap();
    assert_eq!(hit.target, eid(1));
    router.capture.release();
    let hit = router.resolve_target(Point::new(105.0, 105.0)).unwrap();
    assert_eq!(hit.target, eid(9));
}

#[test]
fn pointer_capture_api() {
    let mut cap = PointerCapture::default();
    assert!(cap.get().is_none());
    cap.capture(eid(3));
    assert_eq!(cap.get(), Some(eid(3)));
    cap.release();
    assert!(cap.get().is_none());
}

#[test]
fn unparenting_stops_the_removed_node_from_bubbling() {
    let mut router = EventRouter::new();
    let root = eid(1);
    let child = eid(2);
    router.set_parent(child, root);
    assert!(router.route(child).iter().any(|stop| stop.id == root));

    router.unparent(child);

    // Still routable as its own target, but its ex-ancestors are gone.
    let route = router.route(child);
    assert_eq!(route.len(), 1, "no ancestors remain in the route");
    assert_eq!(route[0].id, child);
    assert_eq!(route[0].phase, EventPhase::Target);
}

#[test]
fn unmounting_a_captured_node_releases_the_capture() {
    let mut router = EventRouter::new();
    let dragged = eid(7);
    router.hits.set_region(
        dragged,
        HitRegion {
            bounds: Rect::from_origin_size(
                Point::new(0.0, 0.0),
                floem_types::Size::new(10.0, 10.0),
            ),
            native: None,
            z: 0,
        },
    );
    router.capture.capture(dragged);
    // While captured, every pointer resolves to it regardless of position.
    let far = Point::new(500.0, 500.0);
    assert_eq!(router.resolve_target(far).map(|h| h.target), Some(dragged));

    // The row disappears mid-drag.
    router.unparent(dragged);

    assert_eq!(router.capture.get(), None, "capture must die with its node");
    assert_eq!(
        router.resolve_target(far).map(|h| h.target),
        None,
        "input falls back to plain hit-testing instead of sticking to a dead target"
    );
}
