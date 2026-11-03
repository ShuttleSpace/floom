
use std::sync::Arc;

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Widget, WidgetAny};
use floom_types::{OverlayKind, Rect};

use floom_ui_composites::Dialog;
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};
use floom_widgets::{read_flag, shared_flag, write_flag, SharedFlag};

/// A viewport, because a dialog covers the window rather than its own box.
const VIEWPORT: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 800.0,
    y1: 600.0,
};

const ID: ElementId = ElementId::new(1, 1);

fn dialog(presence: Option<SharedFlag>) -> Dialog {
    let dialog = Dialog::new().title("Confirm").body(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(floom_widgets::Text::new("body")) as Box<dyn WidgetAny>,
    ));
    match presence {
        Some(flag) => dialog.with_shared_presence(flag),
        None => dialog,
    }
}

fn context() -> RecordingUpdateContext {
    // A dialog is sized from the viewport, so both boxes are set and the request is
    // checked against the viewport rather than the node.
    RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT)
}

// --- opening from a flag --------------------------------------------------

/// A flag set to true asks the host for a modal dialog over the whole viewport.
#[test]
fn a_flag_set_to_true_opens_a_modal_dialog_over_the_viewport() {
    let flag = shared_flag(true);
    let mut dialog = dialog(Some(flag.clone()));
    let mut cx = context();
    dialog.update(&mut cx, ID);

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one overlay asked for");
    match opened[0] {
        floom_widgets::testkit::OverlayCall::Open {
            kind,
            modal,
            bounds,
            ..
        } => {
            assert_eq!(*kind, OverlayKind::Dialog);
            assert!(
                *modal,
                "a dialog holds the window — which is exactly what makes it different \
                 from a popup, and why the host's outside-press rule does not touch it"
            );
            assert_eq!(*bounds, VIEWPORT, "and covers the window, not its own box");
        }
        _ => panic!("expected an open, got a close"),
    }
}

/// A flag set to false asks for nothing — the control for the test above, since a
/// request with `modal: true` and a zero rect would look right at a glance.
#[test]
fn a_flag_set_to_false_asks_for_nothing() {
    let flag = shared_flag(false);
    let mut dialog = dialog(Some(flag));
    let mut cx = context();
    dialog.update(&mut cx, ID);
    assert!(!cx.opened_any_overlay());
}

/// The flag is read on **every** update, not only the first. This is the difference
/// between a flag and a constructor argument, and it is the whole reason the flag exists:
/// an external button flips it long after the dialog was built.
#[test]
fn the_flag_is_read_on_every_update_not_only_the_first() {
    let flag = shared_flag(false);
    let mut dialog = dialog(Some(flag.clone()));
    let mut cx = context();

    dialog.update(&mut cx, ID);
    assert!(!cx.opened_any_overlay(), "shut to begin with");

    cx.clear();
    write_flag(&flag, true);
    dialog.update(&mut cx, ID);
    assert_eq!(cx.opened_overlays().len(), 1, "and the flip opens it");
}

// --- the host closing it --------------------------------------------------

/// **A host dismissal clears the flag**, so a caller reading it sees the dialog close.
///
/// Without this the dialog comes straight back on the next update — the stuck-dialog bug,
/// and the mirror image of the one `Combobox` had.
#[test]
fn a_host_dismissal_clears_the_flag() {
    let flag = shared_flag(true);
    let mut dialog = dialog(Some(flag.clone()));
    let mut cx = context();
    dialog.update(&mut cx, ID);
    let granted = cx.opened_overlays()[0].handle();
    dialog.overlay_opened(&mut cx, ID, granted);
    assert!(read_flag(&flag), "open, and the caller can see it");

    dialog.overlay_closed(
        &mut cx,
        ID,
        granted,
        Box::new(floom_widgets::Text::new("content")),
    );
    assert!(
        !read_flag(&flag),
        "the host took it down, so the caller's flag says so too"
    );

    cx.clear();
    dialog.update(&mut cx, ID);
    assert!(
        !cx.opened_any_overlay(),
        "and the next frame does not ask again — which is the whole point"
    );
}

/// A hand-back for somebody else's overlay is ignored, so a stale content box cannot
/// clear a live dialog.
#[test]
fn a_hand_back_for_another_overlay_does_not_clear_the_flag() {
    let flag = shared_flag(true);
    let mut dialog = dialog(Some(flag.clone()));
    let mut cx = context();
    dialog.update(&mut cx, ID);
    let granted = cx.opened_overlays()[0].handle();
    dialog.overlay_opened(&mut cx, ID, granted);

    dialog.overlay_closed(
        &mut cx,
        ID,
        floom_types::OverlayHandle::new(31337),
        Box::new(floom_widgets::Text::new("content")),
    );
    assert!(read_flag(&flag), "still open, because that was not ours");
}

// --- the flag itself ------------------------------------------------------

/// `presence()` hands back the same allocation. `Arc::ptr_eq`, because two independent
/// flags would pass every behavioural test above and then quietly diverge.
#[test]
fn the_flag_is_one_allocation() {
    let flag = shared_flag(true);
    let dialog = dialog(Some(flag.clone()));
    let handed_back = dialog.presence().expect("a flag was given");
    assert!(Arc::ptr_eq(&flag, &handed_back));
}

/// And a dialog with no flag says so rather than inventing one.
#[test]
fn a_dialog_with_no_flag_reports_none() {
    assert!(dialog(None).presence().is_none());
}

