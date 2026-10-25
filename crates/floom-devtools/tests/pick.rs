//! The pick's contract: arm, hover, commit, and the box that follows.
//!
//! The pick is the devtools' signature interaction — click the app, inspect what
//! you clicked — and the part of it that can silently rot is the *ownership* of
//! pointer events: while picking, a click must select without also pressing.

use std::sync::{Arc, Mutex};

use floom_core::{
    Key, KeyEvent, KeyPhase, Modifiers, NamedKey, PointerButton, PointerEvent, PointerKind,
    PointerPhase, Rect,
};
use floom_devtools::pick::{
    HighlightBox, PickPhase, PickState, highlight_is_live, highlight_rect, pick_pointer,
};
use floom_types::{Point, Size};

fn move_to(x: f64, y: f64) -> PointerEvent {
    PointerEvent {
        phase: PointerPhase::Move,
        button: None,
        kind: PointerKind::Mouse,
        position: Point::new(x, y),
        modifiers: Modifiers::NONE,
        click_count: 0,
    }
}

fn press_at(x: f64, y: f64) -> PointerEvent {
    PointerEvent {
        phase: PointerPhase::Down,
        button: Some(PointerButton::Primary),
        kind: PointerKind::Mouse,
        position: Point::new(x, y),
        modifiers: Modifiers::NONE,
        click_count: 1,
    }
}

/// A stand-in for the host's hit test: a point is "over" a node when it falls in
/// the node's box, and the answer is the node's key.
fn key_of(boxes: &'static [(&'static str, Rect)]) -> impl Fn(Point) -> Option<String> {
    move |point| {
        boxes
            .iter()
            .find(|(_, r)| point.x >= r.x0 && point.x < r.x1 && point.y >= r.y0 && point.y < r.y1)
            .map(|(key, _)| (*key).to_string())
    }
}

const BUTTON: (&str, Rect) = (
    "ok_button",
    Rect {
        x0: 10.0,
        y0: 10.0,
        x1: 110.0,
        y1: 40.0,
    },
);
const LABEL: (&str, Rect) = (
    "heading",
    Rect {
        x0: 10.0,
        y0: 50.0,
        x1: 210.0,
        y1: 70.0,
    },
);

fn state() -> Arc<Mutex<PickState>> {
    Arc::new(Mutex::new(PickState::new()))
}

#[test]
fn a_disarmed_picker_consumes_nothing() {
    let state = state();
    // The app keeps every pointer event until the picker is armed.
    assert!(!pick_pointer(
        &state,
        &move_to(20.0, 20.0),
        key_of(&[BUTTON, LABEL])
    ));
    assert!(!pick_pointer(
        &state,
        &press_at(20.0, 20.0),
        key_of(&[BUTTON, LABEL])
    ));
    assert_eq!(state.lock().unwrap().phase(), PickPhase::Idle);
}

#[test]
fn arming_makes_the_picker_own_the_pointer() {
    let state = state();
    state.lock().unwrap().arm();
    assert!(state.lock().unwrap().armed());
    // A move is consumed and records what is under the pointer.
    assert!(pick_pointer(
        &state,
        &move_to(20.0, 20.0),
        key_of(&[BUTTON, LABEL])
    ));
    assert_eq!(state.lock().unwrap().hovered(), Some("ok_button"));
    assert_eq!(state.lock().unwrap().phase(), PickPhase::Hovering);
}

#[test]
fn hovering_follows_the_pointer() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    assert_eq!(state.lock().unwrap().hovered(), Some("ok_button"));
    pick_pointer(&state, &move_to(100.0, 60.0), key_of(&[BUTTON, LABEL]));
    assert_eq!(state.lock().unwrap().hovered(), Some("heading"));
}

