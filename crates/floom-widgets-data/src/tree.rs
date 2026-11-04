//! Virtualized tree.
//!
//! A tree is a flat list plus a depth, and that is exactly how this is built:
//! [`VecTree`] owns the hierarchy and its expansion set, and [`Tree`] views the
//! *flattening* of that hierarchy through the same [`VirtualViewport`] the grid
//! uses. Expansion is therefore a model operation — collapsing a node removes
//! rows from the display list — rather than a visibility flag the view has to
//! honour during paint.

use std::any::Any;
use std::collections::BTreeSet;

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, FontWeight, Key, KeyPhase, LayoutRequest,
    NamedKey, PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::{Modifiers, PointerButton, PointerPhase};
use floom_virtual::{ItemSizes, Overscan, VirtualViewport};
use floom_widgets::{SemanticsHint, to_core_role, to_core_state};

use crate::data_grid::RowPool;
use crate::model::{SelectionMode, SelectionModel};

const DEFAULT_ROW_HEIGHT: f64 = 20.0;
/// Horizontal room each depth level claims, and within it the disclosure
/// triangle's own column.
const DEFAULT_INDENT: f64 = 16.0;
const LABEL_FONT_SIZE: f32 = 13.0;
const LINE_BOX: f64 = LABEL_FONT_SIZE as f64 * 1.2;

const ROW_BG: Color = Color::rgb(0xFF, 0xFF, 0xFF);
const ROW_ALT_BG: Color = Color::rgb(0xF7, 0xF9, 0xFB);
const ROW_FG: Color = Color::rgb(0x24, 0x2A, 0x31);
const SELECTED_BG: Color = Color::rgb(0xD6, 0xE4, 0xF0);
const DISCLOSURE_FG: Color = Color::rgb(0x5A, 0x63, 0x6C);

fn text_y(top: f64, height: f64) -> f64 {
    top + (height - LINE_BOX) / 2.0
}

/// Stable identity of a tree node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TreeNodeId(pub u64);

