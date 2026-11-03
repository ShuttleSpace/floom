
use floom_core::{
    Child, ChildKey, ElementId, LayoutRequest, OverlayRequest, UpdateContext, Widget, WidgetAny,
};
use floom_types::{OverlayHandle, OverlayKind, Rect};

use floom_ui_composites::{OverlayHost, Popup};
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};
use floom_widgets::Text;

/// A control in the middle of nowhere, so a test that "looks right" has to mean it.
const ANCHOR: Rect = Rect {
    x0: 30.0,
    y0: 100.0,
    x1: 230.0,
    y1: 140.0,
};

const ID: ElementId = ElementId::new(1, 1);

/// A `build` that always offers the anchor, so the request is about the *transition*
/// rather than about what is in it.
fn offer(cx_bounds: Rect) -> impl FnOnce(Rect, Rect) -> Option<OverlayRequest> {
    move |_bounds, _viewport| {
        Some(OverlayRequest::new(
            OverlayKind::Popup,
            false,
            cx_bounds,
            |_handle| Box::new(Text::new("row")) as Box<dyn WidgetAny>,
        ))
    }
}

fn content() -> Child {
    Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(Text::new("row")) as Box<dyn WidgetAny>
    })
}

// --- opening --------------------------------------------------------------

/// Presenting asks the host, for the rect `build` produced.
#[test]
fn presenting_asks_the_host_for_what_build_offered() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);

    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one overlay asked for");
    match opened[0] {
        floom_widgets::testkit::OverlayCall::Open { bounds, kind, .. } => {
            assert_eq!(*bounds, ANCHOR);
            assert_eq!(*kind, OverlayKind::Popup);
        }
        _ => panic!("expected an open, got a close"),
    }
}

/// `build` runs only when the surface is coming up. A reconcile with nothing changed
/// must not run it — that is the difference between "the panel opened" and "the panel
/// is rebuilt every frame", and only one of them is a bug you can see.
#[test]
fn a_reconcile_with_nothing_changed_asks_nothing() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    assert_eq!(cx.opened_overlays().len(), 1, "the first one opened it");

    cx.clear();
    // A second present-and-unchanged reconcile. `build` is a `FnOnce`, so if this
    // called it the test would not compile — which is the point of the signature.
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    assert!(
        !cx.opened_any_overlay(),
        "two more updates, two more nothing"
    );
}

/// The transition re-declares the subtree, because both outcomes change which nodes
/// exist. A widget that only repainted would leave the panel's own children stale.
#[test]
fn a_transition_reconciles_and_repaints() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));

    assert!(cx.reconciled(), "the children must be re-declared");
    assert!(!cx.dirty().is_empty(), "and the node repainted");
}

// --- closing --------------------------------------------------------------

/// Closing closes *the handle the host issued* — not a fresh one, and not none.
///
/// The first half is the bug a plausible implementation has: asking to close without
/// remembering what it was given leaves the panel up forever, because the only way to
/// take it down is by name.
#[test]
fn closing_closes_the_handle_the_host_issued() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    let granted = cx.opened_overlays()[0].handle();
    // The host answers *later*, through `overlay_opened`. Without this the widget has
    // no handle to name and the close below has nothing to say — which is the whole
    // reason the grant is a separate callback and not a return value.
    host.opened(granted);

    cx.clear();
    host.set_present(false);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    assert_eq!(
        cx.closed_overlays(),
        vec![granted],
        "closed by name, and the handle is the host's"
    );
}

/// A popup that was never opened is not closed, or the host gets close requests for
/// overlays that never existed.
#[test]
fn a_never_opened_popup_closes_nothing() {
    let mut host = OverlayHost::new(false);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    assert!(
        cx.closed_overlays().is_empty(),
        "nothing opened, so nothing to close"
    );
}

// --- the host closing it --------------------------------------------------

/// When the host takes the overlay down on its own — an outside press — `present`
/// must follow, or the next reconcile re-opens it on the next frame.
///
/// This is the stuck-popup bug in one line: a combobox that believes its dropdown is
/// open, with nothing on screen, re-requests it every frame.
#[test]
fn a_host_dismissal_clears_present_so_it_does_not_come_straight_back() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    let granted = cx.opened_overlays()[0].handle();
    host.opened(granted);

    cx.clear();
    host.closed(granted);
    assert!(
        !host.is_shown(),
        "the host took it down, so the widget agrees"
    );

    cx.clear();
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    assert!(
        !cx.opened_any_overlay(),
        "and the next frame does not ask again — which is the whole point"
    );
}

