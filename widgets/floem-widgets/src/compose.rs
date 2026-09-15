//! Composite (layout + navigation) widgets — Tier-2.
//!
//! These sit above the primitives and describe *structure*: a [`Stack`] arranges
//! children along an axis, [`List`] mirrors a data collection with keyed rows,
//! [`Conditional`] swaps a subtree by a boolean, [`Overlay`]/[`Dialog`] cover
//! their parent with an absolutely positioned layer, and [`Tabs`] shows one of
//! several panels beside a selectable bar.
//!
//! Every composite owns only builders (`Arc<dyn Fn ..>`), never the widgets
//! behind them: [`Widget::children`] hands fresh [`Child`] slots to the host,
//! which diffs them by [`ChildKey`] and reuses mounted widgets across updates.
//! That is what lets a [`List`] keep each row's state when an item is inserted in
//! the middle, and a [`Dialog`] keep focus when it is dismissed and reopened.
//!
//! Composites that change their children at runtime (`List`, `Conditional`,
//! `Overlay`, `Dialog`, `Tabs`) flag the change from [`Widget::update`] by
//! calling [`UpdateContext::mark_children_dirty`]; the host then reconciles just
//! the affected subtree instead of repainting everything.

use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floem_accessibility::SemanticsRole;
use floem_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, OverlayHandle,
    OverlayKind, OverlayRequest, PaintContext, Rect, SemanticsContext, UpdateContext, Widget,
    WidgetAny,
};

use crate::controls::Button;
use crate::primitives::{SemanticsHint, Text};

// ---------------------------------------------------------------------------
// Stack
// ---------------------------------------------------------------------------

/// Lays out its children along one [`Axis`] with a fixed gap.
#[derive(Clone)]
pub struct Stack {
    axis: Axis,
    gap: f32,
    main_align: Option<Align>,
    cross_align: Option<Align>,
    children: Vec<Child>,
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stack")
            .field("axis", &self.axis)
            .field("gap", &self.gap)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Stack {
    /// A stack laid out along `axis`, no gap, default (stretch) alignment.
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            gap: 0.0,
            main_align: None,
            cross_align: None,
            children: Vec::new(),
        }
    }

    /// Gap between children in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// How this stack aligns its children on the main and cross axes.
    pub fn align(mut self, main: Align, cross: Align) -> Self {
        self.main_align = Some(main);
        self.cross_align = Some(cross);
        self
    }

    /// Appends a prebuilt child slot.
    pub fn child(mut self, child: Child) -> Self {
        self.children.push(child);
        self
    }

    /// Appends a child auto-keyed by position with default layout.
    pub fn push(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        let key = ChildKey::index(self.children.len());
        self.children
            .push(Child::new(key, LayoutRequest::new(), build));
        self
    }

    /// Appends a child auto-keyed by position with an explicit layout.
    pub fn push_layout(
        mut self,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        let key = ChildKey::index(self.children.len());
        self.children.push(Child::new(key, layout, build));
        self
    }

    /// Appends a child keyed by a stable id (state follows the item, not the slot).
    pub fn push_keyed(
        mut self,
        id: u64,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.children
            .push(Child::new(ChildKey::keyed(id), LayoutRequest::new(), build));
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Stack {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        self.children.clone()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        let mut req = LayoutRequest::new().stack(self.axis, self.gap);
        if let Some(a) = self.main_align {
            req = req.main_align(a);
        }
        if let Some(a) = self.cross_align {
            req = req.cross_align(a);
        }
        Some(req)
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Conditional
// ---------------------------------------------------------------------------

/// Shows its single child when `when` is true, otherwise renders nothing.
///
/// Toggling `when` mounts/unmounts the subtree without touching its siblings.
#[derive(Clone)]
pub struct Conditional {
    when: bool,
    last_when: bool,
    child: Option<Child>,
}

impl std::fmt::Debug for Conditional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conditional")
            .field("when", &self.when)
            .finish()
    }
}