impl TreeNodeId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// One node and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub id: TreeNodeId,
    pub label: String,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(id: TreeNodeId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn child_many(mut self, children: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

/// One row of the flattened view.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatRow {
    pub id: TreeNodeId,
    pub label: String,
    /// Distance from the root, along the *visible* path.
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
}

/// A hierarchy plus its expansion set — the headless half of the tree.
#[derive(Debug, Clone, Default)]
pub struct VecTree {
    roots: Vec<TreeNode>,
    expanded: BTreeSet<TreeNodeId>,
}

impl VecTree {
    pub fn new(roots: Vec<TreeNode>) -> Self {
        Self {
            roots,
            expanded: BTreeSet::new(),
        }
    }

    pub fn roots(&self) -> &[TreeNode] {
        &self.roots
    }

    pub fn find(&self, id: TreeNodeId) -> Option<&TreeNode> {
        fn walk(nodes: &[TreeNode], id: TreeNodeId) -> Option<&TreeNode> {
            for node in nodes {
                if node.id == id {
                    return Some(node);
                }
                if let Some(found) = walk(&node.children, id) {
                    return Some(found);
                }
            }
            None
        }
        walk(&self.roots, id)
    }

    pub fn is_expanded(&self, id: TreeNodeId) -> bool {
        self.expanded.contains(&id)
    }

    /// Total nodes, expanded or not.
    pub fn node_count(&self) -> usize {
        fn walk(nodes: &[TreeNode]) -> usize {
            nodes.iter().map(|node| 1 + walk(&node.children)).sum()
        }
        walk(&self.roots)
    }

    /// How many nodes that *could* be open actually are.
    pub fn expanded_count(&self) -> usize {
        fn walk(nodes: &[TreeNode], expanded: &BTreeSet<TreeNodeId>) -> usize {
            nodes
                .iter()
                .map(|node| {
                    let self_counts =
                        usize::from(node.has_children() && expanded.contains(&node.id));
                    self_counts + walk(&node.children, expanded)
                })
                .sum()
        }
        walk(&self.roots, &self.expanded)
    }

    /// A leaf has nothing to disclose, so asking to open it is refused rather
    /// than silently recorded as an expanded leaf.
    pub fn set_expanded(&mut self, id: TreeNodeId, expanded: bool) -> bool {
        let Some(node) = self.find(id) else {
            return false;
        };
        if !node.has_children() {
            return false;
        }
        if expanded {
            self.expanded.insert(id)
        } else {
            self.expanded.remove(&id)
        }
    }

    pub fn expand(&mut self, id: TreeNodeId) -> bool {
        self.set_expanded(id, true)
    }

    /// Collapsing hides the whole subtree but forgets nothing: the descendants
    /// keep their own expansion state, so re-opening the parent restores what
    /// the user had open instead of resetting it.
    pub fn collapse(&mut self, id: TreeNodeId) -> bool {
        self.set_expanded(id, false)
    }

    pub fn toggle(&mut self, id: TreeNodeId) -> bool {
        self.set_expanded(id, !self.is_expanded(id))
    }

    pub fn expand_all(&mut self) -> bool {
        fn walk(nodes: &[TreeNode], out: &mut BTreeSet<TreeNodeId>) {
            for node in nodes {
                if node.has_children() {
                    out.insert(node.id);
                    walk(&node.children, out);
                }
            }
        }
        let mut all = BTreeSet::new();
        walk(&self.roots, &mut all);
        if all == self.expanded {
            return false;
        }
        self.expanded = all;
        true
    }

    pub fn collapse_all(&mut self) -> bool {
        if self.expanded.is_empty() {
            return false;
        }
        self.expanded.clear();
        true
    }

    /// Visible rows, depth-first, with a closed node's descendants omitted.
    pub fn flatten(&self) -> Vec<FlatRow> {
        fn walk(
            nodes: &[TreeNode],
            depth: usize,
            expanded: &BTreeSet<TreeNodeId>,
            out: &mut Vec<FlatRow>,
        ) {
            for node in nodes {
                let open = node.has_children() && expanded.contains(&node.id);
                out.push(FlatRow {
                    id: node.id,
                    label: node.label.clone(),
                    depth,
                    has_children: node.has_children(),
                    expanded: open,
                });
                if open {
                    walk(&node.children, depth + 1, expanded, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.roots, 0, &self.expanded, &mut out);
        out
    }
}

/// A virtualized tree view.
pub struct Tree {
    model: VecTree,
    rows: Vec<FlatRow>,
    /// Row ids in display order, kept alongside `rows` so navigation does not
    /// have to re-project the whole flattening on every key press.
    ids: Vec<TreeNodeId>,
    row_height: f64,
    indent: f64,
    container: Rect,
    viewport: VirtualViewport<TreeNodeId, ElementId>,
    selection: SelectionModel<TreeNodeId>,
    focused: bool,
    pending_repaint: bool,
}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("visible_rows", &self.rows.len())
            .field("expanded", &self.model.expanded_count())
            .finish_non_exhaustive()
    }
}

impl Tree {
    pub fn new(model: VecTree) -> Self {
        let mut tree = Self {
            model,
            rows: Vec::new(),
            ids: Vec::new(),
            row_height: DEFAULT_ROW_HEIGHT,
            indent: DEFAULT_INDENT,
            container: Rect::ZERO,
            viewport: VirtualViewport::new(ItemSizes::uniform(DEFAULT_ROW_HEIGHT as f32, 0), 0.0),
            selection: SelectionModel::new(SelectionMode::Single),
            focused: false,
            pending_repaint: true,
        };
        tree.recompute();
        tree
    }

    pub fn row_height(mut self, height: f64) -> Self {
        self.row_height = height;
        self.recompute();
        self
    }

    pub fn indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self
    }

    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection.set_mode(mode);
        self
    }

    pub fn overscan(mut self, overscan: Overscan) -> Self {
        self.viewport.set_overscan(overscan);
        self
    }

    pub fn model(&self) -> &VecTree {
        &self.model
    }

    /// Every visible row, flattened.
    pub fn rows(&self) -> &[FlatRow] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn row_at_index(&self, index: usize) -> Option<&FlatRow> {
        self.rows.get(index)
    }

    pub fn selection(&self) -> &SelectionModel<TreeNodeId> {
        &self.selection
    }

    pub const fn container(&self) -> Rect {
        self.container
    }

    pub fn is_expanded(&self, id: TreeNodeId) -> bool {
        self.model.is_expanded(id)
    }

    pub fn expanded_count(&self) -> usize {
        self.model.expanded_count()
    }

    pub fn expand(&mut self, id: TreeNodeId) -> bool {
        self.mutate(|model| model.expand(id))
    }

    pub fn collapse(&mut self, id: TreeNodeId) -> bool {
        self.mutate(|model| model.collapse(id))
    }

    pub fn toggle(&mut self, id: TreeNodeId) -> bool {
        self.mutate(|model| model.toggle(id))
    }

    pub fn expand_all(&mut self) -> bool {
        self.mutate(|model| model.expand_all())
    }

    pub fn collapse_all(&mut self) -> bool {
        self.mutate(|model| model.collapse_all())
    }

    fn mutate(&mut self, change: impl FnOnce(&mut VecTree) -> bool) -> bool {
        if !change(&mut self.model) {
            return false;
        }
        self.recompute();
        self.pending_repaint = true;
        true
    }

    pub fn set_container(&mut self, rect: Rect) -> bool {
        self.container = rect;
        let before = self.viewport.visible_range();
        self.viewport
            .set_viewport_extent(rect.height().max(0.0) as f32);
        let clamped = self.clamp_scroll();
        before != self.viewport.visible_range() || clamped
    }

    pub fn set_overscan(&mut self, overscan: Overscan) -> bool {
        if self.viewport.overscan() == overscan {
            return false;
        }
        self.viewport.set_overscan(overscan);
        self.pending_repaint = true;
        true
    }

    pub fn visible_range(&self) -> std::ops::Range<usize> {
        self.viewport.visible_range()
    }

    pub fn materialized_count(&self) -> usize {
        self.viewport.visible_range().len()
    }

    pub fn visible_rows(&self) -> Vec<FlatRow> {
        self.rows[self.viewport.visible_range()].to_vec()
    }

    pub fn bound_element(&self, id: TreeNodeId) -> Option<ElementId> {
        self.viewport.get(&id)
    }

    pub fn scroll_offset(&self) -> f64 {
        f64::from(self.viewport.scroll_offset())
    }

    pub fn max_scroll(&self) -> f64 {
        let content = self.rows.len() as f64 * self.row_height;
        (content - f64::from(self.viewport.viewport_extent())).max(0.0)
    }

    pub fn scroll_to(&mut self, offset: f64) -> bool {
        let offset = offset.clamp(0.0, self.max_scroll());
        if (self.scroll_offset() - offset).abs() < f64::EPSILON {
            return false;
        }
        self.viewport.scroll_to(offset as f32);
        self.pending_repaint = true;
        true
    }

    pub fn scroll_by(&mut self, delta: f64) -> bool {
        self.scroll_to(self.scroll_offset() + delta)
    }

    pub fn sync_rows(&mut self, pool: &mut RowPool) -> usize {
        let Self { viewport, rows, .. } = self;
        let diff = viewport.update(|index| rows[index].id);
        for (_, element) in diff.exit {
            pool.release(element);
        }
        for (_, key) in diff.enter {
            let element = pool.acquire();
            viewport.assign(key, element);
        }
        viewport.materialized_count()
    }

    /// The disclosure triangle's box for the row at `index`.
    ///
    /// It is one indent wide and starts at the row's own depth offset, which is
    /// what makes "click the triangle to open" a positional test rather than a
    /// hit test against painted glyphs.
    pub fn row_disclosure_bounds(&self, container: Rect, index: usize) -> Option<Rect> {
        let row = self.rows.get(index)?;
        let bounds = self.row_bounds_in(container, index)?;
        let x0 = container.x0 + row.depth as f64 * self.indent;
        Some(Rect {
            x0,
            y0: bounds.y0,
            x1: x0 + self.indent,
            y1: bounds.y1,
        })
    }

    pub fn row_bounds(&self, index: usize) -> Option<Rect> {
        self.row_bounds_in(self.container, index)
    }

    fn row_bounds_in(&self, container: Rect, index: usize) -> Option<Rect> {
        let top = container.y0 + index as f64 * self.row_height - self.scroll_offset();
        let bottom = top + self.row_height;
        if bottom <= container.y0 || top >= container.y1 {
            return None;
        }
        Some(Rect {
            x0: container.x0,
            y0: top.max(container.y0),
            x1: container.x1,
            y1: bottom.min(container.y1),
        })
    }

    fn row_at_in(&self, container: Rect, y: f64) -> Option<usize> {
        let offset = y - container.y0 + self.scroll_offset();
        if offset < 0.0 || self.row_height <= 0.0 {
            return None;
        }
        let index = (offset / self.row_height).floor() as usize;
        (index < self.rows.len()).then_some(index)
    }

    pub fn select_row(&mut self, index: usize) -> bool {
        let changed = self.selection.move_to(index, &self.ids);
        if changed {
            self.pending_repaint = true;
        }
        changed
    }

    fn recompute(&mut self) {
        self.rows = self.model.flatten();
        self.viewport
            .set_sizes(ItemSizes::uniform(self.row_height as f32, self.rows.len()));
        // `ids` mirrors `rows` so a key press does not re-project the whole
        // flattening; it has to be refreshed here, or every cursor move runs
        // against a stale list (and against an empty one on a fresh tree).
        self.ids.clear();
        self.ids.extend(self.rows.iter().map(|row| row.id));
        if self.selection.clamp_to(&self.ids) {
            self.pending_repaint = true;
        }
    }

    fn clamp_scroll(&mut self) -> bool {
        let max = self.max_scroll();
        if self.scroll_offset() > max {
            self.viewport.scroll_to(max as f32);
            self.pending_repaint = true;
            return true;
        }
        false
    }

    fn row_index(&self, id: TreeNodeId) -> Option<usize> {
        self.rows.iter().position(|row| row.id == id)
    }

    fn cursor_row(&self) -> Option<(usize, FlatRow)> {
        let index = self.row_index(self.selection.cursor()?)?;
        Some((index, self.rows[index].clone()))
    }

    fn scroll_cursor_into_view(&mut self) -> bool {
        let Some(index) = self.row_index_of_cursor() else {
            return false;
        };
        let top = index as f64 * self.row_height;
        let extent = f64::from(self.viewport.viewport_extent());
        if top < self.scroll_offset() {
            return self.scroll_to(top);
        }
        if top + self.row_height > self.scroll_offset() + extent {
            return self.scroll_to(top + self.row_height - extent);
        }
        false
    }

    fn row_index_of_cursor(&self) -> Option<usize> {
        self.row_index(self.selection.cursor()?)
    }

    fn focus(&mut self, cx: &mut dyn EventContext, id: ElementId) {
        if !self.focused {
            self.focused = true;
            cx.request_focus(id);
            self.pending_repaint = true;
        }
    }

    fn pointer(
        &mut self,
        pointer: &floom_types::PointerEvent,
        container: Rect,
        cx: &mut dyn EventContext,
        id: ElementId,
    ) -> bool {
        if !matches!(pointer.phase, PointerPhase::Down | PointerPhase::Up) {
            return false;
        }
        self.focus(cx, id);
        if pointer.phase != PointerPhase::Down || pointer.button != Some(PointerButton::Primary) {
            return true;
        }

        let Some(index) = self.row_at_in(container, pointer.position.y) else {
            return true;
        };
        let local_x = pointer.position.x - container.x0;
        let (depth, has_children, node) = {
            let row = &self.rows[index];
            (row.depth, row.has_children, row.id)
        };

        // The disclosure column is the first indent of the row's own offset.
        let left = depth as f64 * self.indent;
        if has_children && local_x >= left && local_x < left + self.indent {
            return self.toggle(node);
        }
        if pointer.modifiers.contains(Modifiers::SHIFT) {
            let changed = self.selection.extend_to(index, &self.ids);
            if changed {
                self.pending_repaint = true;
            }
            return changed;
        }
        self.select_row(index)
    }

    fn key_event(
        &mut self,
        key: &floom_types::KeyEvent,
        cx: &mut dyn EventContext,
        id: ElementId,
    ) -> bool {
        if key.phase == KeyPhase::Up {
            return false;
        }
        self.focus(cx, id);
        let shift = key.modifiers.contains(Modifiers::SHIFT);

        let moved = match key.key {
            Key::Named(NamedKey::ArrowDown) => self.selection.navigate(1, &self.ids, shift),
            Key::Named(NamedKey::ArrowUp) => self.selection.navigate(-1, &self.ids, shift),
            Key::Named(NamedKey::Home) => self.selection.jump(0, &self.ids, shift),
            Key::Named(NamedKey::End) => {
                self.selection
                    .jump(self.ids.len().saturating_sub(1), &self.ids, shift)
            }
            Key::Named(NamedKey::ArrowRight) => return self.drill_in(),
            Key::Named(NamedKey::ArrowLeft) => return self.drill_out(),
            _ => return false,
        };
        if moved {
            self.pending_repaint = true;
            self.scroll_cursor_into_view();
        }
        moved
    }

    /// Right: open a closed node, otherwise step into its first child.
    fn drill_in(&mut self) -> bool {
        let Some((index, row)) = self.cursor_row() else {
            return false;
        };
        if !row.has_children {
            return false;
        }
        if !row.expanded {
            return self.expand(row.id);
        }
        if index + 1 >= self.rows.len() {
            return false;
        }
        self.select_row(index + 1)
    }

    /// Left: close an open node, otherwise walk up to the parent row.
    fn drill_out(&mut self) -> bool {
        let Some((index, row)) = self.cursor_row() else {
            return false;
        };
        if row.has_children && row.expanded {
            return self.collapse(row.id);
        }
        if row.depth == 0 {
            return false;
        }
        // The parent is the nearest preceding row shallower than this one.
        let parent = self.rows[..index]
            .iter()
            .rposition(|candidate| candidate.depth < row.depth);
        match parent {
            Some(parent) => self.select_row(parent),
            None => false,
        }
    }

    fn semantics_hint(&self) -> SemanticsHint {
        let mut state =
            SemanticsState::ENABLED | SemanticsState::FOCUSABLE | SemanticsState::SCROLLABLE;
        if self.focused {
            state |= SemanticsState::FOCUSED;
        }
        SemanticsHint {
            role: SemanticsRole::List,
            label: Some(format!(
                "{} rows, {} expanded",
                self.rows.len(),
                self.model.expanded_count()
            )),
            state,
        }
    }
}

