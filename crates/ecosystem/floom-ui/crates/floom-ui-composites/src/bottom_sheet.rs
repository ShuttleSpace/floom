//! A modal sheet anchored to the bottom edge, spanning the width.
//!
//! A dialog that slides in from an edge. Its drag handle and corner radii are style; the scrim and the dismissal are the ones a dialog already had.
//!
//! It lived in `floom-widgets/src/compose.rs` beside the layout vocabulary
//! (`Stack`, `List`, `Grid`, `Conditional`, `Crossfade`). The two are not the
//! same kind of thing: a `Stack` arranges children along an axis, which is
//! arithmetic; this decides *what a control looks like*, which is a product
//! decision. So it moved up a layer with the tree, the counter and the select,
//! and the dependency runs one way -- this crate composes the engine's
//! vocabulary rather than the other way round.

use std::any::Any;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, OverlayHandle,
    OverlayKind, OverlayRequest, PaintContext, SemanticsContext, UpdateContext, Widget, WidgetAny,
};

use floom_widgets::primitives::{SemanticsHint, Text};
use floom_widgets::Stack;

use crate::overlay::Overlay;
use crate::overlay_bookkeeping::{empty_body, OverlayHost};

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
    surface: OverlayHost,
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
            surface: OverlayHost::new(false),
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
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::Dialog;
    use crate::popup::Popup;
    use crate::tabs::Tabs;
    use floom_core::{Axis, Position};
    use floom_widgets::{Conditional, Grid, List};

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
