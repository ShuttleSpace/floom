//! Navigation chrome — Tier-2 widgets.
//!
//! [`NavigationBar`] (a bottom bar), [`NavigationRail`] (a side rail) and
//! [`NavigationDrawer`] (a modal side panel) all present the same thing: a list
//! of [`NavDestination`]s and a single selected index held in a shared
//! `Arc<AtomicUsize>`, so a destination button can flip selection from its own
//! `on_click` and `update` notices the change.
//!
//! Following [`crate::compose::Tabs`], the active index is mixed into every
//! button's [`ChildKey`]. Flipping selection therefore re-keys the strip, the
//! host remounts it, and the highlight updates — no in-place mutation of a
//! mounted button is needed.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, OverlayHandle,
    OverlayKind, OverlayRequest, PaintContext, SemanticsContext, UpdateContext, Widget, WidgetAny,
};

use crate::compose::{Overlay, OverlaySurface, Stack};
use crate::controls::Button;
use crate::primitives::{SemanticsHint, Text};

/// One selectable navigation destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavDestination {
    label: String,
}

impl NavDestination {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl From<&str> for NavDestination {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for NavDestination {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

/// Shared selected-index handle.
type Selection = Arc<AtomicUsize>;

// Distinct key tags so strips that share index space never collide.
const BAR_TAG: u64 = 0x4E41_5600;
const RAIL_TAG: u64 = 0x4E41_5601;
const DRAWER_TAG: u64 = 0x4E41_5602;

/// Builds the buttons for a nav strip.
///
/// Each button carries `cur` in its key so a selection change re-keys the whole
/// strip and the `●` highlight re-renders. Horizontal strips make their buttons
/// grow (equal widths); vertical strips give every item a fixed height and let
/// the cross-axis stretch set its width.
fn nav_buttons(
    items: &[NavDestination],
    selection: &Selection,
    cur: usize,
    axis: Axis,
    tag: u64,
) -> Vec<Child> {
    let horizontal = axis == Axis::Horizontal;
    items
        .iter()
        .enumerate()
        .map(|(i, dest)| {
            let sel = selection.clone();
            let active = i == cur;
            let label = if active {
                format!("● {}", dest.label)
            } else {
                dest.label.clone()
            };
            let mut layout = LayoutRequest::new().height(48.0);
            if horizontal {
                layout = layout.grow(1.0);
            }
            Child::new(
                ChildKey::keyed(tag ^ (i as u64) ^ ((cur as u64) << 32)),
                layout,
                move || {
                    let sel = sel.clone();
                    Box::new(
                        Button::new(label.clone()).on_click(move || sel.store(i, Ordering::SeqCst)),
                    )
                },
            )
        })
        .collect()
}

fn tab_bar_hint() -> SemanticsHint {
    SemanticsHint {
        role: SemanticsRole::TabBar,
        label: None,
        state: SemanticsState::empty(),
    }
}

// ---------------------------------------------------------------------------
// NavigationBar
// ---------------------------------------------------------------------------

/// A horizontal navigation bar of equally sized destinations.
#[derive(Clone)]
pub struct NavigationBar {
    items: Vec<NavDestination>,
    selection: Selection,
    last_selected: usize,
}

impl std::fmt::Debug for NavigationBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationBar")
            .field("items", &self.items.len())
            .finish()
    }
}

impl NavigationBar {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selection: Arc::new(AtomicUsize::new(0)),
            last_selected: usize::MAX,
        }
    }

    /// Appends one destination.
    pub fn destination(mut self, destination: impl Into<NavDestination>) -> Self {
        self.items.push(destination.into());
        self
    }

    /// Appends several destinations.
    pub fn destinations(mut self, destinations: impl IntoIterator<Item = NavDestination>) -> Self {
        self.items.extend(destinations);
        self
    }

    /// Shared selection handle (also driven by the destination buttons).
    pub fn selection(&self) -> Selection {
        self.selection.clone()
    }

    /// Selects a destination; the next frame re-reconciles if it changed.
    pub fn set_selected(&mut self, index: usize) {
        self.selection.store(index, Ordering::SeqCst);
    }

    /// The currently selected index, clamped to the item count.
    pub fn selected(&self) -> usize {
        self.current()
    }

    fn current(&self) -> usize {
        if self.items.is_empty() {
            return 0;
        }
        self.selection
            .load(Ordering::SeqCst)
            .min(self.items.len() - 1)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        tab_bar_hint()
    }
}