impl Widget for Tree {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() > 0.0 && bounds.height() > 0.0 {
            self.set_container(bounds);
        }
        if self.pending_repaint {
            self.pending_repaint = false;
            cx.mark_dirty(id);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().fill())
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let bounds = cx.bounds();
        if bounds.width() > 0.0 && bounds.height() > 0.0 {
            self.set_container(bounds);
        }
        match cx.event() {
            Event::Pointer(pointer) => self.pointer(&pointer, bounds, cx, id),
            Event::Key(key) => self.key_event(&key, cx, id),
            Event::Scroll(scroll) => self.scroll_by(-scroll.delta_y),
            Event::Ime(_) => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        for index in self.viewport.visible_range() {
            let Some(row) = self.rows.get(index) else {
                continue;
            };
            let Some(row_bounds) = self.row_bounds_in(bounds, index) else {
                continue;
            };
            let background = if self.selection.is_selected(row.id) {
                SELECTED_BG
            } else if index % 2 == 1 {
                ROW_ALT_BG
            } else {
                ROW_BG
            };
            cx.fill_rect(row_bounds, background);

            let depth_x = bounds.x0 + row.depth as f64 * self.indent;
            if row.has_children {
                let glyph = if row.expanded { '▼' } else { '▶' };
                cx.fill_text(
                    Point::new(depth_x + 2.0, text_y(row_bounds.y0, self.row_height)),
                    &glyph.to_string(),
                    LABEL_FONT_SIZE,
                    FontWeight::Normal,
                    DISCLOSURE_FG,
                );
            }
            cx.fill_text(
                Point::new(
                    depth_x + self.indent,
                    text_y(row_bounds.y0, self.row_height),
                ),
                &row.label,
                LABEL_FONT_SIZE,
                FontWeight::Normal,
                ROW_FG,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        let hint = self.semantics_hint();
        if let Some(label) = hint.label.as_deref() {
            cx.set_label(label);
        }
        cx.set_bounds(cx.bounds());
    }
}

impl WidgetAny for Tree {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let hint = self.semantics_hint();
        Some(floom_core::SemanticsHint {
            role: to_core_role(hint.role),
            label: hint.label,
            state: to_core_state(hint.state),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> VecTree {
        VecTree::new(vec![
            TreeNode::new(TreeNodeId(1), "a")
                .child(TreeNode::new(TreeNodeId(2), "b").child(TreeNode::new(TreeNodeId(3), "c")))
                .child(TreeNode::new(TreeNodeId(4), "d")),
        ])
    }

    #[test]
    fn flattening_reports_the_visible_path_depth() {
        let mut model = tree();
        assert_eq!(
            model.flatten().iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![TreeNodeId(1)]
        );
        model.expand(TreeNodeId(1));
        model.expand(TreeNodeId(2));
        let rows = model.flatten();
        assert_eq!(
            rows.iter().map(|r| (r.id, r.depth)).collect::<Vec<_>>(),
            vec![
                (TreeNodeId(1), 0),
                (TreeNodeId(2), 1),
                (TreeNodeId(3), 2),
                (TreeNodeId(4), 1)
            ]
        );
    }

    #[test]
    fn node_and_expanded_counts_ignore_invisible_state() {
        let mut model = tree();
        assert_eq!(model.node_count(), 4);
        assert_eq!(model.expanded_count(), 0);
        model.expand_all();
        assert_eq!(model.expanded_count(), 2, "only nodes with children count");
        model.collapse(TreeNodeId(1));
        assert_eq!(
            model.expanded_count(),
            1,
            "the hidden inner node is still open"
        );
        assert_eq!(
            model.node_count(),
            4,
            "collapsing hides, it does not delete"
        );
    }

    #[test]
    fn a_missing_node_id_cannot_be_expanded() {
        let mut model = tree();
        assert!(!model.expand(TreeNodeId(99)));
        assert!(!model.collapse(TreeNodeId(99)));
    }
}