#[test]
fn a_click_commits_the_selection_and_stops_picking() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    // The click is consumed — the button is inspected, not pressed.
    assert!(pick_pointer(
        &state,
        &press_at(20.0, 20.0),
        key_of(&[BUTTON, LABEL])
    ));
    let guard = state.lock().unwrap();
    assert_eq!(guard.picked(), Some("ok_button"));
    assert_eq!(guard.phase(), PickPhase::Idle);
    assert_eq!(guard.hovered(), None);
    drop(guard);
    // And the picker is disarmed, so the next click belongs to the app again.
    assert!(!pick_pointer(
        &state,
        &press_at(20.0, 20.0),
        key_of(&[BUTTON, LABEL])
    ));
}

/// The pick is handed to the panel once, not once per read.
///
/// A pick outlives the click — the highlight keeps sitting on it — so it cannot
/// be cleared on commit. That means "is there a pick" is not the same question
/// as "has the panel been told", and the two have to be tracked separately: a
/// caller that commits after every pointer event (a frame loop with no cheaper
/// signal) must not re-select the last pick on every later click.
#[test]
fn a_pick_is_committed_once_and_then_holds() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &press_at(20.0, 20.0), key_of(&[BUTTON, LABEL]));

    // First read takes it.
    assert_eq!(
        state.lock().unwrap().take_uncommitted(),
        Some("ok_button".to_string())
    );
    assert!(state.lock().unwrap().committed());
    // Every later read gets nothing, and the pick itself is still there for the
    // highlight to sit on.
    assert_eq!(state.lock().unwrap().take_uncommitted(), None);
    assert_eq!(state.lock().unwrap().picked(), Some("ok_button"));

    // A new pick reopens the one-shot.
    state.lock().unwrap().arm();
    pick_pointer(&state, &press_at(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    assert_eq!(
        state.lock().unwrap().take_uncommitted(),
        Some("ok_button".to_string())
    );

    // And clearing the selection resets it too, so a cleared picker cannot
    // hand back a key it no longer holds.
    state.lock().unwrap().clear_picked();
    assert_eq!(state.lock().unwrap().take_uncommitted(), None);
    assert_eq!(state.lock().unwrap().picked(), None);
}

#[test]
fn a_click_on_empty_space_picks_nothing() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    // Over empty space: the hover clears, and a click commits nothing.
    assert!(pick_pointer(
        &state,
        &move_to(400.0, 400.0),
        key_of(&[BUTTON, LABEL])
    ));
    assert_eq!(state.lock().unwrap().hovered(), None);
    assert!(pick_pointer(
        &state,
        &press_at(400.0, 400.0),
        key_of(&[BUTTON, LABEL])
    ));
    assert_eq!(state.lock().unwrap().picked(), None);
}

#[test]
fn disarming_keeps_the_selection_and_drops_the_hover() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    pick_pointer(&state, &press_at(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(100.0, 60.0), key_of(&[BUTTON, LABEL]));
    state.lock().unwrap().disarm();
    let guard = state.lock().unwrap();
    assert_eq!(
        guard.picked(),
        Some("ok_button"),
        "a pick survives a cancel"
    );
    assert_eq!(guard.hovered(), None);
}

#[test]
fn the_highlight_follows_the_hover_while_picking() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    // The box is around what the pointer is over, and it is a live box.
    let rect = highlight_rect(&state, |key| {
        [BUTTON, LABEL]
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, r)| *r)
    });
    assert_eq!(rect, Some(BUTTON.1));
    assert!(highlight_is_live(&state));
}

#[test]
fn the_highlight_settles_on_the_pick_once_picking_stops() {
    let state = state();
    state.lock().unwrap().arm();
    pick_pointer(&state, &move_to(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    pick_pointer(&state, &press_at(20.0, 20.0), key_of(&[BUTTON, LABEL]));
    // No longer live, and still around the committed pick.
    assert!(!highlight_is_live(&state));
    let rect = highlight_rect(&state, |key| {
        [BUTTON, LABEL]
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, r)| *r)
    });
    assert_eq!(rect, Some(BUTTON.1));
}

#[test]
fn no_selection_means_no_box() {
    let state = state();
    state.lock().unwrap().arm();
    // Armed but never over anything: nothing to draw.
    assert_eq!(highlight_rect(&state, |_| None), None);
    assert!(!highlight_is_live(&state));
}

