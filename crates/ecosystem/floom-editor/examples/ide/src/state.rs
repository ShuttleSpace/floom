//! Everything the demo knows: the workspace, the open files, and the one
//! editor that is showing one of them.
//!
//! The editor widget is deliberately *one* widget for the whole session. A tab
//! is not a widget: it is a document, a selection and a scroll position. That
//! is what lets switching tabs keep each file's history, and it is also the
//! reason the editor's document cache had to learn that a document is not just
//! its revision number.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use floom_core::{ElementId, EventContext};
use floom_editor::core::buffer::rope_text::RopeText;
use floom_editor::core::cursor::CursorAffinity;
use floom_editor::core::selection::Selection;
use floom_editor::{Document, EditorState, TextDocument};
use floom_types::Point;

use crate::lang::{Lang, Syntax};

/// The explorer's width, in logical pixels.
pub const SIDEBAR_WIDTH: f64 = 238.0;
/// The tab strip's height.
pub const TAB_BAR_HEIGHT: f64 = 36.0;
/// The status bar's height.
pub const STATUS_BAR_HEIGHT: f64 = 24.0;
/// The explorer's header, above the first row.
pub const EXPLORER_HEADER: f64 = 34.0;
/// One explorer row's height.
pub const ROW_HEIGHT: f64 = 23.0;
/// How deep the scanner walks. Deeper than this is a directory tree, not a
/// demo workspace.
const MAX_DEPTH: usize = 5;
/// A file larger than this is not opened: the demo is for reading source.
const MAX_FILE_BYTES: u64 = 1 << 20;

/// One file open in a tab.
pub struct OpenFile {
    pub path: PathBuf,
    pub title: String,
    pub lang: Lang,
    /// The editor's document: the rope, its undo history, its revision.
    ///
    /// Each open file owns one, which is why closing and reopening a tab is
    /// the only way to lose an undo stack.
    pub doc: Arc<dyn Document>,
    /// Where the caret was when this file was last on screen.
    pub sel: Selection,
    /// The first source line on screen, and how far above the viewport it is.
    pub scroll_line: usize,
    pub scroll_offset: f64,
    /// The document revision that matches the file on disk.
    pub saved_rev: u64,
}

impl OpenFile {
    /// Whether the buffer differs from what was last read or written.
    pub fn dirty(&self) -> bool {
        self.doc.revision() != self.saved_rev
    }
}

/// One line in the explorer.
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    /// How far to indent the row.
    pub depth: usize,
    pub dir: bool,
    /// Directories only: whether its children are listed below it.
    pub expanded: bool,
    pub lang: Lang,
}

/// The demo's model.
pub struct App {
    pub root: PathBuf,
    pub name: String,
    pub rows: Vec<Row>,
    pub open: Vec<OpenFile>,
    pub active: Option<usize>,
    /// The first explorer row on screen.
    pub scroll: usize,
    /// A one-line note in the status bar: "Saved src/main.rs", an error.
    pub message: Option<String>,
    /// Bumped on every change a chrome widget should repaint for.
    pub generation: u64,
    expanded: HashSet<PathBuf>,
}

