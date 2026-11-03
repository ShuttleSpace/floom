//! Headless data models for the Phase 20 data widgets.
//!
//! Everything here is pure data: a table is a row count plus a cell lookup, a
//! selection is a cursor plus a set, an edit stack is two vectors. The widgets
//! in [`crate::data_grid`] and [`crate::tree`] add geometry and painting, but
//! the behaviour they expose is decided here — so it can be tested without a
//! renderer, a window, or a clock.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::hash::Hash;

/// Stable identity of a row, independent of its current display position.
///
/// Sorting and filtering reorder rows; identity has to survive that, or a
/// selection would follow the slot instead of the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowId(pub u64);

impl RowId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Anything a selection can be keyed by.
///
/// The grid keys on [`RowId`]; the tree keys on `TreeNodeId`. Both are
/// newtypes over an integer, and neither is interchangeable with the other, so
/// the selection is generic rather than stringly typed.
pub trait RowKey: Copy + Eq + Ord + Hash + Debug {}
impl<T: Copy + Eq + Ord + Hash + Debug> RowKey for T {}

/// Zero-based column index.
///
/// Columns are fixed for the lifetime of a grid, so position *is* identity
/// here — unlike rows, which sort and filter underneath their indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnId(pub usize);

impl ColumnId {
    pub const fn index(self) -> usize {
        self.0
    }
}

/// How a column's values are displayed, parsed and edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColumnKind {
    #[default]
    Text,
    Number,
    Bool,
}

/// One column's metadata. The model owns rows, the grid owns columns.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub title: String,
    pub kind: ColumnKind,
    pub width: f64,
    pub sortable: bool,
    pub editable: bool,
}

impl Column {
    /// A sortable, read-only text column of `width` logical pixels.
    pub fn new(title: impl Into<String>, width: f64) -> Self {
        Self {
            title: title.into(),
            kind: ColumnKind::Text,
            width,
            sortable: true,
            editable: false,
        }
    }

    pub fn kind(mut self, kind: ColumnKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
}

/// One cell's value.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Empty,
    Text(String),
    Number(f64),
    Bool(bool),
}

impl CellValue {
    /// The string a cell paints and reports to assistive technology.
    ///
    /// Whole numbers lose their `.0` — a table full of `42.0` reads as a bug —
    /// and a non-finite number prints as itself rather than as an integer cast
    /// (`i64::MAX` for infinity would be a lie).
    pub fn display(&self) -> String {
        match self {
            CellValue::Empty => String::new(),
            CellValue::Text(text) => text.clone(),
            CellValue::Bool(value) => value.to_string(),
            CellValue::Number(value) => {
                if !value.is_finite() {
                    value.to_string()
                } else if value.fract() == 0.0 && value.abs() < 1e15 {
                    format!("{}", *value as i64)
                } else {
                    format!("{value}")
                }
            }
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            CellValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }

    /// Total order used by column sorting.
    ///
    /// Variants rank `Empty < Bool < Number < Text`, so a heterogeneous column
    /// still sorts deterministically instead of by whatever order the rows
    /// happened to arrive in. `NaN` compares equal to itself: a sort must not
    /// be able to fail on data it did not author.
    pub fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (CellValue::Empty, CellValue::Empty) => Ordering::Equal,
            (CellValue::Empty, _) => Ordering::Less,
            (_, CellValue::Empty) => Ordering::Greater,
            (CellValue::Bool(a), CellValue::Bool(b)) => a.cmp(b),
            (CellValue::Bool(_), _) => Ordering::Less,
            (_, CellValue::Bool(_)) => Ordering::Greater,
            (CellValue::Number(a), CellValue::Number(b)) => {
                a.partial_cmp(b).unwrap_or(Ordering::Equal)
            }
            (CellValue::Number(_), _) => Ordering::Less,
            (_, CellValue::Number(_)) => Ordering::Greater,
            (CellValue::Text(a), CellValue::Text(b)) => a.cmp(b),
        }
    }

    /// Parses user input for a column of `kind`; `None` means "reject".
    ///
    /// Rejecting is the point: a `Number` column that silently turned `abc`
    /// into `Empty` would destroy the value the user was trying to fix.
    pub fn parse(kind: ColumnKind, text: &str) -> Option<Self> {
        match kind {
            ColumnKind::Text => Some(CellValue::Text(text.to_string())),
            ColumnKind::Number => text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(CellValue::Number),
            ColumnKind::Bool => match text.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(CellValue::Bool(true)),
                "false" | "0" | "no" | "off" => Some(CellValue::Bool(false)),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub const fn flipped(self) -> Self {
        match self {
            SortDirection::Ascending => SortDirection::Descending,
            SortDirection::Descending => SortDirection::Ascending,
        }
    }

    pub const fn is_ascending(self) -> bool {
        matches!(self, SortDirection::Ascending)
    }
}

/// Which column the view is sorted by, and which way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SortSpec {
    pub column: ColumnId,
    pub direction: SortDirection,
}