impl Conditional {
    pub fn new(when: bool) -> Self {
        // Start "changed" so the first frame always reconciles.
        Self {
            when,
            last_when: !when,
            child: None,
        }
    }

    /// Sets the (single) child shown while `when` is true.
    pub fn show(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Sets the (single) child with an explicit key and layout.
    pub fn show_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new(key, layout, build));
        self
    }

    /// Updates whether the child is shown; the next frame reconciles.
    pub fn set_when(&mut self, when: bool) {
        self.when = when;
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Conditional {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.when != self.last_when {
            cx.mark_children_dirty(id);
            self.last_when = self.when;
        }
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) if self.when => vec![child.clone()],
            _ => Vec::new(),
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Overlay-backed surface plumbing
// ---------------------------------------------------------------------------

/// Bookkeeping shared by the composites whose surface belongs *above* the tree.
///
/// These widgets are driven by state — an app flips `present` — so there is no
/// event to react to and the request has to be made from [`Widget::update`].
/// Two outcomes are possible. The host mounts the surface on its overlay stack,
/// which is the normal case and the only one that escapes the parent's clip. Or
/// the host has no overlay support and refuses, in which case the composite
/// falls back to mounting the same surface in-tree — what a headless embedder
/// gets, and what these widgets did before overlays existed.
#[derive(Clone)]
struct Surface {
    present: bool,
    last_present: bool,
    /// The ticket the host issued, while the surface sits on its overlay stack.
    handle: Option<OverlayHandle>,
    /// True while the surface is mounted in-tree rather than on the stack.
    in_tree: bool,
}

impl Surface {
    fn new(present: bool) -> Self {
        Self {
            present,
            // "Not yet seen": the first `update` always acts on whatever the
            // widget currently is. Starting it at `!present` instead would lose
            // a transition made before the first update — a nested composite is
            // only mounted by the reconciler *after* the update pass that would
            // have run it, so `set_present(true)` in between would be swallowed
            // and the surface would never open.
            last_present: false,
            handle: None,
            in_tree: true,
        }
    }

    /// Whether the surface should be visible at all.
    fn is_shown(&self) -> bool {
        self.present
    }

    /// Whether the surface is currently mounted as in-tree children, i.e. the
    /// host refused the overlay (or never had one).
    fn takes_children_in_tree(&self) -> bool {
        self.present && self.in_tree
    }

    fn set_present(&mut self, present: bool) {
        self.present = present;
    }

    /// Opens or closes this surface's host overlay so it matches `present`.
    ///
    /// `build` runs only while the surface is coming up, and turns the node's
    /// own box plus the host viewport into the request to place — or `None` when
    /// there is nothing to put in an overlay after all (a `Popup` with no
    /// content), in which case nothing is mounted anywhere.
    fn reconcile(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        build: impl FnOnce(Rect, Rect) -> Option<OverlayRequest>,
    ) {
        if self.present == self.last_present {
            return;
        }
        let opening = self.present;
        self.last_present = self.present;
        if opening {
            let opened = match build(cx.bounds(), cx.viewport()) {
                Some(request) => cx.open_overlay(request),
                None => false,
            };
            self.in_tree = !opened;
        } else {
            if let Some(handle) = self.handle.take() {
                cx.close_overlay(handle);
            }
            self.in_tree = false;
        }
        // Both outcomes change which subtree exists: taking the host overlay
        // means the in-tree children must go, and a refusal means the in-tree
        // children must come back.
        cx.mark_children_dirty(id);
        cx.mark_dirty(id);
    }

    /// Records the handle the host issued for this surface's overlay.
    fn opened(&mut self, handle: OverlayHandle) {
        self.handle = Some(handle);
    }

    /// The overlay carrying this surface came down — because the widget asked it
    /// to, or because the host dismissed it. Reports whether this surface was
    /// the one that closed.
    ///
    /// `present` is cleared as well: a dismissal the widget did not ask for
    /// (clicking outside a popup) has to leave the widget's own state matching
    /// what the user sees, or the next `update` would believe the surface is
    /// still up and never reopen it.
    fn closed(&mut self, handle: OverlayHandle) -> bool {
        if self.handle != Some(handle) {
            return false;
        }
        self.handle = None;
        self.present = false;
        self.last_present = false;
        true
    }
}

/// The fallback content for a body that was never supplied.
fn empty_body() -> Child {
    Child::new(ChildKey::index(1), LayoutRequest::new(), || {
        Box::new(Text::new("(empty)"))
    })
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

/// An absolutely positioned layer that covers its parent's whole box.
///
/// Use it for modals, drawers and toasts: the child escapes the parent's flow
/// and spans the containing block. Toggling the content mounts/unmounts just
/// the layer, leaving the rest of the tree untouched.
///
/// This is a *layout* primitive, not a host overlay: the layer is clipped by
/// whatever contains it. [`Dialog`], [`Popup`] and [`BottomSheet`] are the
/// composites that hand their surface to the host instead (and fall back to
/// building on this layer when the host has no overlays), which is the
/// difference between "inside this box" and "above everything".
#[derive(Clone)]
pub struct Overlay {
    present: bool,
    last_present: bool,
    child: Option<Child>,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("present", &self.present)
            .finish()
    }
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            present: false,
            last_present: false,
            child: None,
        }
    }

    /// Puts `child` on the layer and marks it present.
    pub fn content(mut self, child: Child) -> Self {
        self.child = Some(child);
        self.present = true;
        self
    }

    /// Puts `build` on the layer with an explicit key and layout.
    pub fn content_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new(key, layout, build));
        self.present = true;
        self
    }

    /// Replaces (or clears) the layer content; `None` dismisses it.
    pub fn set_content(&mut self, child: Option<Child>) {
        self.present = child.is_some();
        self.child = child;
    }

    /// Dismisses the layer.
    pub fn dismiss(&mut self) {
        self.child = None;
        self.present = false;
    }

    /// Whether the layer is up.
    pub fn is_present(&self) -> bool {
        self.present
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Overlay {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.present != self.last_present {
            cx.mark_children_dirty(id);
            self.last_present = self.present;
        }
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) if self.present => vec![child.clone()],
            _ => Vec::new(),
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------

/// A modal dialog: a centred card rendered above the tree.
///
/// `present` toggles it; while hidden it contributes nothing, so the rest of the
/// tree lays out exactly as if it were not there. Showing it hands the card to
/// the host as a modal overlay, which is what makes it cover the window rather
/// than the box it happens to be declared in — a dialog declared inside a
/// scrolling list is still centred on the screen and still owns the keyboard.
#[derive(Clone)]
pub struct Dialog {
    surface: Surface,
    title: String,
    body: Option<Child>,
}

impl std::fmt::Debug for Dialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("present", &self.surface.present)
            .finish()
    }
}