impl App {
    /// Scans `root` and opens its first Rust file, or its first file.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
            .to_string();
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        // The demo's workspace is one level deep by design; opening `src` and
        // `docs` on entry is what makes the explorer look like an IDE rather
        // than like a closed folder.
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    expanded.insert(entry.path());
                }
            }
        }
        let mut app = Self {
            root,
            name,
            rows: Vec::new(),
            open: Vec::new(),
            active: None,
            scroll: 0,
            message: None,
            generation: 1,
            expanded,
        };
        app.rescan();

        let first = app
            .rows()
            .iter()
            .find(|row| !row.dir && row.path.ends_with("src/main.rs"))
            .or_else(|| app.rows().iter().find(|row| !row.dir))
            .map(|row| row.path.clone());
        if let Some(path) = first {
            if let Err(message) = app.open_path(&path) {
                app.message = Some(message);
            }
        }
        app
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Rebuilds the visible rows from the expanded set.
    fn rescan(&mut self) {
        let mut rows = Vec::new();
        self.walk(&self.root.clone(), 0, &mut rows);
        self.rows = rows;
    }

    fn walk(&self, dir: &Path, depth: usize, rows: &mut Vec<Row>) {
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| visible(path))
            .collect();
        entries.sort_by(|left, right| {
            let left_dir = left.is_dir();
            let right_dir = right.is_dir();
            right_dir.cmp(&left_dir).then_with(|| {
                name_of(left)
                    .to_lowercase()
                    .cmp(&name_of(right).to_lowercase())
            })
        });

        for path in entries {
            let dir = path.is_dir();
            let expanded = self.expanded.contains(&path);
            rows.push(Row {
                name: name_of(&path),
                lang: Lang::from_path(&path),
                depth,
                dir,
                expanded,
                path: path.clone(),
            });
            if dir && expanded {
                self.walk(&path, depth + 1, rows);
            }
        }
    }

    /// Opens `path` in a new tab, or activates the tab that already has it.
    pub fn open_path(&mut self, path: &Path) -> Result<usize, String> {
        if let Some(index) = self.open.iter().position(|file| file.path == path) {
            self.active = Some(index);
            self.generation += 1;
            return Ok(index);
        }
        let size = std::fs::metadata(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?
            .len();
        if size > MAX_FILE_BYTES {
            return Err(format!("{} is too large to open", name_of(path)));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if text.contains('\0') {
            return Err(format!("{} is not a text file", name_of(path)));
        }

        let doc: Arc<dyn Document> = Arc::new(TextDocument::new(text));
        let file = OpenFile {
            title: name_of(path),
            lang: Lang::from_path(path),
            saved_rev: doc.revision(),
            doc,
            sel: Selection::caret(0, CursorAffinity::Forward),
            scroll_line: 0,
            scroll_offset: 0.0,
            path: path.to_path_buf(),
        };
        self.open.push(file);
        self.active = Some(self.open.len() - 1);
        self.message = None;
        self.generation += 1;
        Ok(self.open.len() - 1)
    }

    /// The row for `path`, if it is visible.
    pub fn row_of(&self, path: &Path) -> Option<usize> {
        self.rows.iter().position(|row| row.path == path)
    }

    /// Toggles a directory row and rescans.
    pub fn toggle(&mut self, row: usize) {
        let Some(entry) = self.rows.get(row) else {
            return;
        };
        if !entry.dir {
            return;
        }
        let path = entry.path.clone();
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
        self.rescan();
        self.generation += 1;
    }

    /// Clamps the explorer's scroll to the rows that fit.
    pub fn clamp_scroll(&mut self, visible: usize) {
        let max = self.rows.len().saturating_sub(visible.max(1));
        if self.scroll > max {
            self.scroll = max;
        }
    }

    /// Removes a tab and picks the neighbour to show instead.
    ///
    /// Returns the file to load into the editor, or `None` when no tabs are
    /// left (the caller then clears the editor).
    pub fn close(&mut self, index: usize) -> Option<Arc<dyn Document>> {
        if index >= self.open.len() {
            return None;
        }
        self.open.remove(index);
        match self.active {
            _ if self.open.is_empty() => {
                self.active = None;
                None
            }
            Some(active) if active == index => {
                let next = index.min(self.open.len() - 1);
                self.active = Some(next);
                Some(self.open[next].doc.clone())
            }
            Some(active) if active > index => {
                self.active = Some(active - 1);
                None
            }
            _ => None,
        }
    }
}

/// `path`'s file name, as a string.
pub fn name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("?")
        .to_string()
}

/// Whether the explorer shows `path`.
fn visible(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !name.starts_with('.') && name != "target" && name != "node_modules"
}

/// The handles every widget shares.
///
/// One lock for the model, one for the editor's state, and one slot for the
/// editor's element id (found after mounting, so the explorer can hand focus
/// back to the text after opening a file).
#[derive(Clone)]
pub struct Shared {
    app: Arc<Mutex<App>>,
    editor: Arc<Mutex<EditorState>>,
    syntax: Arc<Syntax>,
    editor_element: Arc<Mutex<Option<ElementId>>>,
}

impl Shared {
    pub fn new(app: App, editor: Arc<Mutex<EditorState>>, syntax: Arc<Syntax>) -> Self {
        Self {
            app: Arc::new(Mutex::new(app)),
            editor,
            syntax,
            editor_element: Arc::new(Mutex::new(None)),
        }
    }

    pub fn app(&self) -> MutexGuard<'_, App> {
        self.app.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub fn editor(&self) -> MutexGuard<'_, EditorState> {
        self.editor
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub fn set_editor_element(&self, id: ElementId) {
        *self
            .editor_element
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(id);
    }

    /// Hands keyboard focus back to the text after a click in the chrome.
    pub fn focus_editor(&self, cx: &mut dyn EventContext) {
        let id = *self
            .editor_element
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(id) = id {
            cx.request_focus(id);
        }
    }

    /// The model's change counter.
    pub fn generation(&self) -> u64 {
        self.app().generation
    }

    /// The editor's change counter: caret movements, edits and scrolls.
    pub fn editor_generation(&self) -> u64 {
        self.editor().generation
    }

    /// Expands or collapses a directory row.
    pub fn toggle(&self, row: usize) {
        let mut app = self.app();
        app.toggle(row);
    }

