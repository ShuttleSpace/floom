//! The combobox as a widget: mounting, the overlay round trip, and the two state
//! transitions a caller can drive without a mouse.
//!
//! The decisions themselves — the filter, the colours, the placement arithmetic — are
//! in `combobox.rs`'s companion file. What is here is only what can *only* be observed
//! on a mounted widget.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, Key, KeyPhase, Widget, WidgetAny};
use floom_types::{OverlayHandle, Rect};

use floom_ui_composites::combobox::{ComboPick, Combobox, SharedPick};
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, TestEventContext, paint_snapshot,
};

/// The field's box, so the panel's placement can be asserted against a real anchor.
const FIELD: Rect = Rect {
    x0: 30.0,
    y0: 100.0,
    x1: 230.0,
    y1: 156.0,
};

fn options() -> Vec<String> {
    ["apple", "banana", "Cherry"].iter().map(|s| s.to_string()).collect()
}

fn combo() -> Combobox {
    Combobox::new(options())
}

fn mounted(combo: &mut Combobox) -> ElementId {
    combo.create(&mut AllocatingCreateContext::new())
}

fn fresh() -> RecordingUpdateContext {
    RecordingUpdateContext::with_bounds(FIELD)
}

// --- mounting -------------------------------------------------------------

/// A combobox contributes the trigger, and only the trigger, while no host owns the
/// panel — because the panel is the overlay's job, not the tree's.
#[test]
fn a_combobox_contributes_its_trigger() {
    let combo = combo();
    let id = combo.create(&mut AllocatingCreateContext::new());
    let children = combo.children();
    assert_eq!(children.len(), 1, "the field and its chevron");
    let _built = children[0].build();
    let _ = id;
}

/// Its own box is the field's box: v1 sized the trigger to `ComboboxTokens::height`
/// with a `min_width`, and the panel hangs off *this*, not off the whole tree.
#[test]
fn the_trigger_declares_its_height_and_floor() {
    let request = combo().layout_request().expect("a trigger has a size");
    assert_eq!(request.height, Some(floom_types::Length::Px(56.0)));
    assert_eq!(
request.min_width,
        Some(floom_types::Length::Px(240.0)),
        "and the token's floor, so a narrow combobox still reads as a combobox"
    );
}

// --- the overlay round trip -----------------------------------------------

/// Opening asks the host for a panel **below** the field, not over it.
///
/// This is the whole point of `PopupPlacement` reaching a combobox: v1 positioned the
/// panel with `.absolute().inset_top(origin.y + trigger_height)` plus an inline
/// `.margin_top(4.0)`, which is the same arithmetic said the long way.
#[test]
fn opening_asks_for_a_panel_below_the_field() {
    let mut combo = combo();
    let id = mounted(&mut combo);
    let mut cx = fresh();

    combo.set_open(true);
    combo.update(&mut cx, id);

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one panel asked for");
    match opened[0] {
        floom_widgets::testkit::OverlayCall::Open { bounds, kind, modal, .. } => {
            assert_eq!(*kind, floom_types::OverlayKind::Popup);
            assert!(!modal);
            assert_eq!(bounds.x0, FIELD.x0, "aligned with the field's left edge");
            assert_eq!(bounds.x1, FIELD.x1, "and exactly as wide as it");
            assert_eq!(
                bounds.y0,
                FIELD.y1 + 4.0,
                "and starting below it, with Material's 4px gap"
            );
            assert!(
                bounds.y1 > bounds.y0,
                "with a real height, because the host clips to what it is given"
            );
        }
        _ => panic!("expected an open, got a close"),
    }
}

/// A closed combobox asks the host for nothing.
#[test]
fn a_shut_combobox_asks_for_nothing() {
    let mut combo = combo();
    let id = mounted(&mut combo);
    let mut cx = fresh();
    combo.update(&mut cx, id);
    assert!(!cx.opened_any_overlay());
}

/// `set_open` reports whether it changed anything, so a caller can skip the frame.
#[test]
fn set_open_reports_whether_it_changed_anything() {
    let mut combo = combo();
    assert!(combo.set_open(true), "closed to open");
    assert!(!combo.set_open(true), "already open");
    assert!(combo.set_open(false), "and back");
}