impl SortSpec {
    pub const fn ascending(column: ColumnId) -> Self {
        Self {
            column,
            direction: SortDirection::Ascending,
        }
    }

    pub const fn descending(column: ColumnId) -> Self {
        Self {
            column,
            direction: SortDirection::Descending,
        }
    }

    /// What clicking `column`'s header should do, given the current sort.
    ///
    /// The three-state cycle (ascending → descending → unsorted) is what keeps
    /// the model's own order reachable — a two-state toggle traps the user in a
    /// sorted view with no way back to the order the data arrived in.
    /// Clicking a different column starts over at ascending on that column.
    pub fn toggled(current: Option<SortSpec>, column: ColumnId) -> Option<Self> {
        match current {
            Some(spec) if spec.column == column => match spec.direction {
                SortDirection::Ascending => Some(SortSpec::descending(column)),
                SortDirection::Descending => None,
            },
            _ => Some(SortSpec::ascending(column)),
        }
    }
}

/// Display order of a model's rows under `sort`, as model indices.
///
/// The sort is **stable**: rows with equal keys keep their model order, and
/// `None` returns the identity order. The display order is therefore a pure
/// function of `(model, sort)`, which is what lets the selection, the viewport
/// and the paint pass agree after a re-sort without re-deriving anything.
pub fn sorted_order(model: &dyn TableModel, sort: Option<SortSpec>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..model.row_count()).collect();
    let Some(spec) = sort else {
        return order;
    };
    order.sort_by(|&a, &b| {
        let left = model.cell(model.row_id(a), spec.column);
        let right = model.cell(model.row_id(b), spec.column);
        let ordering = left.compare(&right);
        match spec.direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
    order
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SelectionMode {
    #[default]
    Single,
    Multiple,
}

/// Cursor, anchor and selected set for one view.
///
/// The cursor/anchor pair is stored as *keys*, not indices, so a re-sort moves
/// the selection with the rows rather than leaving it on whatever now occupies
/// the old position.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionModel<K = RowId> {
    mode: SelectionMode,
    cursor: Option<K>,
    anchor: Option<K>,
    selected: BTreeSet<K>,
}

impl<K> Default for SelectionModel<K> {
    fn default() -> Self {
        Self {
            mode: SelectionMode::default(),
            cursor: None,
            anchor: None,
            selected: BTreeSet::new(),
        }
    }
}

impl<K: RowKey> SelectionModel<K> {
    pub fn new(mode: SelectionMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Narrows the model. Going multiple → single collapses the selection onto
    /// the cursor, because a single-selection view reporting four selected rows
    /// is a state the view cannot represent.
    pub fn set_mode(&mut self, mode: SelectionMode) -> bool {
        if self.mode == mode {
            return false;
        }
        self.mode = mode;
        if mode == SelectionMode::Single && self.selected.len() > 1 {
            let keep = self.cursor.or_else(|| self.selected.iter().next().copied());
            self.selected.clear();
            if let Some(key) = keep {
                self.selected.insert(key);
                self.cursor = Some(key);
                self.anchor = Some(key);
            }
        }
        true
    }

    pub const fn cursor(&self) -> Option<K> {
        self.cursor
    }

    /// Position of the cursor in `rows`, if it is still present.
    pub fn cursor_index(&self, rows: &[K]) -> Option<usize> {
        let cursor = self.cursor?;
        rows.iter().position(|row| *row == cursor)
    }

    pub const fn anchor(&self) -> Option<K> {
        self.anchor
    }

    pub fn selected_ids(&self) -> Vec<K> {
        self.selected.iter().copied().collect()
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn is_selected(&self, key: K) -> bool {
        self.selected.contains(&key)
    }

    pub fn clear(&mut self) -> bool {
        if self.cursor.is_none() && self.anchor.is_none() && self.selected.is_empty() {
            return false;
        }
        self.cursor = None;
        self.anchor = None;
        self.selected.clear();
        true
    }

    /// Click-style selection: moves the cursor and makes it the whole selection.
    pub fn move_to(&mut self, index: usize, rows: &[K]) -> bool {
        self.apply_cursor(index, rows, false)
    }

    /// Shift-click: extends the range from the anchor.
    ///
    /// In single-selection mode this degrades to [`SelectionModel::move_to`] —
    /// a range is a state the mode cannot represent, and silently keeping the
    /// old selection would be worse than following the click.
    pub fn extend_to(&mut self, index: usize, rows: &[K]) -> bool {
        if self.mode == SelectionMode::Single {
            return self.move_to(index, rows);
        }
        self.apply_cursor(index, rows, true)
    }

    /// Modifier-click: adds the row, or removes it if it was already selected.
    pub fn toggle_at(&mut self, index: usize, rows: &[K]) -> bool {
        if rows.is_empty() {
            return false;
        }
        if self.mode == SelectionMode::Single {
            return self.move_to(index, rows);
        }
        let key = rows[index.min(rows.len() - 1)];
        if !self.selected.remove(&key) {
            self.selected.insert(key);
        }
        self.cursor = Some(key);
        self.anchor = Some(key);
        true
    }

    /// Keyboard move. With no cursor, a forward step lands on the first row and
    /// a backward step on the last, which is what arrow keys are expected to do
    /// in a list that has never been focused.
    pub fn navigate(&mut self, step: isize, rows: &[K], extend: bool) -> bool {
        if rows.is_empty() {
            return false;
        }
        let target = match self.cursor_index(rows) {
            Some(position) => (position as isize + step).clamp(0, rows.len() as isize - 1) as usize,
            None if step < 0 => rows.len() - 1,
            None => 0,
        };
        self.apply_cursor(target, rows, extend)
    }

    /// Home/End/PageUp/PageDown: jump to an absolute position.
    pub fn jump(&mut self, index: usize, rows: &[K], extend: bool) -> bool {
        if rows.is_empty() {
            return false;
        }
        self.apply_cursor(index.min(rows.len() - 1), rows, extend)
    }

    pub fn select_all(&mut self, rows: &[K]) -> bool {
        if self.mode != SelectionMode::Multiple || rows.is_empty() {
            return false;
        }
        let all: BTreeSet<K> = rows.iter().copied().collect();
        if all == self.selected {
            return false;
        }
        self.selected = all;
        let first = rows[0];
        self.cursor.get_or_insert(first);
        self.anchor.get_or_insert(first);
        true
    }

    /// Drops keys that are no longer present and moves a vanished cursor onto a
    /// survivor. Without this, a filter that removes selected rows would leave
    /// `selected_count` reporting rows the view cannot draw.
    pub fn clamp_to(&mut self, rows: &[K]) -> bool {
        if rows.is_empty() {
            if self.cursor.is_none() && self.anchor.is_none() && self.selected.is_empty() {
                return false;
            }
            self.cursor = None;
            self.anchor = None;
            self.selected.clear();
            return true;
        }
        let before = self.selected.len();
        self.selected.retain(|key| rows.contains(key));
        let mut changed = self.selected.len() != before;

        let first = rows[0];
        if self.cursor.is_some_and(|cursor| !rows.contains(&cursor)) {
            self.cursor = Some(first);
            changed = true;
        }
        if self.anchor.is_some_and(|anchor| !rows.contains(&anchor)) {
            self.anchor = Some(first);
            changed = true;
        }
        changed
    }

    /// Moves the cursor to `index` and rewrites the selection accordingly.
    ///
    /// The anchor is remembered across a drag of shift-arrow presses, so
    /// extending back over the starting row shrinks the range instead of
    /// growing it forever.
    fn apply_cursor(&mut self, index: usize, rows: &[K], extend: bool) -> bool {
        if rows.is_empty() {
            return false;
        }
        let key = rows[index.min(rows.len() - 1)];
        let previous = self.cursor;
        let mut changed = previous != Some(key);
        self.cursor = Some(key);

        if extend && self.mode == SelectionMode::Multiple {
            let anchor = self
                .anchor
                .filter(|anchor| rows.contains(anchor))
                .or(previous)
                .unwrap_or(key);
            self.anchor = Some(anchor);
            let anchor_at = rows.iter().position(|row| *row == anchor).unwrap_or(index);
            let (low, high) = if anchor_at <= index {
                (anchor_at, index)
            } else {
                (index, anchor_at)
            };
            let range: BTreeSet<K> = rows[low..=high].iter().copied().collect();
            if range != self.selected {
                self.selected = range;
                changed = true;
            }
        } else {
            self.anchor = Some(key);
            if self.selected.len() != 1 || !self.selected.contains(&key) {
                self.selected.clear();
                self.selected.insert(key);
                changed = true;
            }
        }
        changed
    }
}

/// Row/cell storage behind a view.
///
/// Deliberately tiny: a model answers "how many rows" and "what is in this
/// cell". Presentation, ordering and selection are the view's business, which
/// is what lets the same model drive a grid, a tree and a chart.
pub trait TableModel {
    fn row_count(&self) -> usize;

    /// Identity of the row at `index`, in model order.
    fn row_id(&self, index: usize) -> RowId;

    fn cell(&self, row: RowId, column: ColumnId) -> CellValue;

    /// Replaces a cell. The default refuses: a model is read-only until it says
    /// otherwise, so an editing UI cannot write into a query result by mistake.
    fn set_cell(&mut self, row: RowId, column: ColumnId, value: CellValue) -> bool {
        let _ = (row, column, value);
        false
    }

    /// Every row id, in model order.
    fn row_ids(&self) -> Vec<RowId> {
        (0..self.row_count())
            .map(|index| self.row_id(index))
            .collect()
    }
}

/// A model backed by a `Vec<Vec<CellValue>>` — the fixture every test and
/// example reaches for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VecTable {
    ids: Vec<RowId>,
    rows: Vec<Vec<CellValue>>,
}

impl VecTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rows get sequential ids starting at zero.
    pub fn from_rows(rows: Vec<Vec<CellValue>>) -> Self {
        let ids = (0..rows.len() as u64).map(RowId).collect();
        Self { ids, rows }
    }

    pub fn push(&mut self, row: Vec<CellValue>) {
        self.ids.push(RowId(self.rows.len() as u64));
        self.rows.push(row);
    }

    pub fn get(&self, row: RowId, column: ColumnId) -> Option<&CellValue> {
        let index = self.ids.iter().position(|id| *id == row)?;
        self.rows.get(index)?.get(column.index())
    }

    fn index_of(&self, row: RowId) -> Option<usize> {
        self.ids.iter().position(|id| *id == row)
    }
}

impl TableModel for VecTable {
    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn row_id(&self, index: usize) -> RowId {
        self.ids[index]
    }

    fn cell(&self, row: RowId, column: ColumnId) -> CellValue {
        self.get(row, column).cloned().unwrap_or(CellValue::Empty)
    }

    fn set_cell(&mut self, row: RowId, column: ColumnId, value: CellValue) -> bool {
        let Some(index) = self.index_of(row) else {
            return false;
        };
        let Some(cell) = self.rows[index].get_mut(column.index()) else {
            return false;
        };
        *cell = value;
        true
    }
}

/// A completed cell edit, in the form an undo stack needs.
#[derive(Debug, Clone, PartialEq)]
pub struct CellEdit {
    pub row: RowId,
    pub column: ColumnId,
    pub before: CellValue,
    pub after: CellValue,
}

/// An in-flight inline edit.
///
/// The session holds *text*, not a value: the column's kind decides whether
/// that text is acceptable, and the decision is deferred until commit so a
/// half-typed number does not have to be parseable the whole time.
#[derive(Debug, Clone, PartialEq)]
pub struct EditSession {
    pub row: RowId,
    pub column: ColumnId,
    pub text: String,
    /// Caret position in characters, not bytes.
    pub caret: usize,
    before: CellValue,
}

impl EditSession {
    pub fn begin(row: RowId, column: ColumnId, before: CellValue) -> Self {
        let text = before.display();
        let caret = text.chars().count();
        Self {
            row,
            column,
            text,
            caret,
            before,
        }
    }

    pub fn before(&self) -> &CellValue {
        &self.before
    }

    pub fn set_text(&mut self, text: impl Into<String>) -> bool {
        let text = text.into();
        if text == self.text {
            return false;
        }
        self.text = text;
        self.caret = self.text.chars().count();
        true
    }

    pub fn insert(&mut self, ch: char) -> bool {
        let byte = self
            .text
            .char_indices()
            .nth(self.caret)
            .map(|(byte, _)| byte)
            .unwrap_or(self.text.len());
        self.text.insert(byte, ch);
        self.caret += 1;
        true
    }

    pub fn backspace(&mut self) -> bool {
        if self.caret == 0 {
            return false;
        }
        self.caret -= 1;
        let byte = self
            .text
            .char_indices()
            .nth(self.caret)
            .map(|(byte, _)| byte)
            .expect("caret is always on a character boundary");
        self.text.remove(byte);
        true
    }

    /// Moves the caret by `delta` characters, refusing to leave the text.
    ///
    /// Caret movement lives on the session rather than in the widget so the
    /// bounds rule has one home: every caller means the same thing by "move
    /// left", and none of them can walk the caret off the text.
    pub fn move_caret(&mut self, delta: isize) -> bool {
        let len = self.text.chars().count() as isize;
        let next = self.caret as isize + delta;
        if next < 0 || next > len || next == self.caret as isize {
            return false;
        }
        self.caret = next as usize;
        true
    }

    /// The value this text would commit as, or `None` if it does not parse.
    pub fn to_value(&self, kind: ColumnKind) -> Option<CellValue> {
        CellValue::parse(kind, &self.text)
    }

    /// Ends the session. `None` covers both "does not parse" and "nothing
    /// changed" — neither is an edit worth recording.
    pub fn finish(&self, kind: ColumnKind) -> Option<CellEdit> {
        let after = CellValue::parse(kind, &self.text)?;
        if after == self.before {
            return None;
        }
        Some(CellEdit {
            row: self.row,
            column: self.column,
            before: self.before.clone(),
            after,
        })
    }
}

/// Undo/redo stacks over [`CellEdit`]s.
///
/// The history records what happened; applying it is the view's job, because
/// only the view holds the model. That split is why `push` refuses a no-op
/// edit: a stacked edit that changes nothing would consume an undo press and
/// appear to do nothing.
#[derive(Debug, Clone, Default)]
pub struct EditHistory {
    undo: Vec<CellEdit>,
    redo: Vec<CellEdit>,
}

impl EditHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, edit: CellEdit) -> bool {
        if edit.before == edit.after {
            return false;
        }
        self.undo.push(edit);
        self.redo.clear();
        true
    }

    /// Pops the newest edit and moves it onto the redo stack.
    ///
    /// The caller applies `before`. Locking in the redo entry up front is safe
    /// because only edits the model already accepted ever reach the stack.
    pub fn undo(&mut self) -> Option<CellEdit> {
        let edit = self.undo.pop()?;
        self.redo.push(edit.clone());
        Some(edit)
    }

    /// Pops the newest undone edit and moves it back onto the undo stack.
    pub fn redo(&mut self) -> Option<CellEdit> {
        let edit = self.redo.pop()?;
        self.undo.push(edit.clone());
        Some(edit)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_direction_flips() {
        assert_eq!(
            SortDirection::Ascending.flipped(),
            SortDirection::Descending
        );
        assert_eq!(
            SortDirection::Descending.flipped(),
            SortDirection::Ascending
        );
        assert!(SortDirection::Ascending.is_ascending());
        assert!(!SortDirection::Descending.is_ascending());
    }

    #[test]
    fn row_ids_are_ordered_by_their_raw_value() {
        let mut ids = vec![RowId(9), RowId(1), RowId(5)];
        ids.sort();
        assert_eq!(ids, vec![RowId(1), RowId(5), RowId(9)]);
        assert_eq!(RowId(7).raw(), 7);
    }

    #[test]
    fn a_cursor_walks_a_selection_model_by_key_not_index() {
        // After a re-sort the cursor's *index* changes; its key does not.
        let rows = vec![RowId(4), RowId(1), RowId(3)];
        let mut selection = SelectionModel::new(SelectionMode::Single);
        selection.move_to(1, &rows);
        assert_eq!(selection.cursor(), Some(RowId(1)));
        assert_eq!(selection.cursor_index(&rows), Some(1));

        let resorted = vec![RowId(1), RowId(3), RowId(4)];
        assert_eq!(selection.cursor_index(&resorted), Some(0));
        assert!(selection.is_selected(RowId(1)));
    }

    #[test]
    fn edit_sessions_handle_multibyte_text_on_character_boundaries() {
        // The caret is a character index; splicing by byte would panic here.
        let mut session = EditSession::begin(RowId(0), ColumnId(0), CellValue::Text("日本".into()));
        assert_eq!(session.caret, 2);
        assert!(session.insert('語'));
        assert_eq!(session.text, "日本語");
        assert!(session.backspace());
        assert_eq!(session.text, "日本");
    }
}