impl Default for NavigationBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for NavigationBar {
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
        if self.items.is_empty() {
            return Vec::new();
        }
        nav_buttons(
            &self.items,
            &self.selection,
            self.current(),
            Axis::Horizontal,
            BAR_TAG,
        )
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .stack(Axis::Horizontal, 0.0)
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
// NavigationRail
// ---------------------------------------------------------------------------

/// A vertical navigation rail of fixed-height destinations.
#[derive(Clone)]
pub struct NavigationRail {
    items: Vec<NavDestination>,
    selection: Selection,
    last_selected: usize,
}

impl std::fmt::Debug for NavigationRail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationRail")
            .field("items", &self.items.len())
            .finish()
    }
}

impl NavigationRail {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selection: Arc::new(AtomicUsize::new(0)),
            last_selected: usize::MAX,
        }
    }

    pub fn destination(mut self, destination: impl Into<NavDestination>) -> Self {
        self.items.push(destination.into());
        self
    }

    pub fn destinations(mut self, destinations: impl IntoIterator<Item = NavDestination>) -> Self {
        self.items.extend(destinations);
        self
    }

    pub fn selection(&self) -> Selection {
        self.selection.clone()
    }

    pub fn set_selected(&mut self, index: usize) {
        self.selection.store(index, Ordering::SeqCst);
    }

    pub fn selected(&self) -> usize {
        self.current()
    }

    fn current(&self) -> usize {
        if self.items.is_empty() {
            return 0;
        }
        self.selection
            .load(Ordering::SeqCst)
            .min(self.items.len() - 1)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        tab_bar_hint()
    }
}

impl Default for NavigationRail {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for NavigationRail {
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
        if self.items.is_empty() {
            return Vec::new();
        }
        nav_buttons(
            &self.items,
            &self.selection,
            self.current(),
            Axis::Vertical,
            RAIL_TAG,
        )
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .stack(Axis::Vertical, 0.0)
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
// NavigationDrawer
// ---------------------------------------------------------------------------

/// A modal side panel: a header plus a vertical list of destinations.
///
/// Hidden while `present` is false, so it contributes nothing to layout. When
/// shown it covers the *window* — not the box it was declared in — with an
/// absolute scrim and slides a full-height panel in from the leading edge (via
/// a start-aligned column). Like the other modal composites it hands that
/// surface to the host as an overlay, so a drawer declared inside a scrolled
/// list still covers the screen; a host with no overlay support gets the same
/// layer mounted in-tree.
#[derive(Clone)]
pub struct NavigationDrawer {
    surface: OverlaySurface,
    header: Option<String>,
    width: f32,
    items: Vec<NavDestination>,
    selection: Selection,
    last_selected: usize,
}

impl std::fmt::Debug for NavigationDrawer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationDrawer")
            .field("present", &self.surface.is_shown())
            .field("items", &self.items.len())
            .finish()
    }
}

impl NavigationDrawer {
    pub fn new() -> Self {
        Self {
            surface: OverlaySurface::new(false),
            header: None,
            width: 260.0,
            items: Vec::new(),
            selection: Arc::new(AtomicUsize::new(0)),
            last_selected: usize::MAX,
        }
    }

