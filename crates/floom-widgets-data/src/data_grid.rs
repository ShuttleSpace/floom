//! Virtualized data grid.
//!
//! The grid owns three things and delegates the rest: a [`TableModel`] (the
//! data), a [`VirtualViewport`] (which rows exist on screen), and a
//! [`SelectionModel`] plus [`EditHistory`] (what the user did). It paints its
//! rows itself rather than minting a child element per row, because a
//! `Box<dyn TableModel>` is not `Send + Sync` and so cannot be captured by the
//! `ChildBuilder` a composite would need.
//!
//! That choice is only defensible if the cost is genuinely windowed, so it is
//! measured rather than asserted: [`DataGrid::visible_range`] is a pure
//! function of the box and the row height, the paint pass iterates exactly that
//! range, and [`DataGrid::sync_rows`] hands a host the element-backed rows it
//! would need — recycled through a window-sized pool.

use std::any::Any;

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, FontWeight, Key, KeyPhase, LayoutRequest,
    NamedKey, PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::{Modifiers, PointerButton, PointerPhase};
use floom_virtual::{ItemSizes, Overscan, VirtualViewport};
use floom_widgets::{SemanticsHint, to_core_role, to_core_state};

use crate::model::{
    CellEdit, CellValue, Column, ColumnId, EditHistory, EditSession, RowId, SelectionMode,
    SelectionModel, SortSpec, TableModel, sorted_order,
};

/// Which selection gesture a pointer event asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectKind {
    Replace,
    Extend,
    Toggle,
}

const DEFAULT_ROW_HEIGHT: f64 = 24.0;
const DEFAULT_HEADER_HEIGHT: f64 = 28.0;
const CELL_PADDING: f64 = 6.0;
const LABEL_FONT_SIZE: f32 = 13.0;
/// Line box a label occupies, used to centre text in a row.
const LINE_BOX: f64 = LABEL_FONT_SIZE as f64 * 1.2;

const HEADER_BG: Color = Color::rgb(0xF0, 0xF3, 0xF6);
const HEADER_FG: Color = Color::rgb(0x1F, 0x24, 0x2B);
const ROW_BG: Color = Color::rgb(0xFF, 0xFF, 0xFF);
const ROW_ALT_BG: Color = Color::rgb(0xF7, 0xF9, 0xFB);
const ROW_FG: Color = Color::rgb(0x24, 0x2A, 0x31);
const SELECTED_BG: Color = Color::rgb(0xD6, 0xE4, 0xF0);
const ACTIVE_CELL_BORDER: Color = Color::rgb(0x3B, 0x6E, 0xA5);

/// Top of a label's line box, centred inside a band of `height` starting at `top`.
fn text_y(top: f64, height: f64) -> f64 {
    top + (height - LINE_BOX) / 2.0
}

fn emit_semantics(hint: SemanticsHint, cx: &mut dyn SemanticsContext) {
    if hint.role == SemanticsRole::None && hint.label.is_none() {
        return;
    }
    if let Some(label) = hint.label.as_deref() {
        cx.set_label(label);
    }
    cx.set_bounds(cx.bounds());
}

/// Recycles element ids for element-backed rows.
///
/// A grid that only paints does not need this. A host that wants a real
/// element per row (to embed a live widget in a cell) does — and then the
/// question "how many elements does a 100k-row table cost?" has an answer that
/// can be measured: [`RowPool::high_water`].
#[derive(Debug, Default)]
pub struct RowPool {
    free: Vec<ElementId>,
    next: u32,
    in_use: usize,
    allocated: usize,
    high_water: usize,
}

impl RowPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rows currently bound.
    pub fn in_use(&self) -> usize {
        self.in_use
    }

    /// Distinct ids ever minted.
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// The most rows ever bound at once — the pool's real size.
    pub fn high_water(&self) -> usize {
        self.high_water
    }

    pub fn free_count(&self) -> usize {
        self.free.len()
    }

    pub fn acquire(&mut self) -> ElementId {
        let id = match self.free.pop() {
            Some(id) => id,
            None => {
                let id = ElementId::new(self.next, 1);
                self.next += 1;
                self.allocated += 1;
                id
            }
        };
        self.in_use += 1;
        self.high_water = self.high_water.max(self.in_use);
        id
    }

    pub fn release(&mut self, id: ElementId) {
        self.free.push(id);
        self.in_use = self.in_use.saturating_sub(1);
    }
}