    /// Opens `path` (or activates its tab) and shows it.
    pub fn open(&self, path: &Path) {
        let mut app = self.app();
        {
            let state = self.editor();
            if let Some(current) = app.active {
                let file = &mut app.open[current];
                file.doc = state.doc.clone();
                file.sel = state.sel.clone();
                file.scroll_line = state.scroll_line;
                file.scroll_offset = state.scroll_offset;
            }
        }
        match app.open_path(path) {
            Ok(index) => {
                let file = &app.open[index];
                let (doc, sel, line, offset, lang) = (
                    file.doc.clone(),
                    file.sel.clone(),
                    file.scroll_line,
                    file.scroll_offset,
                    file.lang,
                );
                let mut state = self.editor();
                state.doc = doc;
                state.sel = sel;
                state.scroll_line = line;
                state.scroll_offset = offset;
                state.generation += 1;
                self.syntax.set(lang);
            }
            Err(message) => {
                app.message = Some(message);
                app.generation += 1;
            }
        }
    }

    /// Shows the tab at `index`.
    pub fn activate(&self, index: usize) {
        let mut app = self.app();
        if app.active == Some(index) || index >= app.open.len() {
            return;
        }
        {
            let mut state = self.editor();
            if let Some(current) = app.active {
                let file = &mut app.open[current];
                file.doc = state.doc.clone();
                file.sel = state.sel.clone();
                file.scroll_line = state.scroll_line;
                file.scroll_offset = state.scroll_offset;
            }
            let file = &app.open[index];
            state.doc = file.doc.clone();
            state.sel = file.sel.clone();
            state.scroll_line = file.scroll_line;
            state.scroll_offset = file.scroll_offset;
            state.generation += 1;
            self.syntax.set(file.lang);
        }
        app.active = Some(index);
        app.message = None;
        app.generation += 1;
    }

    /// Closes the tab at `index`.
    pub fn close(&self, index: usize) {
        let mut app = self.app();
        let was_active = app.active == Some(index);
        let next = app.close(index);
        if was_active {
            match next {
                Some(doc) => {
                    let lang = app
                        .active
                        .and_then(|active| app.open.get(active))
                        .map(|file| file.lang)
                        .unwrap_or(Lang::Text);
                    let mut state = self.editor();
                    state.doc = doc;
                    state.sel = Selection::caret(0, CursorAffinity::Forward);
                    state.scroll_line = 0;
                    state.scroll_offset = 0.0;
                    state.generation += 1;
                    self.syntax.set(lang);
                }
                None => {
                    let mut state = self.editor();
                    state.doc = Arc::new(TextDocument::new(""));
                    state.sel = Selection::caret(0, CursorAffinity::Forward);
                    state.scroll_line = 0;
                    state.scroll_offset = 0.0;
                    state.generation += 1;
                    self.syntax.set(Lang::Text);
                }
            }
        }
        app.generation += 1;
    }

    /// Writes the active buffer back to its file.
    pub fn save(&self) {
        let mut app = self.app();
        let Some(active) = app.active else {
            return;
        };
        let message = {
            let file = &mut app.open[active];
            let text = file.doc.text().to_string();
            match std::fs::write(&file.path, text) {
                Ok(()) => {
                    file.saved_rev = file.doc.revision();
                    format!("Saved {}", file.title)
                }
                Err(error) => format!("Save failed: {error}"),
            }
        };
        app.message = Some(message);
        app.generation += 1;
    }

    /// Scrolls the explorer by `delta_y` pixels.
    pub fn scroll(&self, delta_y: f64, visible: usize) {
        let mut app = self.app();
        let step = if delta_y.abs() >= ROW_HEIGHT {
            (delta_y / ROW_HEIGHT).round() as i64
        } else if delta_y > 0.0 {
            1
        } else if delta_y < 0.0 {
            -1
        } else {
            0
        };
        let max = app.rows.len().saturating_sub(visible.max(1));
        let next = (app.scroll as i64 + step).clamp(0, max as i64) as usize;
        if next != app.scroll {
            app.scroll = next;
            app.generation += 1;
        }
    }

    /// Where the primary caret is, for the status bar.
    pub fn caret(&self) -> Caret {
        let state = self.editor();
        let text = state.doc.rope_text();
        let cursor = state.cursor();
        let line = text.line_of_offset(cursor);
        let line_start = text.offset_of_line(line);
        let column = text.slice_to_cow(line_start..cursor).chars().count();
        Caret {
            line: line + 1,
            column: column + 1,
            carets: state.caret_count(),
            selected: state.has_selection(),
        }
    }
}

/// The primary caret, one-based.
pub struct Caret {
    pub line: usize,
    pub column: usize,
    pub carets: usize,
    pub selected: bool,
}

/// The centre of explorer row `index` in viewport coordinates.
///
/// The tests click with it; the widget hit-tests with the same arithmetic, so a
/// test that clicks "the fourth row" clicks what the user would.
pub fn row_center(index: usize) -> Point {
    Point::new(60.0, EXPLORER_HEADER + (index as f64 + 0.5) * ROW_HEIGHT)
}