    /// The title shown above the destinations.
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// The panel width in logical pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn destination(mut self, destination: impl Into<NavDestination>) -> Self {
        self.items.push(destination.into());
        self
    }

    pub fn destinations(mut self, destinations: impl IntoIterator<Item = NavDestination>) -> Self {
        self.items.extend(destinations);
        self
    }

    pub fn selection(&self) -> Selection {
        self.selection.clone()
    }

    pub fn set_selection(&mut self, index: usize) {
        self.selection.store(index, Ordering::SeqCst);
    }

    /// Shows or hides the drawer; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Whether the drawer is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    pub fn selected(&self) -> usize {
        self.current()
    }

    fn current(&self) -> usize {
        if self.items.is_empty() {
            return 0;
        }
        self.selection
            .load(Ordering::SeqCst)
            .min(self.items.len() - 1)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint::labeled(
                SemanticsRole::Dialog,
                self.header
                    .clone()
                    .unwrap_or_else(|| "Navigation".to_string()),
            )
        } else {
            SemanticsHint::decorative()
        }
    }

    /// The drawer's content: a full-height, fixed-width panel pinned to the
    /// leading edge of the box it is given.
    fn column(&self) -> Stack {
        let cur = self.current();
        let width = self.width;
        let header = self.header.clone();
        let mut panel = Stack::new(Axis::Vertical).gap(4.0);
        if let Some(header) = header {
            panel = panel.child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().height(40.0),
                move || Box::new(Text::new(header.clone())),
            ));
        }
        for button in nav_buttons(
            &self.items,
            &self.selection,
            cur,
            Axis::Vertical,
            DRAWER_TAG,
        ) {
            panel = panel.child(button);
        }
        let panel_child = Child::new(
            ChildKey::keyed(DRAWER_TAG),
            LayoutRequest::new().width(width).fill(),
            move || Box::new(panel.clone()),
        );
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Stretch)
            .child(panel_child)
    }
}

impl Default for NavigationDrawer {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for NavigationDrawer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let cur = self.current();
        if cur != self.last_selected {
            cx.mark_children_dirty(id);
            self.last_selected = cur;
        }
        let column = self.column();
        // A drawer covers the window, not the box it was declared in, so it is
        // sized from the host viewport — the same reason a sheet is.
        self.surface.reconcile(cx, id, move |_bounds, viewport| {
            Some(OverlayRequest::new(
                OverlayKind::Drawer,
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
        let cur = self.current();
        let column = self.column();
        // No host overlay: fall back to an in-tree layer, as before.
        let overlay = Overlay::new().content_with(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || Box::new(column.clone()),
        );
        vec![Child::new(
            ChildKey::keyed(DRAWER_TAG ^ cur as u64),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(overlay.clone()),
        )]
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

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_lays_out_horizontally_and_keys_by_selection() {
        let bar = NavigationBar::new()
            .destination("Home")
            .destination("Search");
        let children = bar.children();
        assert_eq!(children.len(), 2);
        let req = bar.layout_request().expect("bar declares a row");
        assert_eq!(req.axis, Some(Axis::Horizontal));

        // The active destination is highlighted, and its key tracks selection.
        let home = &children[0];
        let before = home.key;
        let mut bar = bar;
        bar.set_selected(1);
        let children = bar.children();
        assert_eq!(children.len(), 2);
        assert_ne!(
            children[0].key, before,
            "keys re-mint when selection changes"
        );
    }

    #[test]
    fn rail_lays_out_vertically() {
        let rail = NavigationRail::new()
            .destination("A")
            .destination("B")
            .destination("C");
        assert_eq!(rail.children().len(), 3);
        let req = rail.layout_request().expect("rail declares a column");
        assert_eq!(req.axis, Some(Axis::Vertical));
    }

    #[test]
    fn drawer_is_empty_until_shown() {
        let drawer = NavigationDrawer::new()
            .header("Menu")
            .destination("Home")
            .destination("Settings");
        assert_eq!(drawer.children().len(), 0, "hidden drawer mounts nothing");
        let mut shown = drawer;
        shown.set_present(true);
        assert_eq!(shown.children().len(), 1, "shown drawer mounts its layer");
    }

    #[test]
    fn destinations_convert_from_strings() {
        let bar = NavigationBar::new()
            .destination("Home")
            .destination(String::from("Search"));
        assert_eq!(bar.children().len(), 2);
    }
}