/// A hand-back for an overlay this host had already forgotten is ignored. Otherwise a
/// stale content box would clear a *live* handle, leaving a panel nothing can close.
#[test]
fn a_hand_back_for_another_overlay_is_ignored() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    let mine = cx.opened_overlays()[0].handle();
    let theirs = OverlayHandle::new(9999);
    host.opened(mine);

    assert!(
        !host.closed(theirs),
        "a handle we never opened closes nothing"
    );
    assert!(host.is_shown(), "so the panel this host owns is still up");
}

// --- the refusal, which is the branch that rots ----------------------------

/// A host with no overlay support refuses, and the surface falls back in-tree.
///
/// The alternative — showing nothing — is the failure `UpdateContext::open_overlay`'s
/// own doc warns about. This test exists so that branch is not unreachable: a double
/// that always granted would let it rot.
#[test]
fn a_refused_request_falls_back_in_tree_rather_than_showing_nothing() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);

    let mut cx = RecordingUpdateContext::without_overlay_support();
    host.reconcile(&mut cx, ID, offer(ANCHOR));

    assert!(!cx.opened_any_overlay(), "nothing was granted");
    assert!(
        host.takes_children_in_tree(),
        "so the surface is mounted in-tree instead"
    );
    assert!(
        !host
            .fallback_children(Some(content()), ChildKey::index(0))
            .is_empty(),
        "and there is a layer for it to live in"
    );
}

/// A granted overlay means *no* in-tree layer, or the panel appears twice.
#[test]
fn a_granted_overlay_mounts_nothing_in_tree() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, offer(ANCHOR));
    assert!(
        host.fallback_children(Some(content()), ChildKey::index(0))
            .is_empty(),
        "the host stack owns it now"
    );
}

/// A closed surface contributes no in-tree layer either — the fallback must not
/// outlive the reason it exists.
#[test]
fn a_closed_surface_mounts_nothing_in_tree() {
    let mut host = OverlayHost::new(false);
    assert!(
        host.fallback_children(Some(content()), ChildKey::index(0))
            .is_empty(),
        "not shown, so not mounted"
    );
}

/// Nothing to put in an overlay means nothing mounted anywhere.
///
/// A `Popup` with no content is the case: `build` returns `None`, the host is never
/// asked, and the in-tree layer is empty too — so the "no content" state is
/// indistinguishable from "hidden", which is right, because both show nothing.
#[test]
fn nothing_to_show_mounts_nothing_anywhere() {
    let mut host = OverlayHost::new(false);
    host.set_present(true);
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    host.reconcile(&mut cx, ID, |_bounds, _viewport| None);

    assert!(!cx.opened_any_overlay(), "no request for an empty panel");
    // A refused request is exactly what arms the in-tree fallback, so this is true
    // here for the same reason it is true on a headless host. The panel is absent
    // because there was no *content*, not because the fallback declined — and that
    // distinction is what the next assertion pins.
    assert!(
        host.takes_children_in_tree(),
        "a refusal is a refusal, whatever caused it"
    );
    assert!(
        host.fallback_children(None, ChildKey::index(0)).is_empty(),
        "there was no content to begin with, so there is nothing to mount"
    );
    assert_eq!(
        host.fallback_children(Some(content()), ChildKey::index(0))
            .len(),
        1,
        "and a panel that *did* have content would have fallen back in-tree"
    );
}

// --- it still works for the composite that already used it ----------------

/// The control group for the whole file: `Popup` still opens, closes and reports
/// itself. `OverlayHost` was `pub(crate)` when these paths were written, so this is
/// what proves making it public moved nothing.
#[test]
fn a_popup_still_opens_through_the_shared_host() {
    let mut popup = Popup::new().content_with(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(Text::new("row")) as Box<dyn WidgetAny>
    });
    let mut cx = RecordingUpdateContext::with_bounds(ANCHOR);
    let id = popup.create(&mut AllocatingCreateContext::new());

    // Not presented: nothing.
    popup.update(&mut cx, id);
    assert!(!cx.opened_any_overlay(), "a popup nobody opened");

    cx.clear();
    popup.set_present(true);
    popup.update(&mut cx, id);
    assert_eq!(cx.opened_overlays().len(), 1, "and a presented one opens");

    // And it contributes no children while the host owns it.
    assert!(
        popup.children().is_empty(),
        "the panel is not also in the tree"
    );
}