impl Dialog {
    pub fn new() -> Self {
        Self {
            surface: Surface::new(false),
            title: String::new(),
            body: None,
        }
    }

    /// The text shown at the top of the card.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The card body (any widget subtree).
    pub fn body(mut self, body: Child) -> Self {
        self.body = Some(body);
        self
    }

    /// The card body built lazily with an explicit key and layout.
    pub fn body_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.body = Some(Child::new(key, layout, build));
        self
    }

    /// Shows or hides the dialog; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Whether the dialog is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    /// The card: a centred, vertically stacked [`Stack`] holding the title and
    /// the body. Built fresh on demand so each mount yields new builders; the
    /// host reuses nodes by key and only mounts/unmounts on `present`.
    fn card(&self) -> Stack {
        let title = self.title.clone();
        let body = self.body.clone().unwrap_or_else(empty_body);
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .align(Align::Center, Align::Center)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(Text::new(title.clone())),
            ))
            .child(body)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint::labeled(SemanticsRole::Dialog, self.title.clone())
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Dialog {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let card = self.card();
        // A dialog covers the window, not its own box, so it is sized from the
        // viewport the host reports rather than from `bounds`.
        self.surface.reconcile(cx, id, move |_bounds, viewport| {
            Some(OverlayRequest::new(
                OverlayKind::Dialog,
                true,
                viewport,
                move |_handle| Box::new(card),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        // The host refused the overlay (or has none at all), so fall back to the
        // in-tree layer this widget used before it could escape its parent.
        let card = self.card();
        let surface =
            Overlay::new().content_with(ChildKey::index(0), LayoutRequest::new(), move || {
                Box::new(card.clone())
            });
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// A data-driven list. Mirrors `items` into keyed rows so inserting/removing in
/// the middle keeps each surviving row's widget (scroll position, focus, …)
/// instead of rebuilding the whole list.
///
/// `T` must be `Clone + Send + Sync + 'static` because each row is built from an
/// owned copy of its item inside a `Send + Sync` builder closure.
pub struct List<T> {
    axis: Axis,
    gap: f32,
    items: Vec<T>,
    key_of: Arc<dyn Fn(&T) -> u64 + Send + Sync>,
    build_item: Arc<dyn Fn(T) -> Box<dyn WidgetAny> + Send + Sync>,
    item_layout: LayoutRequest,
    dirty: bool,
}

impl<T: std::fmt::Debug> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("len", &self.items.len())
            .finish()
    }
}

impl<T: Clone + Send + Sync + 'static> List<T> {
    /// Builds a list from `items`, a stable `key_of` extractor, and an item
    /// builder that turns an owned item into a widget.
    pub fn new(
        axis: Axis,
        items: Vec<T>,
        key_of: impl Fn(&T) -> u64 + Send + Sync + 'static,
        build_item: impl Fn(T) -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        Self {
            axis,
            gap: 0.0,
            items,
            key_of: Arc::new(key_of),
            build_item: Arc::new(build_item),
            item_layout: LayoutRequest::new(),
            dirty: false,
        }
    }

    /// Gap between rows in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Layout applied to every row (defaults to auto-sized).
    pub fn item_layout(mut self, layout: LayoutRequest) -> Self {
        self.item_layout = layout;
        self
    }

    /// Replaces the backing collection; the next frame reconciles the diff.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.dirty = true;
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl<T: Clone + Send + Sync + 'static> Widget for List<T> {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.dirty {
            cx.mark_children_dirty(id);
            self.dirty = false;
        }
    }

    fn children(&self) -> Vec<Child> {
        self.items
            .iter()
            .map(|item| {
                let key = ChildKey::keyed((self.key_of)(item));
                let item = item.clone();
                let build = self.build_item.clone();
                Child::new(key, self.item_layout, move || (build)(item.clone()))
            })
            .collect()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(self.axis, self.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl<T: Clone + Send + Sync + 'static> WidgetAny for List<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floem_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/// A tabbed navigation: a selectable bar plus the panel for the active tab.
///
/// Selection lives in an `Arc<AtomicUsize>` so a tab button's `on_click` can flip
/// it; `update` notices the change and re-reconciles, mounting the new panel and
/// (because the bar is keyed by selection) re-rendering the highlight. The bar
/// and panel are keyed by the active index, so switching tabs unmounts the old
/// pair and mounts a fresh one rather than mutating in place.
#[derive(Clone)]
pub struct Tabs {
    selection: Arc<AtomicUsize>,
    last_selected: usize,
    tabs: Vec<(String, Child)>,
}

impl std::fmt::Debug for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tabs")
            .field("tabs", &self.tabs.len())
            .finish()
    }
}

impl Tabs {
    pub fn new() -> Self {
        // `last_selected` starts impossible so the first frame always reconciles.
        Self {
            selection: Arc::new(AtomicUsize::new(0)),
            last_selected: usize::MAX,
            tabs: Vec::new(),
        }
    }

    /// Adds a tab with `title` and its `body` widget subtree.
    pub fn tab(mut self, title: impl Into<String>, body: Child) -> Self {
        self.tabs.push((title.into(), body));
        self
    }

    /// Shared selection handle; the example can also drive it imperatively.
    pub fn selection(&self) -> Arc<AtomicUsize> {
        self.selection.clone()
    }

    /// Selects tab `index`; the next frame re-reconciles if it changed.
    pub fn set_selected(&mut self, index: usize) {
        self.selection.store(index, Ordering::SeqCst);
    }

    fn current(&self) -> usize {
        let n = self.tabs.len();
        if n == 0 {
            return 0;
        }
        self.selection.load(Ordering::SeqCst).min(n - 1)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Tabs {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let cur = self.current();
        if cur != self.last_selected {
            cx.mark_children_dirty(id);
            self.last_selected = cur;
        }
    }

    fn children(&self) -> Vec<Child> {
        if self.tabs.is_empty() {
            return Vec::new();
        }
        let cur = self.current();
        let sel = self.selection.clone();

        // The bar is keyed by the current selection so it rebuilds (and
        // re-highlights) whenever the active tab changes.
        let mut bar_stack = Stack::new(Axis::Horizontal).gap(4.0);
        for i in 0..self.tabs.len() {
            let sel = sel.clone();
            let active = i == cur;
            let label = if active {
                format!("▶ {}", self.tabs[i].0)
            } else {
                self.tabs[i].0.clone()
            };
            bar_stack = bar_stack.child(Child::new(
                ChildKey::keyed(0xA11B_0000 + i as u64),
                LayoutRequest::new().height(24.0).grow(1.0),
                move || {
                    let sel = sel.clone();
                    Box::new(
                        Button::new(label.clone()).on_click(move || sel.store(i, Ordering::SeqCst)),
                    )
                },
            ));
        }
        let bar_child = Child::new(
            ChildKey::keyed(0xA11B_4000 + cur as u64),
            LayoutRequest::new().stack(Axis::Horizontal, 4.0),
            move || Box::new(bar_stack.clone()),
        );

        let body = self.tabs[cur].1.clone();
        let panel_child = Child::new(
            ChildKey::keyed(0xA11B_8000 + cur as u64),
            LayoutRequest::new().fill(),
            move || body.build(),
        );

        vec![bar_child, panel_child]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .stack(Axis::Vertical, 6.0)
                .fill()
                .cross_align(Align::Stretch),
        )
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Grid
// ---------------------------------------------------------------------------

/// A data-driven grid: rows of `columns` keyed cells.
///
/// Where [`List`] mirrors a one-dimensional collection, `Grid` chunks the same
/// collection into fixed-size rows and lays each row out as a [`Stack`] along
/// the horizontal axis. Row keys are a fingerprint of the item keys they hold,
/// so inserting or removing an item remounts only the rows whose membership
/// actually changed — every other row keeps its widgets.
pub struct Grid<T> {
    columns: usize,
    column_gap: f32,
    row_gap: f32,
    items: Vec<T>,
    key_of: Arc<dyn Fn(&T) -> u64 + Send + Sync>,
    build_item: Arc<dyn Fn(T) -> Box<dyn WidgetAny> + Send + Sync>,
    item_layout: LayoutRequest,
    dirty: bool,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Grid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("columns", &self.columns)
            .field("len", &self.items.len())
            .finish()
    }
}

impl<T: Clone + Send + Sync + 'static> Grid<T> {
    /// Builds a grid with `columns` cells per row from `items`.
    pub fn new(
        columns: usize,
        items: Vec<T>,
        key_of: impl Fn(&T) -> u64 + Send + Sync + 'static,
        build_item: impl Fn(T) -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        Self {
            columns: columns.max(1),
            column_gap: 0.0,
            row_gap: 0.0,
            items,
            key_of: Arc::new(key_of),
            build_item: Arc::new(build_item),
            item_layout: LayoutRequest::new(),
            dirty: false,
        }
    }

    /// Sets the horizontal (between columns) and vertical (between rows) gaps.
    pub fn gap(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }

    /// Layout applied to every cell (defaults to auto-sized).
    pub fn item_layout(mut self, layout: LayoutRequest) -> Self {
        self.item_layout = layout;
        self
    }

    /// Replaces the backing collection; the next frame reconciles the diff.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.dirty = true;
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl<T: Clone + Send + Sync + 'static> Widget for Grid<T> {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.dirty {
            cx.mark_children_dirty(id);
            self.dirty = false;
        }
    }

    fn children(&self) -> Vec<Child> {
        if self.items.is_empty() {
            return Vec::new();
        }
        self.items
            .chunks(self.columns)
            .enumerate()
            .map(|(row_index, row)| {
                let mut row_stack = Stack::new(Axis::Horizontal).gap(self.column_gap);
                // Fingerprint the row by the keys it holds, so a row whose
                // membership changed remounts while untouched rows survive.
                let mut fingerprint = 0x9E37_79B9_7F4A_7C15u64 ^ row_index as u64;
                for item in row {
                    let key = (self.key_of)(item);
                    fingerprint ^= key;
                    fingerprint = fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
                    let item = item.clone();
                    let build = self.build_item.clone();
                    row_stack = row_stack.child(Child::new(
                        ChildKey::keyed(key),
                        self.item_layout,
                        move || (build)(item.clone()),
                    ));
                }
                Child::new(
                    ChildKey::keyed(fingerprint),
                    LayoutRequest::new().stack(Axis::Horizontal, self.column_gap),
                    move || Box::new(row_stack.clone()),
                )
            })
            .collect()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, self.row_gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl<T: Clone + Send + Sync + 'static> WidgetAny for Grid<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floem_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}

// ---------------------------------------------------------------------------
// Popup
// ---------------------------------------------------------------------------

/// A transient, non-modal layer anchored above its siblings.
///
/// [`Dialog`] is the modal sibling: it carries a titled card and owns the
/// keyboard. `Popup` is the lighter primitive for menus, tooltips and inline
/// pickers — a togglable layer whose content fills the control's box, with no
/// scrim of its own. It is non-modal, so a click that lands beside it reaches
/// whatever is underneath, and the host dismisses it on that click. Like every
/// overlay composite it contributes nothing while hidden.
#[derive(Clone)]
pub struct Popup {
    surface: Surface,
    label: Option<String>,
    content: Option<Child>,
}

impl std::fmt::Debug for Popup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup")
            .field("present", &self.surface.present)
            .field("label", &self.label)
            .finish()
    }
}