/// `write_flag` reports whether it changed anything, so a caller can skip a frame; and
/// `read_flag` on a poisoned mutex is `false` rather than a panic in a paint pass.
#[test]
fn write_flag_reports_whether_it_changed_anything() {
    let flag = shared_flag(false);
    assert!(write_flag(&flag, true), "false to true");
    assert!(
        !write_flag(&flag, true),
        "and then nothing, so a caller can skip a frame"
    );
    assert!(read_flag(&flag));
    assert!(write_flag(&flag, false), "and true to false");
}

/// A poisoned mutex reads as `false` rather than panicking.
///
/// Poisoning a real `Mutex` needs a panic while it is held, which is awkward to arrange
/// from a test — so the claim is made about the *code path* instead: `read_flag` matches
/// on the `LockResult` and takes the `Err` arm, while every `SharedBuffer` reader in this
/// crate does the same. That consistency is the property, and it is why a widget whose
/// state a third party holds cannot take down a frame over it.
#[test]
fn a_poisoned_flag_reads_as_false_rather_than_panicking() {
    let flag = shared_flag(true);
    let _ = std::panic::catch_unwind(|| {
        let _guard = flag.lock().unwrap();
        panic!("poison it");
    });
    assert!(
        !read_flag(&flag),
        "a flag somebody poisoned reads as not-set — the alternative is a paint pass \
         that panics over a bool nobody was looking at"
    );
}

/// The control group for the whole file: a dialog with no flag is still driven by
/// `set_present`, exactly as it was before this change.
#[test]
fn a_dialog_with_no_flag_is_still_driven_by_set_present() {
    let mut dialog = dialog(None);
    let mut cx = context();
    dialog.set_present(true);
    dialog.update(&mut cx, ID);
    assert_eq!(cx.opened_overlays().len(), 1, "unchanged behaviour");
    assert!(
        dialog.presence().is_none(),
        "and still no flag, because none was asked for"
    );
}
// --- the caller-supplied card --------------------------------------------

/// The reason `card_with` exists: v1's card carries a **close button**, and the engine's
/// title-above-body layout has no slot for one.
///
/// Painted rather than asserted structurally, because the claim is about what appears on
/// screen: the override's text is there, and the engine's title is not, so a caller
/// supplying a card does not get two titles.
#[test]
fn a_supplied_card_is_what_gets_mounted() {
    let flag = shared_flag(true);
    let mut dialog = dialog(Some(flag))
        .title("Engine title")
        .card_with(|| Box::new(floom_widgets::Text::new("supplied card")) as Box<dyn WidgetAny>);
    let mut cx = context();
    dialog.update(&mut cx, ID);

    let content = match cx.opened_overlays()[0] {
        floom_widgets::testkit::OverlayCall::Open { content, .. } => content,
        _ => panic!("expected an open"),
    };
    let scene = floom_widgets::testkit::paint_snapshot_any(content.as_ref(), ID, VIEWPORT);
    assert!(
        scene.contains("supplied"),
        "the caller's card is what the host mounts: {scene}"
    );
    assert!(
        !scene.contains("Engine title"),
        "and the engine's title is *not* also rendered, so there is no duplicate: {scene}"
    );
}

/// The control: with no override, the request carries the engine's own `Stack`.
///
/// I first wrote this by painting and asserting the title appeared, and it failed with
/// **"(empty)"** — because an unmounted `Stack` paints none of its children's ink, so a
/// scene-based assertion here would have been vacuous. Worse, the *supplied-card* test
/// above passes for the same reason it would pass if the widget painted nothing at all: a
/// bare `Text` does paint itself, a `Stack` of children does not.
///
/// So the control downcasts instead. It also caught the real thing: the control was
/// asserting the wrong thing, and a green run would have hidden that.
#[test]
fn without_an_override_the_request_carries_the_engines_own_stack() {
    let flag = shared_flag(true);
    let mut dialog = dialog(Some(flag)).title("Engine title");
    let mut cx = context();
    dialog.update(&mut cx, ID);
    let content = match cx.opened_overlays()[0] {
        floom_widgets::testkit::OverlayCall::Open { content, .. } => content,
        _ => panic!("expected an open"),
    };
    assert!(
        content.as_any().is::<floom_widgets::Stack>(),
        "the engine's title-above-body stack, not the caller's card"
    );
}

/// And a card override is built **fresh per mount** rather than shared — a card is a
/// widget instance, and handing the same one to two overlays would give two dialogs one
/// set of children.
#[test]
fn a_supplied_card_is_built_afresh_each_mount() {
    let flag = shared_flag(true);
    let built = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = built.clone();
    let mut dialog = dialog(Some(flag.clone())).card_with(move || {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::new(floom_widgets::Text::new("card")) as Box<dyn WidgetAny>
    });

    let mut cx = context();
    dialog.update(&mut cx, ID);
    assert_eq!(built.load(std::sync::atomic::Ordering::SeqCst), 1);

    // Close it and open it again: a second mount must be a second build, or the second
    // dialog would reuse the first one's already-mounted children.
    //
    // **The flag, not `set_present`.** I first used `set_present(false)` here and the
    // close silently did nothing: with a shared presence flag, `update` overwrites
    // `present` from the flag on the very next line, so `set_present` is only meaningful
    // between an update and the next one. That is a real interaction between the two
    // ways to drive a dialog and it is worth a reader knowing — a dialog given a flag has
    // one source of truth, and `set_present` is not it.
    write_flag(&flag, false);
    dialog.update(&mut cx, ID);
    write_flag(&flag, true);
    dialog.update(&mut cx, ID);
    assert_eq!(
        built.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "a second mount builds a second card"
    );
}