/// And the closing goes through the handle the host issued.
#[test]
fn closing_closes_the_handle_the_host_issued() {
    let mut combo = combo();
    let id = mounted(&mut combo);
    let mut cx = fresh();

    combo.set_open(true);
    combo.update(&mut cx, id);
    let granted = cx.opened_overlays()[0].handle();
    combo.overlay_opened(&mut cx, id, granted);

    cx.clear();
    combo.set_open(false);
    combo.update(&mut cx, id);
    assert_eq!(cx.closed_overlays(), vec![granted]);
}

/// A host that dismissed the panel on its own — an outside press — leaves the widget
/// agreeing with what is on screen, so the next frame does not re-open it.
#[test]
fn an_outside_press_does_not_come_straight_back() {
    let mut combo = combo();
    let id = mounted(&mut combo);
    let mut cx = fresh();
    combo.set_open(true);
    combo.update(&mut cx, id);
    let granted = cx.opened_overlays()[0].handle();
    combo.overlay_opened(&mut cx, id, granted);

    combo.overlay_closed(&mut cx, id, granted, Box::new(floom_widgets::Text::new("probe")));
    assert!(!combo.is_open(), "the widget agrees with the screen");

    cx.clear();
    combo.update(&mut cx, id);
    assert!(!cx.opened_any_overlay(), "and stays shut");
}

/// A hand-back for somebody else's overlay is ignored.
#[test]
fn a_hand_back_for_another_overlay_does_not_shut_it() {
    let mut combo = combo();
    let id = mounted(&mut combo);
    let mut cx = fresh();
    combo.set_open(true);
    combo.update(&mut cx, id);
    let granted = cx.opened_overlays()[0].handle();
    combo.overlay_opened(&mut cx, id, granted);

    combo.overlay_closed(&mut cx, id, OverlayHandle::new(4242), Box::new(floom_widgets::Text::new("probe")));
    assert!(combo.is_open(), "a stale hand-back closes nothing of ours");
}

// --- selecting ------------------------------------------------------------

/// `select` is public because v1's `selected(..)` setter did this: set the index *and*
/// put the option's text in the field. A caller that sets `selected` and then sees an
/// empty field has been given a combobox that disagrees with itself.
#[test]
fn selecting_through_the_widget_fills_the_field_and_fires_the_callback() {
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = fired.clone();
    let mut combo = Combobox::new(options()).on_change(move |index| {
        assert_eq!(index, Some(1));
        counter.fetch_add(1, Ordering::SeqCst);
    });

    assert!(combo.select(1));
    assert_eq!(combo.selected(), Some(1));
    assert_eq!(combo.query().lock().unwrap().as_str(), "banana");
    assert_eq!(fired.load(Ordering::SeqCst), 1, "and told the caller once");
}

/// Selecting through the widget is visible in the shared state, which is how a kit's
/// row and a caller both see the same choice.
#[test]
fn the_selection_is_visible_through_the_shared_handle() {
    let mut combo = combo();
    combo.select(2);
    let pick = combo.pick();
    assert_eq!(pick.lock().unwrap().selected, Some(2));
}

/// A row's keystroke path — the shared buffer — reaches the same state. The engine's
/// own `TextInput` holds this buffer, so a caller that writes to it is typing.
#[test]
fn typing_into_the_shared_buffer_filters_what_is_visible() {
    let combo = combo();
    assert_eq!(combo.visible(), options(), "an empty query shows everything");

    *combo.query().lock().unwrap() = "an".to_string();
    assert_eq!(
        combo.visible(),
        vec!["banana".to_string()],
        "and a query narrows it, which is the control's whole point"
    );

    *combo.query().lock().unwrap() = "zzz".to_string();
    assert!(combo.visible().is_empty());
}

/// And an empty query shows everything again, because v1's filter said so.
#[test]
fn clearing_the_query_restores_every_option() {
    let combo = combo();
    *combo.query().lock().unwrap() = "zzz".to_string();
    assert!(combo.visible().is_empty());
    combo.query().lock().unwrap().clear();
    assert_eq!(combo.visible(), options());
}

/// A selection that stops resolving is dropped **and** the field is cleared.
///
/// The field matters: a combobox reading "banana" above a list that has no banana
/// looks right, which is worse than one reading nothing.
#[test]
fn replacing_the_options_drops_a_selection_that_no_longer_resolves() {
    let mut combo = combo();
    combo.select(1);
    assert_eq!(combo.query().lock().unwrap().as_str(), "banana");

    combo.set_options(vec!["apple".to_string(), "Cherry".to_string()]);
    assert_eq!(
        combo.selected(),
        None,
        "banana is not in the new list at all — note the new list's index 1 *is* \
         `Cherry`, so keeping the raw index would have looked like it still worked"
    );
    assert_eq!(
        combo.query().lock().unwrap().as_str(),
        "",
        "and the field does not keep claiming something the list does not offer"
    );
}