/// A virtualized, sortable, editable table.
pub struct DataGrid {
    columns: Vec<Column>,
    model: Box<dyn TableModel>,
    header_height: f64,
    row_height: f64,
    container: Rect,
    viewport: VirtualViewport<RowId, ElementId>,
    selection: SelectionModel<RowId>,
    active_column: ColumnId,
    sort: Option<SortSpec>,
    /// Model indices in display order.
    order: Vec<usize>,
    /// Row ids in display order — the geometry's key space.
    display: Vec<RowId>,
    edit: Option<EditSession>,
    history: EditHistory,
    focused: bool,
    pending_repaint: bool,
}

impl std::fmt::Debug for DataGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataGrid")
            .field("columns", &self.columns)
            .field("rows", &self.model.row_count())
            .field("sort", &self.sort)
            .field("selected", &self.selection.selected_count())
            .field("editing", &self.edit.is_some())
            .finish_non_exhaustive()
    }
}

impl DataGrid {
    pub fn new(columns: Vec<Column>, model: impl TableModel + 'static) -> Self {
        let mut grid = Self {
            columns,
            model: Box::new(model),
            header_height: DEFAULT_HEADER_HEIGHT,
            row_height: DEFAULT_ROW_HEIGHT,
            container: Rect::ZERO,
            viewport: VirtualViewport::new(ItemSizes::uniform(DEFAULT_ROW_HEIGHT as f32, 0), 0.0),
            selection: SelectionModel::new(SelectionMode::Single),
            active_column: ColumnId(0),
            sort: None,
            order: Vec::new(),
            display: Vec::new(),
            edit: None,
            history: EditHistory::new(),
            focused: false,
            pending_repaint: true,
        };
        grid.refresh_sizes();
        grid.recompute();
        grid
    }

    pub fn row_height(mut self, height: f64) -> Self {
        self.row_height = height;
        self.refresh_sizes();
        self
    }

