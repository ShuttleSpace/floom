// SPDX-License-Identifier: MIT
//! Declarative widget description contract.

use crate::compose::{Child, LayoutRequest};
use crate::context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
use floom_types::ElementId;

/// High-level UI description. Implementations must not depend on concrete renderers.
///
/// The `on_unimplemented` diagnostic turns a bare "trait bound not satisfied"
/// into a message at the exact call site that expects a widget, so a value that
/// is not one reports what it actually is instead of a generic trait failure.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a Floem `Widget`",
    label = "not a Floem `Widget`",
    note = "this value is not a widget and cannot be mounted into a host",
    note = "rebuild the subtree from `floom::prelude` widgets"
)]
pub trait Widget {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId;
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId);
    /// Children to mount underneath this widget's own element.
    ///
    /// Returning an empty list (the default) makes a leaf widget. Composites
    /// rebuild this list on demand and the tree owner diffs it by
    /// [`Child::key`], so only genuinely new slots construct a widget.
    fn children(&self) -> Vec<Child> {
        Vec::new()
    }
    /// Layout intent for *this* node's own box — chiefly how it arranges its
    /// children (flex direction, gap). `None` (the default) means "leaf / no
    /// container behaviour", and the tree owner falls back to whatever the
    /// parent supplied for sizing.
    ///
    /// This is the counterpart to [`Widget::children`]: that says *what* this
    /// widget contains, this says *how* it lays those children out. Keeping the
    /// two apart lets a parent size a child while the child stays in charge of
    /// its own internal flow.
    fn layout_request(&self) -> Option<LayoutRequest> {
        None
    }

    /// Take over the part of `previous`'s state that outlives a rebuild.
    ///
    /// A hot reload rebuilds the tree from source, so every widget is new; the
    /// things a user would notice resetting — the text in a field, the knob on a
    /// switch, where a list is scrolled — live in the widget, and the widget is
    /// the only one that knows which of its fields are state rather than
    /// configuration. The default carries nothing, which is the honest answer
    /// for a widget whose fields are all derived from what its parent passed in.
    ///
    /// `previous` is the widget that occupied the same position (same authoured
    /// key) before the rebuild; it may be a different type, in which case the
    /// implementation must ignore it.
    fn carry_state(&mut self, _previous: &dyn crate::WidgetAny) {}
    /// What this widget declares it paints, for an inspector's Style readout.
    ///
    /// Defaults to `None`, which means "this widget does not describe its own
    /// paint" — a vector path, a custom leaf, a composite that draws from state.
    /// The readout then shows nothing for the node rather than guessing from
    /// the scene, because a draw op cannot say which of several rects was the
    /// background.
    ///
    /// This is a *declaration*, computed with no renderer in the loop: what the
    /// widget intends to draw, not what the framebuffer ended up with. See
    /// [`PaintStyle`](crate::PaintStyle) for the subset it covers and why.
    fn paint_style(&self) -> Option<crate::PaintStyle> {
        None
    }
    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let _ = (cx, id);
        false
    }
    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        let _ = (cx, id);
    }
    /// The animation this widget wants the host to keep running on its own
    /// element, or `None` (the default) for a widget that does not move on its
    /// own.
    ///
    /// Declarative rather than imperative on purpose. A widget has no host
    /// handle — it is borrowed *out* of the host for the duration of every call
    /// the host makes into it — so it cannot start anything itself; what it can
    /// do is say what it wants and let the host own the running state. The host
    /// starts the animation when the declaration first appears, leaves a
    /// running one alone while the declaration is unchanged (so a widget that
    /// re-declares its motion every update does not restart it every update),
    /// and cancels it when the declaration goes away.
    ///
    /// **One animation per element.** Two at once — a fade *and* a slide — would
    /// need the node to carry a composed transform rather than a single
    /// compositor property, and this framework does not build one yet: the host
    /// builds a flat scene, and the group node that would hold a composite
    /// transform is the same missing piece that keeps `Scale` from working.
    /// Declaring two is not a thing to plan around; declaring one is.
    fn animation(&self) -> Option<crate::AnimationSpec> {
        None
    }
    /// The host finished an animation this widget declared through
    /// [`Self::animation`].
    ///
    /// The host finished a *declared* animation this element was running.
    ///
    /// Default: nothing. A widget that declares motion for a reason other than
    /// "drive my own paint" uses this to act on completion — the one thing the
    /// host cannot decide for it. The presence transition (a subtree's enter
    /// or exit animation) is the case it exists for: when the *exit* motion
    /// ends, the subtree is safe to unmount; when the *enter* motion ends, it
    /// has reached rest. The host calls this at the top of a frame, after the
    /// clock advanced and before the update pass, so a widget that re-marks
    /// itself dirty here has its end-of-transition folded in the same frame.
    ///
    /// Receiving it does not mean the animation will never run again: re-arming
    /// a declaration (changing its [`crate::AnimationSpec`] so the host
    /// restarts it) sends this again on the next completion. A widget that
    /// treats the event as "I am done forever" will wedge.
    fn animation_finished(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}
    /// The shape this widget's subtree is clipped to, in the widget's own
    /// coordinates, or `None` (the default) for the usual "content may paint
    /// outside my box".
    ///
    /// Declared rather than done at paint time, for a reason that is structural
    /// and not stylistic: this host paints a **flat** scene in node order, so a
    /// widget's paint call is not bracketed by its children's — there is no
    /// "before and after the subtree" moment for a widget to push and pop a
    /// clip in. The host therefore resolves this declaration against laid-out
    /// bounds and writes the clip markers around the whole subtree itself.
    ///
    /// A clip applies to the widget's own paint as well as its descendants'.
    /// The region is resolved from the element's box, so the one thing a widget
    /// cannot express here is a clip unrelated to its own layout.
    fn clip(&self) -> Option<floom_types::ClipShape> {
        None
    }

    /// Whether this widget will ask [`UpdateContext::content_size`].
    ///
    /// Declared rather than inferred so the host can skip measuring for the
    /// other nodes. Answering the question costs a subtree walk, and doing that
    /// for every parent on every frame would charge a 100k-node tree for a
    /// measurement only a scrolling container needs — a cost on the settled-frame
    /// path that `tests/allocation.rs` budgets at eight allocations for the whole
    /// tree.
    ///
    /// A widget that answers `true` must not be lying: the host measures once per
    /// frame for the nodes that answer `true`, so a widget that lies pays for
    /// walks it does not use. Answering `false` while asking is not a
    /// correctness bug — the question still returns the right answer for a node
    /// that was measured some other way — it just costs a walk it did not need.
    ///
    /// The default is `false`, and it is the right default for the overwhelming
    /// majority of widgets: a leaf has no content to measure, and a container
    /// that does not scroll has no reason to know how big its contents are.
    fn needs_content_size(&self) -> bool {
        false
    }
    /// The filter this widget's **whole subtree** is drawn through, or `None`
    /// (the default) for the usual "drawn as it is".
    ///
    /// Declared rather than pushed at paint time for exactly the reason
    /// [`Self::clip`] is, and one more: a filter has to see the composite of
    /// the subtree, so blurring inside `paint` would blur only the nodes this
    /// widget paints itself — the children's ink would stay sharp on top of a
    /// blurred background, which is a different picture, not a cheaper one.
    /// The host brackets the subtree with the filter markers instead.
    ///
    /// This is the content filter (Compose's `Modifier.blur`). Filtering what
    /// sits *behind* the widget is a different request with a different input,
    /// and it is an op rather than a declaration — see
    /// [`crate::PaintContext::backdrop_filter`].
    fn filter(&self) -> Option<floom_types::Filter> {
        None
    }
    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        let _ = (cx, id);
    }
    /// An overlay this widget asked for is now open, under `handle`.
    ///
    /// The opener needs the handle to close its own overlay — a `Select` whose
    /// header is clicked again, a dialog whose owner is torn down — and it
    /// cannot get it from [`EventContext::open_overlay`], because the overlay
    /// only opens once the event that requested it has finished dispatching.
    /// The content does not need this call: it is built by the request's
    /// content factory, which is handed the handle directly.
    ///
    /// [`EventContext::open_overlay`]: crate::EventContext::open_overlay
    fn overlay_opened(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: crate::OverlayHandle,
    ) {
        let _ = (cx, id, handle);
    }
    /// An overlay this widget opened has closed; `content` is its content,
    /// handed back so the opener can read whatever it accumulated.
    ///
    /// This is how an overlay reports a result. The content — a popup list, a
    /// dialog's form — owns the outcome because it is the thing the user acted
    /// on, and the opener owns the consequence because it owns the state being
    /// changed. The host pairs them back up: it remembers which widget asked
    /// for each overlay, and when that overlay closes it returns the content
    /// box to the asker.
    ///
    /// Always called, whether the overlay was closed by the content itself, by
    /// the opener, or because the host took it down — a widget that cached a
    /// handle must clear it here or it will believe a stale popup is still up.
    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: crate::OverlayHandle,
        content: Box<dyn crate::WidgetAny>,
    ) {
        let _ = (cx, id, handle, content);
    }
    /// The host moved keyboard focus onto this widget, or took it off.
    ///
    /// A widget that paints a focus ring needs to know; one that does not
    /// ignores the call, which is what the default does.
    ///
    /// This used to be a downcast list inside the host — `if let Some(control)
    /// = widget.downcast_mut::<Button>()` — which meant the host had to be told
    /// about every focusable control, and a control that lived above the host's
    /// own widget crate could not be added to it at all. The list was a
    /// correctness trap dressed as an implementation detail: a new focusable
    /// widget compiled, mounted, tracked focus and painted no ring, and nothing
    /// failed. Same shape as [`overlay_opened`](crate::Widget::overlay_opened) / [`overlay_closed`](crate::Widget::overlay_closed): the host
    /// asks the tree, the widget answers.
    fn set_focused(&mut self, focused: bool) {
        let _ = focused;
    }
    /// The pointer shape to show while this widget is under the pointer.
    ///
    /// A default method, so no existing widget has to answer.
    ///
    /// The cursor is a property of what kind of thing a widget is, not of a
    /// design system. A button is a pointer under Material, Cupertino and
    /// Fluent alike, so this deliberately does *not* go in a style type and
    /// does *not* get a builder. Do not add a `cursor` field to any style and
    /// do not add a `.cursor()` builder to any widget: a kit that could make a
    /// button not-a-button's cursor would be expressing something that is never
    /// right.
    fn cursor(&self) -> floom_types::CursorStyle {
        floom_types::CursorStyle::Default
    }
}
