use floem_gesture::{GestureArena, GestureKind, PointerSample, RecognizerState};
use floem_types::{ElementId, Point};

fn eid(i: u32) -> ElementId {
    ElementId::new(i, 1)
}

#[test]
fn tap_wins_without_movement() {
    let mut arena = GestureArena::new();
    let tap = arena.add(GestureKind::Tap, eid(1), 0);
    let drag = arena.add(GestureKind::Drag, eid(1), 0);
    arena.on_pointer(PointerSample::Down {
        position: Point::new(0.0, 0.0),
    });
    arena.on_pointer(PointerSample::Up {
        position: Point::new(1.0, 1.0),
    });
    assert_eq!(arena.owner(), Some(tap));
    assert_eq!(
        arena.recognizer(tap).unwrap().state,
        RecognizerState::Accepted
    );
    assert_eq!(
        arena.recognizer(drag).unwrap().state,
        RecognizerState::Cancelled
    );
}

#[test]
fn drag_cancels_tap_on_move() {
    let mut arena = GestureArena::new();
    let tap = arena.add(GestureKind::Tap, eid(1), 0);
    let drag = arena.add(GestureKind::Drag, eid(1), 10);
    arena.on_pointer(PointerSample::Down {
        position: Point::ZERO,
    });
    arena.on_pointer(PointerSample::Move {
        position: Point::new(20.0, 0.0),
    });
    assert_eq!(arena.owner(), Some(drag));
    assert_eq!(
        arena.recognizer(tap).unwrap().state,
        RecognizerState::Rejected
    );
}

#[test]
fn scroll_beats_drag_by_priority() {
    let mut arena = GestureArena::new();
    let _drag = arena.add(GestureKind::Drag, eid(1), 1);
    let scroll = arena.add(GestureKind::Scroll, eid(2), 5);
    arena.on_pointer(PointerSample::Down {
        position: Point::ZERO,
    });
    arena.on_pointer(PointerSample::Move {
        position: Point::new(0.0, 30.0),
    });
    assert_eq!(arena.owner(), Some(scroll));
}

#[test]
fn scale_competes_then_native_scroll_cancels_app() {
    let mut arena = GestureArena::new();
    let scale = arena.add(GestureKind::Scale, eid(1), 2);
    let native = arena.add(GestureKind::NativeScroll, eid(9), 100);
    arena.on_pointer(PointerSample::Down {
        position: Point::ZERO,
    });
    arena.set_native_scroll_active(true);
    assert_eq!(arena.owner(), Some(native));
    assert_eq!(
        arena.recognizer(scale).unwrap().state,
        RecognizerState::Cancelled
    );
}