impl Popup {
    pub fn new() -> Self {
        Self {
            surface: Surface::new(false),
            label: None,
            content: None,
        }
    }

    /// Sets the popup content (does not show it; call [`Popup::set_present`]).
    pub fn content(mut self, child: Child) -> Self {
        self.content = Some(child);
        self
    }

    /// Sets the content with an explicit key and layout.
    pub fn content_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.content = Some(Child::new(key, layout, build));
        self
    }

    /// A label published to assistive technology while the popup is shown.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows or hides the popup; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Hides the popup and drops its content.
    pub fn dismiss(&mut self) {
        self.content = None;
        self.surface.set_present(false);
    }

    /// Whether the popup is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint {
                role: SemanticsRole::Group,
                label: self.label.clone(),
                state: Default::default(),
            }
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Popup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let content = self.content.clone();
        // A popup is anchored to the control that owns it, so unlike a dialog it
        // takes its own box as the rect the content is laid out against.
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            let content = content.clone()?;
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                false,
                bounds,
                move |_handle| content.build(),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        let content = match &self.content {
            Some(content) => content.clone(),
            None => return Vec::new(),
        };
        // No host overlay: fall back to an in-tree layer, as before.
        let surface = Overlay::new().content_with(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || content.build(),
        );
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// BottomSheet
// ---------------------------------------------------------------------------