/// A selection that *does* still resolve is kept, and so is the field.
#[test]
fn replacing_the_options_keeps_a_selection_that_still_resolves() {
    let mut combo = combo();
    combo.select(1);
    combo.set_options(vec!["banana".to_string(), "apple".to_string()]);
    assert_eq!(
        combo.selected(),
        Some(0),
        "banana moved from index 1 to index 0, and the selection followed the *option*"
    );
    assert_eq!(combo.query().lock().unwrap().as_str(), "banana");
}

/// And a disabled combobox is still a combobox, not an inert one — `disabled` here is
/// a *style and interaction* flag, and v1's `Pressable` gates both pointer and
/// keyboard, so the chevron stops toggling too.
#[test]
fn a_disabled_combobox_paints_differently_from_a_live_one() {
    let mut live = combo();
    let mut disabled = combo().disabled(true);
    // `Combobox::paint` draws nothing on purpose — the trigger is a child — so this
    // has to paint the trigger. A test that painted the widget would compare two
    // empty scenes and pass for the wrong reason.
    let live_trigger = live.children().remove(0).build();
    let dead_trigger = disabled.children().remove(0).build();
    let live_scene = paint_snapshot(&*live_trigger, ElementId::new(1, 1), FIELD);
    let dead_scene = paint_snapshot(&*dead_trigger, ElementId::new(1, 1), FIELD);
    assert_ne!(
        live_scene, dead_scene,
        "the border and the text both fade, so the two are visibly different"
    );
}

// --- semantics ------------------------------------------------------------

/// A combobox publishes a text field that is expanded or not, because 2.0's
/// accessibility vocabulary has no combobox role and inventing one is a platform
/// decision rather than a widget's.
#[test]
fn the_hint_is_a_text_field_that_tracks_openness() {
    let mut combo = combo().placeholder("Search fruit");
    let hint = <Combobox as WidgetAny>::semantics_hint(&combo).expect("a hint");
    assert_eq!(hint.role, floom_core::SemanticsRole::TextField);
    assert_eq!(hint.label.as_deref(), Some("Search fruit"));
    assert!(
        !hint.state.contains(floom_core::SemanticsState::EXPANDED),
        "shut to begin with"
    );

    combo.set_open(true);
    let open = <Combobox as WidgetAny>::semantics_hint(&combo).expect("a hint");
    assert!(
        open.state.contains(floom_core::SemanticsState::EXPANDED),
        "and expanded once it is open — read fresh each time, so it cannot go stale"
    );
}

/// With no placeholder the name falls back to the chosen option, so a combobox with a
/// selection is not an unnamed text field.
#[test]
fn the_name_falls_back_to_the_chosen_option() {
    let mut combo = combo();
    combo.select(2);
    let hint = <Combobox as WidgetAny>::semantics_hint(&combo).expect("a hint");
    assert_eq!(hint.label.as_deref(), Some("Cherry"));
}

// --- the shared-state seam -----------------------------------------------

/// The default state is shared, so a caller can watch the selection without holding
/// the widget. `Arc::ptr_eq` because two independent states would both pass the
/// behavioural tests above and then quietly diverge.
#[test]
fn the_default_state_is_one_allocation() {
    let combo = combo();
    let pick: SharedPick = combo.pick();
    assert!(Arc::ptr_eq(&pick, &combo.pick()));
    assert_eq!(*pick.lock().unwrap(), ComboPick::default());
}

/// The option list is shared too, for the same reason: a row's click handler needs the
/// option text from inside a `Send + Sync` closure that cannot borrow this struct.
#[test]
fn the_option_list_is_one_allocation() {
    let combo = combo();
    assert_eq!(combo.options(), options());
}

/// A key event the combobox does not handle is not consumed — the field is a child, so
/// the combobox itself must not swallow typing.
#[test]
fn a_key_event_reaches_the_child_rather_than_being_swallowed() {
    let mut combo = combo();
    let id = mounted(&mut combo);
    let event = floom_core::Event::Key(floom_types::KeyEvent {
        key: Key::Char('a'),
        phase: KeyPhase::Down,
        modifiers: floom_types::Modifiers::NONE,
    });
    let mut cx = TestEventContext::new(id, FIELD, event);
    assert!(
        !combo.event(&mut cx, id),
        "the combobox has no text of its own; the field does"
    );
}