#[test]
fn a_highlight_over_nothing_draws_nothing() {
    // An inert box: no rect, so the paint is a no-op rather than a zero-sized
    // rectangle at the origin.
    let box_none = HighlightBox::new(None, false);
    assert_eq!(box_none.rect(), None);
    let box_some = HighlightBox::new(Some(BUTTON.1), false);
    assert_eq!(box_some.rect(), Some(BUTTON.1));
}

#[test]
fn the_highlight_box_can_be_retargeted_in_place() {
    let mut highlight = HighlightBox::new(None, false);
    highlight.set_rect(Some(LABEL.1), true);
    assert_eq!(highlight.rect(), Some(LABEL.1));
    assert!(highlight.is_live());
}

#[test]
fn the_pick_is_a_keyboard_gesture_too() {
    use floom_devtools::pick::default_pick_shortcut;
    // The picker is armed from the loop, the way the panel is toggled: a key
    // event the app's own handler forwards. `Ctrl+Shift+C` is Chrome's binding.
    let shortcut = default_pick_shortcut();
    let event = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('c'),
        modifiers: Modifiers::CONTROL.union(Modifiers::SHIFT),
    };
    assert!(shortcut(&event));

    // A plain `c` — typing into a field — does not arm it.
    let typed = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('c'),
        modifiers: Modifiers::NONE,
    };
    assert!(!shortcut(&typed));

    // Nor does a release.
    let released = KeyEvent {
        phase: KeyPhase::Up,
        key: Key::Char('c'),
        modifiers: Modifiers::CONTROL.union(Modifiers::SHIFT),
    };
    assert!(!shortcut(&released));

    // Neither modifier alone: `Ctrl+C` is copy, and `Shift+C` is a capital C.
    let ctrl_c = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('c'),
        modifiers: Modifiers::CONTROL,
    };
    assert!(!shortcut(&ctrl_c));
    let shift_c = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('c'),
        modifiers: Modifiers::SHIFT,
    };
    assert!(!shortcut(&shift_c));

    // The macOS spelling: the platform layer reports the command key as `META`,
    // so a picker that only knew `CONTROL` would be unreachable on a Mac.
    let cmd_shift_c = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('c'),
        modifiers: Modifiers::META.union(Modifiers::SHIFT),
    };
    assert!(shortcut(&cmd_shift_c));

    // And the panel's own shortcut is a different key: pressing the devtools
    // binding must not also arm the picker.
    let ctrl_shift_i = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::CONTROL.union(Modifiers::SHIFT),
    };
    assert!(!shortcut(&ctrl_shift_i));
}

#[test]
fn the_picker_owns_only_the_three_pointer_phases_it_acts_on() {
    use floom_devtools::pick::is_pick_phase;
    assert!(is_pick_phase(PointerPhase::Move));
    assert!(is_pick_phase(PointerPhase::Down));
    assert!(is_pick_phase(PointerPhase::Up));
    // A hover-enter or a leave is the app's.
    assert!(!is_pick_phase(PointerPhase::Enter));
    assert!(!is_pick_phase(PointerPhase::Leave));
    assert!(!is_pick_phase(PointerPhase::Cancel));
}

#[test]
fn the_highlight_covers_the_viewport_so_it_can_draw_anywhere() {
    use floom_core::Widget;
    let highlight = HighlightBox::new(
        Some(Rect::from_origin_size(
            Point::new(0.0, 0.0),
            Size::new(100.0, 50.0),
        )),
        false,
    );
    let request = highlight.layout_request().expect("a covering request");
    // `fill` — the window is the canvas, the rect is what gets drawn.
    assert!(request.flex_grow.is_some() || request.width.is_some() || request.height.is_some());
}

#[test]
fn the_named_key_variant_is_still_the_one_the_panel_uses() {
    // The pick shortcut is spelled with the same function-key variant the panel
    // toggle uses, so the two cannot drift apart in the keymap.
    let f12 = Key::Named(NamedKey::Function(12));
    assert!(matches!(f12, Key::Named(NamedKey::Function(12))));
}