/// A modal sheet anchored to the bottom edge, spanning the width.
///
/// Structurally a [`Dialog`] that is pinned bottom instead of centred: a
/// bottom-aligned column over the whole window, holding a sheet with the title
/// and body. `height` sets the sheet's height; it stretches to the full width.
/// Being modal it owns the keyboard while it is up, and escapes its parent's
/// clip because the column is a host overlay.
#[derive(Clone)]
pub struct BottomSheet {
    surface: Surface,
    title: String,
    height: f32,
    body: Option<Child>,
}

impl std::fmt::Debug for BottomSheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BottomSheet")
            .field("present", &self.surface.present)
            .field("height", &self.height)
            .finish()
    }
}

impl BottomSheet {
    pub fn new() -> Self {
        Self {
            surface: Surface::new(false),
            title: String::new(),
            height: 200.0,
            body: None,
        }
    }

    /// The text shown at the top of the sheet.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The sheet's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// The sheet body (any widget subtree).
    pub fn body(mut self, body: Child) -> Self {
        self.body = Some(body);
        self
    }

    /// The sheet body built lazily with an explicit key and layout.
    pub fn body_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.body = Some(Child::new(key, layout, build));
        self
    }

    /// Shows or hides the sheet; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Whether the sheet is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    /// The bottom-aligned column: a fixed-height sheet (full width) pinned to
    /// the bottom edge of the box it is given.
    fn column(&self) -> Stack {
        let title = self.title.clone();
        let height = self.height;
        let body = self.body.clone().unwrap_or_else(empty_body);
        let sheet = Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(Text::new(title.clone())),
            ))
            .child(body);
        let sheet_child = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().height(height),
            move || Box::new(sheet.clone()),
        );
        Stack::new(Axis::Vertical)
            .align(Align::End, Align::Stretch)
            .child(sheet_child)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint::labeled(SemanticsRole::Dialog, self.title.clone())
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for BottomSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for BottomSheet {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let column = self.column();
        // A sheet is pinned to the bottom of the *window*, not of the box it was
        // declared in, so it is sized from the host viewport.
        self.surface.reconcile(cx, id, move |_bounds, viewport| {
            Some(OverlayRequest::new(
                OverlayKind::BottomSheet,
                true,
                viewport,
                move |_handle| Box::new(column),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        let column = self.column();
        // No host overlay: fall back to an in-tree layer, as before.
        let surface = Overlay::new().content_with(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || Box::new(column.clone()),
        );
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_core::{Axis, Position};

    #[test]
    fn stack_reports_its_children_and_axis() {
        let stack = Stack::new(Axis::Vertical)
            .gap(4.0)
            .push(|| Box::new(Text::new("a")))
            .push(|| Box::new(Text::new("b")));
        let children = stack.children();
        assert_eq!(children.len(), 2);
        // Keys default to position, so re-keys survive reordering of slots.
        assert_eq!(children[0].key, ChildKey::index(0));
        assert_eq!(children[1].key, ChildKey::index(1));
        let req = stack.layout_request().expect("stack declares its axis");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(req.gap, Some(4.0));
    }

    #[test]
    fn stack_keyed_children_keep_stable_ids() {
        let stack = Stack::new(Axis::Horizontal)
            .push_keyed(7, || Box::new(Text::new("x")))
            .push_keyed(9, || Box::new(Text::new("y")));
        let children = stack.children();
        assert_eq!(children[0].key, ChildKey::keyed(7));
        assert_eq!(children[1].key, ChildKey::keyed(9));
    }

    #[test]
    fn list_mirrors_items_into_keyed_rows() {
        let list = List::<u32>::new(
            Axis::Vertical,
            vec![10, 20, 30],
            |item: &u32| *item as u64,
            |item: u32| Box::new(Text::new(item.to_string())),
        );
        let children = list.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].key, ChildKey::keyed(10));
        assert_eq!(children[2].key, ChildKey::keyed(30));
    }

    #[test]
    fn conditional_shows_only_when_true() {
        let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("x"))
        });
        assert_eq!(
            Conditional::new(true).show(child.clone()).children().len(),
            1
        );
        assert_eq!(Conditional::new(false).show(child).children().len(), 0);
    }

    #[test]
    fn overlay_is_absolute_and_toggles() {
        let overlay =
            Overlay::new().content(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("x"))
            }));
        assert_eq!(
            overlay.layout_request().unwrap().position,
            Some(Position::Absolute)
        );
        assert_eq!(overlay.children().len(), 1);
        assert_eq!(Overlay::new().children().len(), 0);
    }

    #[test]
    fn dialog_is_empty_when_hidden_and_present_when_shown() {
        let dialog = Dialog::new().title("Hi").body(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            || Box::new(Text::new("body")),
        ));
        assert_eq!(dialog.children().len(), 0, "hidden dialog mounts nothing");
        let mut shown = dialog;
        shown.set_present(true);
        assert_eq!(
            shown.children().len(),
            1,
            "present dialog mounts its surface"
        );
    }

    #[test]
    fn tabs_render_bar_and_panel() {
        let tabs = Tabs::new()
            .tab(
                "One",
                Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                    Box::new(Text::new("p1"))
                }),
            )
            .tab(
                "Two",
                Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                    Box::new(Text::new("p2"))
                }),
            );
        let children = tabs.children();
        assert_eq!(children.len(), 2, "bar + active panel");
        let req = tabs
            .layout_request()
            .expect("tabs declare a vertical stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
    }

    #[test]
    fn grid_chunks_items_into_rows() {
        let grid = Grid::<u32>::new(
            2,
            vec![1, 2, 3, 4, 5],
            |item: &u32| *item as u64,
            |item: u32| Box::new(Text::new(item.to_string())),
        );
        let rows = grid.children();
        assert_eq!(rows.len(), 3, "5 items over 2 columns => 3 rows");
        let req = grid.layout_request().expect("grid stacks rows vertically");
        assert_eq!(req.axis, Some(Axis::Vertical));
    }

    #[test]
    fn grid_rows_remount_only_when_membership_changes() {
        let mut grid = Grid::<u32>::new(
            2,
            vec![1, 2, 3, 4],
            |item: &u32| *item as u64,
            |item: u32| Box::new(Text::new(item.to_string())),
        );
        let before: Vec<_> = grid.children().iter().map(|c| c.key).collect();
        // Append a full new row: earlier rows keep their fingerprints.
        grid.set_items(vec![1, 2, 3, 4, 5, 6]);
        let after: Vec<_> = grid.children().iter().map(|c| c.key).collect();
        assert_eq!(before[..2], after[..2], "untouched rows keep their keys");
        assert_eq!(after.len(), 3);
    }

    #[test]
    fn popup_is_empty_until_shown() {
        let popup =
            Popup::new().content(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("menu"))
            }));
        assert_eq!(popup.children().len(), 0, "hidden popup mounts nothing");
        let mut shown = popup;
        shown.set_present(true);
        assert_eq!(shown.children().len(), 1, "shown popup mounts its layer");
        assert_eq!(
            shown.layout_request().unwrap().position,
            Some(Position::Absolute)
        );
    }

    #[test]
    fn bottom_sheet_mounts_a_bottom_aligned_surface() {
        let sheet = BottomSheet::new()
            .title("Share")
            .height(180.0)
            .body(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("body"))
            }));
        assert_eq!(sheet.children().len(), 0, "hidden sheet mounts nothing");
        let mut shown = sheet;
        shown.set_present(true);
        assert_eq!(shown.children().len(), 1, "shown sheet mounts its layer");
    }
}