    pub fn header_height(mut self, height: f64) -> Self {
        self.header_height = height;
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

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    pub fn column(&self, column: ColumnId) -> Option<&Column> {
        self.columns.get(column.index())
    }

    pub fn model(&self) -> &dyn TableModel {
        self.model.as_ref()
    }

    pub fn row_count(&self) -> usize {
        self.model.row_count()
    }

    pub fn sort(&self) -> Option<SortSpec> {
        self.sort
    }

    /// Model indices in display order.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// Row ids in display order.
    pub fn display_rows(&self) -> &[RowId] {
        &self.display
    }

    pub fn display_row_at(&self, index: usize) -> Option<RowId> {
        self.display.get(index).copied()
    }

    pub const fn selection(&self) -> &SelectionModel<RowId> {
        &self.selection
    }

    pub const fn active_column(&self) -> ColumnId {
        self.active_column
    }

    pub fn editing(&self) -> Option<&EditSession> {
        self.edit.as_ref()
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn history(&self) -> &EditHistory {
        &self.history
    }

    pub const fn container(&self) -> Rect {
        self.container
    }

    /// Total width of all columns.
    pub fn total_width(&self) -> f64 {
        self.columns.iter().map(|column| column.width).sum()
    }

    /// Left edge of `column`, relative to the grid's own box.
    pub fn column_x(&self, column: ColumnId) -> f64 {
        self.columns
            .iter()
            .take(column.index())
            .map(|column| column.width)
            .sum()
    }

    /// The column containing `x` (relative to the grid's box).
    pub fn column_at_x(&self, x: f64) -> Option<ColumnId> {
        let mut left = 0.0;
        for (index, column) in self.columns.iter().enumerate() {
            if x >= left && x < left + column.width {
                return Some(ColumnId(index));
            }
            left += column.width;
        }
        None
    }

    /// Box of the display row at `index`, or `None` when it is fully clipped.
    pub fn row_bounds_at(&self, index: usize) -> Option<Rect> {
        self.row_bounds_in(self.container, index)
    }

    /// The display row containing `y` (in the grid's coordinate space).
    pub fn row_at(&self, y: f64) -> Option<usize> {
        self.row_at_in(self.container, y)
    }

    /// Tells the grid how big its box is. Returns whether the window moved.
    pub fn set_container(&mut self, rect: Rect) -> bool {
        self.container = rect;
        let extent = (rect.height() - self.header_height).max(0.0) as f32;
        let before = self.viewport.visible_range();
        self.viewport.set_viewport_extent(extent);
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

    /// How many rows the current window materializes.
    pub fn materialized_count(&self) -> usize {
        self.viewport.visible_range().len()
    }

    pub fn visible_row_ids(&self) -> Vec<RowId> {
        let range = self.viewport.visible_range();
        self.display[range].to_vec()
    }

    /// The element a host bound to `row` via [`DataGrid::sync_rows`].
    pub fn bound_element(&self, row: RowId) -> Option<ElementId> {
        self.viewport.get(&row)
    }

    pub fn scroll_offset(&self) -> f64 {
        f64::from(self.viewport.scroll_offset())
    }

    pub fn max_scroll(&self) -> f64 {
        let content = self.model.row_count() as f64 * self.row_height;
        (content - f64::from(self.viewport.viewport_extent())).max(0.0)
    }

    pub fn scroll_to(&mut self, offset: f64) -> bool {
        let offset = offset.clamp(0.0, self.max_scroll());
        if (f64::from(self.viewport.scroll_offset()) - offset).abs() < f64::EPSILON {
            return false;
        }
        self.viewport.scroll_to(offset as f32);
        self.pending_repaint = true;
        true
    }

    pub fn scroll_by(&mut self, delta: f64) -> bool {
        self.scroll_to(self.scroll_offset() + delta)
    }

    /// Materializes element-backed rows for the window, recycling through
    /// `pool`. Returns the number of rows currently bound.
    pub fn sync_rows(&mut self, pool: &mut RowPool) -> usize {
        let Self {
            viewport, display, ..
        } = self;
        let diff = viewport.update(|index| display[index]);
        for (_, element) in diff.exit {
            pool.release(element);
        }
        for (_, key) in diff.enter {
            let element = pool.acquire();
            viewport.assign(key, element);
        }
        viewport.materialized_count()
    }

    /// Writes a cell, re-deriving the display order and the selection.
    pub fn set_cell(&mut self, row: RowId, column: ColumnId, value: CellValue) -> bool {
        if !self.model.set_cell(row, column, value) {
            return false;
        }
        self.recompute();
        self.pending_repaint = true;
        true
    }

    pub fn set_sort(&mut self, sort: Option<SortSpec>) -> bool {
        if self.sort == sort {
            return false;
        }
        self.sort = sort;
        self.recompute();
        self.pending_repaint = true;
        true
    }

    /// Header click: cycles ascending → descending → unsorted for `column`.
    pub fn toggle_sort(&mut self, column: ColumnId) -> bool {
        match self.column(column) {
            Some(column_schema) if column_schema.sortable => {}
            _ => return false,
        }
        self.set_sort(SortSpec::toggled(self.sort, column))
    }

    pub fn select_row(&mut self, index: usize) -> bool {
        self.apply_selection(index, SelectKind::Replace)
    }

    pub fn select_all(&mut self) -> bool {
        let changed = self.selection.select_all(&self.display);
        if changed {
            self.pending_repaint = true;
        }
        changed
    }

    /// Opens an editor on the cursor cell, if that cell is editable.
    pub fn begin_edit(&mut self) -> bool {
        let Some(row) = self.selection.cursor() else {
            return false;
        };
        let Some(column) = self.column(self.active_column) else {
            return false;
        };
        if !column.editable {
            return false;
        }
        let before = self.model.cell(row, self.active_column);
        self.edit = Some(EditSession::begin(row, self.active_column, before));
        self.pending_repaint = true;
        true
    }

    pub fn edit_set_text(&mut self, text: impl Into<String>) -> bool {
        let changed = self.edit.as_mut().is_some_and(|edit| edit.set_text(text));
        if changed {
            self.pending_repaint = true;
        }
        changed
    }

    pub fn edit_insert(&mut self, ch: char) -> bool {
        let changed = self.edit.as_mut().is_some_and(|edit| edit.insert(ch));
        if changed {
            self.pending_repaint = true;
        }
        changed
    }

    pub fn edit_backspace(&mut self) -> bool {
        let changed = self.edit.as_mut().is_some_and(|edit| edit.backspace());
        if changed {
            self.pending_repaint = true;
        }
        changed
    }

    /// Commits the open editor.
    ///
    /// Returns the applied edit, or `None` when there was nothing to apply —
    /// either the text does not parse for the column's kind, in which case the
    /// editor stays open so the user can fix it, or the value is unchanged, in
    /// which case the editor closes silently.
    pub fn commit_edit(&mut self) -> Option<CellEdit> {
        let kind = {
            let session = self.edit.as_ref()?;
            self.column(session.column)?.kind
        };
        // Refuse before consuming the session: a rejected commit has to leave
        // the editor open with the user's text intact.
        if self.edit.as_ref()?.to_value(kind).is_none() {
            return None;
        }
        let session = self.edit.take()?;
        self.pending_repaint = true;
        let edit = session.finish(kind)?;
        // Only a *fresh* commit enters the history; `apply` is also the path
        // undo/redo take, and re-pushing there would make undo lose its place.
        self.history.push(edit.clone());
        self.apply(&edit, edit.after.clone());
        Some(edit)
    }

    pub fn cancel_edit(&mut self) -> bool {
        if self.edit.take().is_none() {
            return false;
        }
        self.pending_repaint = true;
        true
    }

    pub fn undo(&mut self) -> Option<CellEdit> {
        let edit = self.history.undo()?;
        self.apply(&edit, edit.before.clone());
        Some(edit)
    }

    pub fn redo(&mut self) -> Option<CellEdit> {
        let edit = self.history.redo()?;
        self.apply(&edit, edit.after.clone());
        Some(edit)
    }

    fn apply(&mut self, edit: &CellEdit, value: CellValue) {
        self.model.set_cell(edit.row, edit.column, value);
        self.recompute();
        self.pending_repaint = true;
    }

    fn refresh_sizes(&mut self) {
        let count = self.model.row_count();
        self.viewport
            .set_sizes(ItemSizes::uniform(self.row_height as f32, count));
    }

    /// Re-derives display order and drops selection entries that vanished.
    fn recompute(&mut self) {
        self.refresh_sizes();
        self.order = sorted_order(self.model.as_ref(), self.sort);
        self.display = self
            .order
            .iter()
            .map(|index| self.model.row_id(*index))
            .collect();
        if self.selection.clamp_to(&self.display) {
            self.pending_repaint = true;
        }
    }

    /// Applies one selection gesture and invalidates if it moved anything.
    fn apply_selection(&mut self, index: usize, kind: SelectKind) -> bool {
        let changed = match kind {
            SelectKind::Replace => self.selection.move_to(index, &self.display),
            SelectKind::Extend => self.selection.extend_to(index, &self.display),
            SelectKind::Toggle => self.selection.toggle_at(index, &self.display),
        };
        if changed {
            self.pending_repaint = true;
        }
        changed
    }

    /// Whether the active-cell indicator belongs on `row`.
    fn active_cursor_is(&self, row: RowId) -> bool {
        self.focused && self.selection.cursor() == Some(row)
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

    fn row_bounds_in(&self, bounds: Rect, index: usize) -> Option<Rect> {
        let body_top = bounds.y0 + self.header_height;
        let top = body_top + index as f64 * self.row_height - self.scroll_offset();
        let bottom = top + self.row_height;
        if bottom <= body_top || top >= bounds.y1 {
            return None;
        }
        // Clipped to the body so a partly scrolled row cannot paint over the
        // header; the paint context has no clip stack, so the clip is done here.
        Some(Rect {
            x0: bounds.x0,
            y0: top.max(body_top),
            x1: bounds.x1,
            y1: bottom.min(bounds.y1),
        })
    }

    fn row_at_in(&self, bounds: Rect, y: f64) -> Option<usize> {
        let offset = y - (bounds.y0 + self.header_height) + self.scroll_offset();
        if offset < 0.0 || self.row_height <= 0.0 {
            return None;
        }
        let index = (offset / self.row_height).floor() as usize;
        (index < self.model.row_count()).then_some(index)
    }

    /// Rows a page key moves, derived from the box rather than a constant.
    fn page_rows(&self) -> isize {
        if self.row_height <= 0.0 {
            return 0;
        }
        (f64::from(self.viewport.viewport_extent()) / self.row_height).floor() as isize
    }

    fn scroll_cursor_into_view(&mut self) -> bool {
        let Some(index) = self.selection.cursor_index(&self.display) else {
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
        bounds: Rect,
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

        let local_x = pointer.position.x - bounds.x0;
        if pointer.position.y < bounds.y0 + self.header_height {
            // Any header press toggles the sort of the column it landed in.
            return match self.column_at_x(local_x) {
                Some(column) => self.toggle_sort(column),
                None => true,
            };
        }

        let Some(index) = self.row_at_in(bounds, pointer.position.y) else {
            return true;
        };
        let Some(column) = self.column_at_x(local_x) else {
            return true;
        };
        self.active_column = column;

        if pointer.click_count >= 2 {
            // The selection change is itself a handled event even when the
            // cell refuses to be edited (a read-only column), so the two
            // outcomes are OR-ed rather than the editor's answer being returned.
            let selected = self.select_row(index);
            let edited = self.begin_edit();
            return selected || edited;
        }
        if pointer.modifiers.contains(Modifiers::SHIFT) {
            return self.apply_selection(index, SelectKind::Extend);
        }
        if pointer.modifiers.contains(Modifiers::META)
            || pointer.modifiers.contains(Modifiers::CONTROL)
        {
            return self.apply_selection(index, SelectKind::Toggle);
        }
        self.apply_selection(index, SelectKind::Replace)
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

        if self.edit.is_some() {
            return self.editor_key(&key.key);
        }

        let command =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::META);
        let shift = key.modifiers.contains(Modifiers::SHIFT);

        match key.key {
            Key::Char('a') if command => return self.select_all(),
            Key::Char('z') if command && shift => return self.redo().is_some(),
            Key::Char('z') if command => return self.undo().is_some(),
            Key::Char('y') if command => return self.redo().is_some(),
            _ => {}
        }

        let moved = match key.key {
            Key::Named(NamedKey::ArrowDown) => self.selection.navigate(1, &self.display, shift),
            Key::Named(NamedKey::ArrowUp) => self.selection.navigate(-1, &self.display, shift),
            Key::Named(NamedKey::Home) => self.selection.jump(0, &self.display, shift),
            Key::Named(NamedKey::End) => {
                self.selection
                    .jump(self.display.len().saturating_sub(1), &self.display, shift)
            }
            Key::Named(NamedKey::PageDown) => {
                let page = self.page_rows();
                self.selection.navigate(page, &self.display, shift)
            }
            Key::Named(NamedKey::PageUp) => {
                let page = self.page_rows();
                self.selection.navigate(-page, &self.display, shift)
            }
            Key::Named(NamedKey::ArrowRight) => {
                return self.move_active_column(1);
            }
            Key::Named(NamedKey::ArrowLeft) => {
                return self.move_active_column(-1);
            }
            Key::Named(NamedKey::Enter) => return self.begin_edit(),
            Key::Char(' ') => match self.selection.cursor_index(&self.display) {
                Some(index) => self.selection.toggle_at(index, &self.display),
                None => false,
            },
            _ => return false,
        };
        if moved {
            self.pending_repaint = true;
            self.scroll_cursor_into_view();
        }
        moved
    }

    fn move_active_column(&mut self, step: isize) -> bool {
        let next = self.active_column.index() as isize + step;
        if next < 0 || next as usize >= self.columns.len() {
            return false;
        }
        self.active_column = ColumnId(next as usize);
        self.pending_repaint = true;
        true
    }

    /// While an editor is open it owns the keyboard: the grid must not move
    /// its cursor under a caret the user is positioning.
    fn editor_key(&mut self, key: &Key) -> bool {
        match key {
            Key::Named(NamedKey::Escape) => self.cancel_edit(),
            Key::Named(NamedKey::Enter) => {
                self.commit_edit();
                true
            }
            Key::Named(NamedKey::Backspace) => self.edit_backspace(),
            Key::Named(NamedKey::ArrowLeft) => {
                self.edit.as_mut().is_some_and(|edit| edit.move_caret(-1))
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.edit.as_mut().is_some_and(|edit| edit.move_caret(1))
            }
            Key::Char(ch) => self.edit_insert(*ch),
            _ => false,
        }
    }

    fn semantics_hint(&self) -> SemanticsHint {
        let titles: Vec<&str> = self
            .columns
            .iter()
            .map(|column| column.title.as_str())
            .collect();
        let selected = self.selection.selected_count();
        let mut label = format!(
            "{} rows, {} columns ({})",
            self.model.row_count(),
            self.columns.len(),
            titles.join(", ")
        );
        if selected > 0 {
            label.push_str(&format!(", {selected} selected"));
        }
        let mut state = SemanticsState::ENABLED | SemanticsState::FOCUSABLE;
        state |= SemanticsState::SCROLLABLE;
        if self.focused {
            state |= SemanticsState::FOCUSED;
        }
        SemanticsHint {
            role: SemanticsRole::List,
            label: Some(label),
            state,
        }
    }
}

impl Widget for DataGrid {
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
        let body_top = bounds.y0 + self.header_height;

        let header = Rect {
            x0: bounds.x0,
            y0: bounds.y0,
            x1: bounds.x1,
            y1: body_top,
        };
        cx.fill_rect(header, HEADER_BG);
        for (index, column) in self.columns.iter().enumerate() {
            let id = ColumnId(index);
            let title = match self.sort {
                Some(spec) if spec.column == id => {
                    let arrow = if spec.direction.is_ascending() {
                        '▲'
                    } else {
                        '▼'
                    };
                    format!("{} {arrow}", column.title)
                }
                _ => column.title.clone(),
            };
            cx.fill_text(
                Point::new(
                    self.column_x(id) + CELL_PADDING,
                    text_y(bounds.y0, self.header_height),
                ),
                &title,
                LABEL_FONT_SIZE,
                FontWeight::Normal,
                HEADER_FG,
            );
        }

        // Only the window is painted, which is the whole point of the viewport.
        for index in self.viewport.visible_range() {
            let Some(row) = self.display_row_at(index) else {
                continue;
            };
            let Some(row_bounds) = self.row_bounds_in(bounds, index) else {
                continue;
            };
            let background = if self.selection.is_selected(row) {
                SELECTED_BG
            } else if index % 2 == 1 {
                ROW_ALT_BG
            } else {
                ROW_BG
            };
            cx.fill_rect(row_bounds, background);

            if self.active_cursor_is(row) {
                cx.fill_rect(
                    Rect {
                        x0: bounds.x0 + self.column_x(self.active_column),
                        y0: row_bounds.y0,
                        x1: bounds.x0
                            + self.column_x(self.active_column)
                            + self
                                .column(self.active_column)
                                .map_or(0.0, |column| column.width),
                        y1: row_bounds.y0 + 1.0,
                    },
                    ACTIVE_CELL_BORDER,
                );
            }

            for (column_index, _) in self.columns.iter().enumerate() {
                let id = ColumnId(column_index);
                let text = self.model.cell(row, id).display();
                if text.is_empty() {
                    continue;
                }
                cx.fill_text(
                    Point::new(
                        bounds.x0 + self.column_x(id) + CELL_PADDING,
                        text_y(row_bounds.y0, self.row_height),
                    ),
                    &text,
                    LABEL_FONT_SIZE,
                    FontWeight::Normal,
                    ROW_FG,
                );
            }
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

impl WidgetAny for DataGrid {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let hint = DataGrid::semantics_hint(self);
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

    #[test]
    fn a_row_pool_reuses_released_ids() {
        let mut pool = RowPool::new();
        let first = pool.acquire();
        let second = pool.acquire();
        assert_eq!(pool.high_water(), 2);
        pool.release(first);
        assert_eq!(pool.in_use(), 1);
        let third = pool.acquire();
        assert_eq!(
            third, first,
            "an id on the free list is reused before minting"
        );
        assert_eq!(pool.allocated(), 2, "no new id was minted");
        assert_ne!(third, second, "a recycled id is not the live one");
        assert_eq!(pool.in_use(), 2);
    }

    #[test]
    fn text_is_centred_within_its_band() {
        assert!((text_y(24.0, 20.0) - (24.0 + (20.0 - LINE_BOX) / 2.0)).abs() < 1e-9);
    }
}
