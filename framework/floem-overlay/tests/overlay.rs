use floem_overlay::{OverlayHandle, OverlayKind, OverlayManager, sheet_bounds};
use floem_types::{ElementId, Point, Rect, Size};

fn eid(i: u32) -> ElementId {
    ElementId::new(i, 1)
}

/// Handles are minted by the host, so the tests do the same thing it does.
fn handle(n: u64) -> OverlayHandle {
    OverlayHandle::new(n)
}

#[test]
fn dialog_popup_bottom_sheet_stacking() {
    let mut om = OverlayManager::new();
    let dialog = om.push(
        handle(0),
        OverlayKind::Dialog,
        eid(1),
        Rect::from_origin_size(Point::new(50.0, 50.0), Size::new(200.0, 120.0)),
        true,
        0.0,
    );
    let popup = om.push(
        handle(1),
        OverlayKind::Popup,
        eid(2),
        Rect::from_origin_size(Point::new(60.0, 60.0), Size::new(80.0, 40.0)),
        false,
        0.0,
    );
    let sheet = om.push(
        handle(2),
        OverlayKind::BottomSheet,
        eid(3),
        sheet_bounds(400.0, 800.0),
        true,
        0.0,
    );
    assert_eq!(om.len(), 3);
    assert_eq!(om.top().unwrap().id, sheet);
    assert!(om.top().unwrap().z > 1);
    assert_eq!(dialog, handle(0));
    assert_eq!(popup, handle(1));
}

#[test]
fn hit_testing_priority_and_modal_scrim() {
    let mut om = OverlayManager::new();
    let dialog = om.push(
        handle(0),
        OverlayKind::Dialog,
        eid(1),
        Rect::from_origin_size(Point::new(100.0, 100.0), Size::new(50.0, 50.0)),
        true,
        0.0,
    );
    // Outside dialog but modal → still hits dialog.
    assert_eq!(om.hit_test(Point::new(10.0, 10.0)), Some(dialog));
    let popup = om.push(
        handle(1),
        OverlayKind::Popup,
        eid(2),
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(20.0, 20.0)),
        false,
        0.0,
    );
    assert_eq!(om.hit_test(Point::new(5.0, 5.0)), Some(popup));
    // Outside popup → falls through to modal dialog scrim
    assert_eq!(om.hit_test(Point::new(200.0, 200.0)), Some(dialog));
}

#[test]
fn focus_moves_to_overlay_and_restores_on_pop() {
    let mut om = OverlayManager::new();
    om.push(
        handle(0),
        OverlayKind::Dialog,
        eid(10),
        Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        true,
        0.0,
    );
    assert_eq!(om.focus.focused(), Some(eid(10)));
    om.push(
        handle(1),
        OverlayKind::Popup,
        eid(20),
        Rect::from_origin_size(Point::ZERO, Size::new(5.0, 5.0)),
        false,
        0.0,
    );
    assert_eq!(om.focus.focused(), Some(eid(20)));
    om.pop();
    assert_eq!(om.focus.focused(), Some(eid(10)));
    om.pop();
    assert!(om.focus.focused().is_none());
}

#[test]
fn animation_ticks_on_overlay() {
    let mut om = OverlayManager::new();
    om.push(
        handle(0),
        OverlayKind::Dialog,
        eid(1),
        Rect::from_origin_size(Point::ZERO, Size::new(1.0, 1.0)),
        false,
        0.0,
    );
    assert!(!om.anim.is_empty());
    om.tick_animations(0.2);
    assert!(om.anim.is_empty());
}

#[test]
fn remove_drops_a_specific_overlay_not_just_the_top() {
    let mut om = OverlayManager::new();
    let a = om.push(
        handle(0),
        OverlayKind::Popup,
        eid(1),
        Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        false,
        0.0,
    );
    let b = om.push(
        handle(1),
        OverlayKind::Dialog,
        eid(2),
        Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        true,
        0.0,
    );
    assert_eq!(om.len(), 2);
    // Drop the bottom overlay; the top one stays and keeps focus.
    assert!(om.remove(a).is_some());
    assert_eq!(om.len(), 1);
    assert_eq!(om.top().unwrap().id, b);
    assert_eq!(om.focus.focused(), Some(eid(2)));
}
