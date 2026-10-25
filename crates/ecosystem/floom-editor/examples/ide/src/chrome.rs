//! The IDE's chrome: an explorer, a tab strip and a status bar.
//!
//! Every one of these is a leaf [`Widget`] that paints its own box and routes
//! its own clicks. They are not composites because their content is not a
//! tree: an explorer row is a label and a dot, a tab is a label and a close
//! affordance, and the status bar is a line of text that reads the editor's
//! caret. Building those out of widgets would add nodes and alignment problems
//! without adding anything a user could see.
//!
//! What they *do* share is the model ([`Shared`]): the panel reads it while
//! painting, so a change made through the editor — a caret that moved, a
//! buffer that became dirty — shows up in the status bar in the same frame.
//!
//! ## Why nothing here asks for a clip
//!
//! A widget-level clip is the obvious way to keep an over-long label inside
//! its panel, and on the GPU presenter it does not work: a `ClipShape::Box`
//! anywhere in the tree paints the whole window black. So the panels do not
//! clip — they *fit*: every label is measured and cut to the room it has
//! ([`fit`]), and the panels draw nothing that could spill. That is the same
//! guarantee, kept where the text is, and it is why the explorer truncates
//! rather than scrolls.

use floom_core::{
    Axis, Child, ChildKey, CreateContext, ElementId, EventContext, LayoutRequest, PaintContext,
    UpdateContext, Widget, WidgetAny,
};
use floom_editor::TextEditor;
use floom_types::{Color, Event, FontWeight, Key, KeyPhase, Modifiers, Path, Point, Rect, Stroke};

use crate::state::{EXPLORER_HEADER, ROW_HEIGHT, STATUS_BAR_HEIGHT, Shared, TAB_BAR_HEIGHT};
use crate::theme;

/// `text` drawn at `(x, y)` as its top-left corner.
fn label(
    cx: &mut dyn PaintContext,
    x: f64,
    y: f64,
    text: &str,
    size: f32,
    weight: FontWeight,
    color: Color,
) {
    cx.fill_text(Point::new(x, y), text, size, weight, color);
}

/// How wide `text` is at `size`.
fn text_width(text: &str, size: f32) -> f64 {
    floom_text::measure_text(text, size, f64::INFINITY).width
}

/// `text`, cut to `max_width` with an ellipsis.
fn fit(text: &str, size: f32, max_width: f64) -> String {
    if text_width(text, size) <= max_width {
        return text.to_string();
    }
    let mut out = String::new();
    for c in text.chars() {
        let mut candidate = out.clone();
        candidate.push(c);
        candidate.push('…');
        if text_width(&candidate, size) > max_width {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

/// The vertical offset that centres one line of `size` in `height`.
fn baseline(size: f32, height: f64) -> f64 {
    ((height - f64::from(size) * 1.35) / 2.0).max(0.0)
}

/// One status-bar item, and the x the next one starts at.
///
/// Everything after the first is preceded by a rule, so the bar's items are
/// separated the same way whether they sit on the left or the right.
fn item(
    cx: &mut dyn PaintContext,
    x: f64,
    bounds: Rect,
    y: f64,
    text: &str,
    color: Color,
    separated: bool,
) -> f64 {
    let mut x = x;
    if separated {
        x += 7.0;
        separator(cx, bounds, x);
        x += 7.0;
    }
    label(cx, x, y, text, theme::SMALL, FontWeight::Normal, color);
    x + text_width(text, theme::SMALL) + 13.0
}

// ---------------------------------------------------------------------------
// Explorer
// ---------------------------------------------------------------------------

/// The file tree.
#[derive(Clone)]
pub struct Explorer {
    shared: Shared,
    hover: Option<usize>,
    painted_app: u64,
    painted_hover: Option<usize>,
}

impl Explorer {
    pub fn new(shared: Shared) -> Self {
        Self {
            shared,
            hover: None,
            painted_app: 0,
            painted_hover: None,
        }
    }

    fn visible_rows(bounds: Rect) -> usize {
        ((bounds.height() - EXPLORER_HEADER) / ROW_HEIGHT)
            .floor()
            .max(0.0) as usize
    }

    fn row_at(&self, point: Point, bounds: Rect) -> Option<usize> {
        if point.y < bounds.y0 + EXPLORER_HEADER || point.y >= bounds.y1 {
            return None;
        }
        let local = ((point.y - bounds.y0 - EXPLORER_HEADER) / ROW_HEIGHT).floor() as usize;
        let row = self.shared.app().scroll + local;
        (row < self.shared.app().rows().len()).then_some(row)
    }
}

impl Widget for Explorer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let app = self.shared.generation();
        if app != self.painted_app || self.hover != self.painted_hover {
            self.painted_app = app;
            self.painted_hover = self.hover;
            cx.mark_dirty(id);
        }
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let bounds = cx.bounds();
        match cx.event() {
            Event::Pointer(pointer) => match pointer.phase {
                floom_types::PointerPhase::Move => {
                    let hover = self.row_at(pointer.position, bounds);
                    if hover != self.hover {
                        self.hover = hover;
                    }
                    true
                }
                floom_types::PointerPhase::Down
                    if pointer.button == Some(floom_types::PointerButton::Primary) =>
                {
                    let Some(row) = self.row_at(pointer.position, bounds) else {
                        return false;
                    };
                    let (dir, path) = {
                        let app = self.shared.app();
                        let Some(entry) = app.rows().get(row) else {
                            return false;
                        };
                        (entry.dir, entry.path.clone())
                    };
                    if dir {
                        self.shared.toggle(row);
                    } else {
                        self.shared.open(&path);
                        self.shared.focus_editor(cx);
                    }
                    true
                }
                _ => false,
            },
            Event::Scroll(scroll) => {
                self.shared
                    .scroll(scroll.delta_y, Self::visible_rows(bounds));
                true
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, theme::CHROME_BG);
        cx.fill_rect(
            Rect {
                x0: bounds.x1 - 1.0,
                y0: bounds.y0,
                x1: bounds.x1,
                y1: bounds.y1,
            },
            theme::BORDER,
        );

        let app = self.shared.app();
        let active_path = app
            .active
            .and_then(|active| app.open.get(active))
            .map(|file| file.path.clone());

        // Header.
        label(
            cx,
            bounds.x0 + 13.0,
            bounds.y0 + 11.0,
            "EXPLORER",
            theme::SMALL,
            FontWeight::Medium,
            theme::DIM,
        );
        let root_width = text_width(&app.name, theme::SMALL);
        label(
            cx,
            (bounds.x1 - 13.0 - root_width).max(bounds.x0 + 80.0),
            bounds.y0 + 11.0,
            &fit(&app.name, theme::SMALL, 110.0),
            theme::SMALL,
            FontWeight::Normal,
            theme::DIMMER,
        );
        cx.fill_rect(
            Rect {
                x0: bounds.x0,
                y0: bounds.y0 + EXPLORER_HEADER - 1.0,
                x1: bounds.x1,
                y1: bounds.y0 + EXPLORER_HEADER,
            },
            theme::BORDER,
        );

        let visible = Self::visible_rows(bounds);
        let scroll = app.scroll;
        for (offset, entry) in app.rows().iter().skip(scroll).take(visible).enumerate() {
            let index = scroll + offset;
            let y = bounds.y0 + EXPLORER_HEADER + offset as f64 * ROW_HEIGHT;
            let is_active = !entry.dir && active_path.as_deref() == Some(entry.path.as_path());
            if is_active {
                cx.fill_rect(
                    Rect {
                        x0: bounds.x0,
                        y0: y,
                        x1: bounds.x1,
                        y1: y + ROW_HEIGHT,
                    },
                    theme::CHROME_ACTIVE,
                );
                cx.fill_rect(
                    Rect {
                        x0: bounds.x0,
                        y0: y,
                        x1: bounds.x0 + 2.0,
                        y1: y + ROW_HEIGHT,
                    },
                    theme::ACCENT,
                );
            } else if self.hover == Some(index) {
                cx.fill_rect(
                    Rect {
                        x0: bounds.x0,
                        y0: y,
                        x1: bounds.x1,
                        y1: y + ROW_HEIGHT,
                    },
                    theme::CHROME_HOVER,
                );
            }

            let indent = bounds.x0 + 12.0 + entry.depth as f64 * 13.0;
            if entry.dir {
                chevron(cx, indent + 1.0, y + 8.0, entry.expanded, theme::DIM);
            } else {
                cx.fill_rounded_rect(
                    Rect {
                        x0: indent + 3.0,
                        y0: y + 8.0,
                        x1: indent + 10.0,
                        y1: y + 15.0,
                    },
                    2.0,
                    entry.lang.color(),
                );
            }
            let text_x = indent + 17.0;
            let room = (bounds.x1 - text_x - 8.0).max(20.0);
            label(
                cx,
                text_x,
                y + baseline(theme::UI_SIZE, ROW_HEIGHT) - 1.0,
                &fit(&entry.name, theme::UI_SIZE, room),
                theme::UI_SIZE,
                FontWeight::Normal,
                if is_active {
                    theme::BRIGHT
                } else {
                    theme::TEXT
                },
            );
        }
    }
}

impl WidgetAny for Explorer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// A disclosure triangle, `x` and `y` at its left edge.
fn chevron(cx: &mut dyn PaintContext, x: f64, y: f64, open: bool, color: Color) {
    let mut path = Path::new();
    if open {
        path.move_to(Point::new(x, y))
            .line_to(Point::new(x + 7.0, y))
            .line_to(Point::new(x + 3.5, y + 4.5));
    } else {
        path.move_to(Point::new(x + 1.0, y - 2.0))
            .line_to(Point::new(x + 5.5, y + 1.5))
            .line_to(Point::new(x + 1.0, y + 5.0));
    }
    path.close();
    cx.fill_path(&path, color);
}

// ---------------------------------------------------------------------------
// Tab strip
// ---------------------------------------------------------------------------

/// How wide a tab for `title` is.
fn tab_width(title: &str) -> f64 {
    (14.0 + text_width(title, theme::UI_SIZE) + 32.0).clamp(104.0, 216.0)
}

/// The horizontal extent of each tab, relative to the strip's left edge.
fn tab_spans(titles: &[String]) -> Vec<(f64, f64)> {
    let mut x = 0.0;
    titles
        .iter()
        .map(|title| {
            let width = tab_width(title);
            let span = (x, x + width);
            x += width;
            span
        })
        .collect()
}

/// The open files, as tabs.
#[derive(Clone)]
pub struct TabBar {
    shared: Shared,
    hover: Option<usize>,
    painted_app: u64,
    painted_editor: u64,
    painted_hover: Option<usize>,
}

impl TabBar {
    pub fn new(shared: Shared) -> Self {
        Self {
            shared,
            hover: None,
            painted_app: 0,
            painted_editor: 0,
            painted_hover: None,
        }
    }
}

impl Widget for TabBar {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let app = self.shared.generation();
        let editor = self.shared.editor_generation();
        if app != self.painted_app
            || editor != self.painted_editor
            || self.hover != self.painted_hover
        {
            self.painted_app = app;
            self.painted_editor = editor;
            self.painted_hover = self.hover;
            cx.mark_dirty(id);
        }
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let bounds = cx.bounds();
        match cx.event() {
            Event::Pointer(pointer) => match pointer.phase {
                floom_types::PointerPhase::Move => {
                    let titles: Vec<String> = self
                        .shared
                        .app()
                        .open
                        .iter()
                        .map(|file| file.title.clone())
                        .collect();
                    let x = pointer.position.x - bounds.x0;
                    let hover = tab_spans(&titles)
                        .iter()
                        .position(|(x0, x1)| x >= *x0 && x < *x1);
                    self.hover = hover;
                    true
                }
                floom_types::PointerPhase::Down
                    if pointer.button == Some(floom_types::PointerButton::Primary) =>
                {
                    let titles: Vec<String> = self
                        .shared
                        .app()
                        .open
                        .iter()
                        .map(|file| file.title.clone())
                        .collect();
                    let x = pointer.position.x - bounds.x0;
                    let Some((index, (_x0, x1))) = tab_spans(&titles)
                        .into_iter()
                        .enumerate()
                        .find(|(_, (x0, x1))| x >= *x0 && x < *x1)
                    else {
                        return false;
                    };
                    if x >= x1 - 24.0 {
                        self.shared.close(index);
                        self.shared.focus_editor(cx);
                    } else {
                        self.shared.activate(index);
                        self.shared.focus_editor(cx);
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, theme::CHROME_BG);
        cx.fill_rect(
            Rect {
                x0: bounds.x0,
                y0: bounds.y1 - 1.0,
                x1: bounds.x1,
                y1: bounds.y1,
            },
            theme::BORDER,
        );

        let app = self.shared.app();
        let titles: Vec<String> = app.open.iter().map(|file| file.title.clone()).collect();
        if titles.is_empty() {
            label(
                cx,
                bounds.x0 + 16.0,
                bounds.y0 + baseline(theme::UI_SIZE, TAB_BAR_HEIGHT) + 1.0,
                "No file open — pick one in the explorer",
                theme::UI_SIZE,
                FontWeight::Normal,
                theme::DIM,
            );
            return;
        }

        let active = app.active;
        for (index, ((x0, x1), file)) in tab_spans(&titles).into_iter().zip(&app.open).enumerate() {
            let is_active = active == Some(index);
            let left = bounds.x0 + x0;
            let right = bounds.x0 + x1;
            if is_active {
                cx.fill_rect(
                    Rect {
                        x0: left,
                        y0: bounds.y0,
                        x1: right,
                        y1: bounds.y1,
                    },
                    theme::EDITOR_BG,
                );
                cx.fill_rect(
                    Rect {
                        x0: left,
                        y0: bounds.y0,
                        x1: right,
                        y1: bounds.y0 + 2.0,
                    },
                    theme::ACCENT,
                );
            } else if self.hover == Some(index) {
                cx.fill_rect(
                    Rect {
                        x0: left,
                        y0: bounds.y0,
                        x1: right,
                        y1: bounds.y1,
                    },
                    theme::CHROME_HOVER,
                );
            }
            if index > 0 {
                cx.fill_rect(
                    Rect {
                        x0: left,
                        y0: bounds.y0 + 8.0,
                        x1: left + 1.0,
                        y1: bounds.y1 - 8.0,
                    },
                    theme::BORDER,
                );
            }

            let mut text_x = left + 12.0;
            if file.dirty() {
                cx.fill_rounded_rect(
                    Rect {
                        x0: text_x,
                        y0: bounds.y0 + 16.0,
                        x1: text_x + 6.0,
                        y1: bounds.y0 + 22.0,
                    },
                    3.0,
                    theme::TEXT,
                );
                text_x += 12.0;
            }
            let room = (right - 26.0 - text_x).max(24.0);
            label(
                cx,
                text_x,
                bounds.y0 + baseline(theme::UI_SIZE, TAB_BAR_HEIGHT),
                &fit(&file.title, theme::UI_SIZE, room),
                theme::UI_SIZE,
                FontWeight::Normal,
                if is_active { theme::BRIGHT } else { theme::DIM },
            );

            // The close affordance: two short strokes, so it does not depend on
            // the platform's font carrying a multiplication sign.
            let close_x = right - 18.0;
            let close_y = bounds.y0 + TAB_BAR_HEIGHT / 2.0;
            let close_color = if is_active || self.hover == Some(index) {
                theme::TEXT
            } else {
                theme::DIMMER
            };
            let mut cross = Path::new();
            cross
                .move_to(Point::new(close_x - 3.0, close_y - 3.0))
                .line_to(Point::new(close_x + 3.0, close_y + 3.0));
            let mut cross2 = Path::new();
            cross2
                .move_to(Point::new(close_x + 3.0, close_y - 3.0))
                .line_to(Point::new(close_x - 3.0, close_y + 3.0));
            cx.stroke_path(&cross, &Stroke::new(1.3, close_color));
            cx.stroke_path(&cross2, &Stroke::new(1.3, close_color));
        }
    }
}

impl WidgetAny for TabBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

/// The line under the text: where the caret is, what the file is, what is
/// unsaved.
#[derive(Clone)]
pub struct StatusBar {
    shared: Shared,
    painted_app: u64,
    painted_editor: u64,
}

impl StatusBar {
    pub fn new(shared: Shared) -> Self {
        Self {
            shared,
            painted_app: 0,
            painted_editor: 0,
        }
    }
}

impl Widget for StatusBar {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let app = self.shared.generation();
        let editor = self.shared.editor_generation();
        if app != self.painted_app || editor != self.painted_editor {
            self.painted_app = app;
            self.painted_editor = editor;
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, theme::CHROME_BG);
        cx.fill_rect(
            Rect {
                x0: bounds.x0,
                y0: bounds.y0,
                x1: bounds.x1,
                y1: bounds.y0 + 1.0,
            },
            theme::BORDER,
        );

        let caret = self.shared.caret();
        let app = self.shared.app();
        let active = app.active.and_then(|active| app.open.get(active));
        let lang = active
            .map(|file| file.lang)
            .unwrap_or(crate::lang::Lang::Text);
        let dirty = active.map(|file| file.dirty()).unwrap_or(false);

        let y = bounds.y0 + baseline(theme::SMALL, STATUS_BAR_HEIGHT) + 1.0;
        let mut x = bounds.x0 + 12.0;
        let mut after_first = false;
        // The left half, one item at a time, each after the first behind the
        // same rule the right half uses — so a status bar reads as one bar
        // rather than as two groups that happen to share a background.
        match active {
            Some(file) => {
                let path = file
                    .path
                    .strip_prefix(&app.root)
                    .unwrap_or(&file.path)
                    .display()
                    .to_string();
                x = item(cx, x, bounds, y, &path, theme::TEXT, after_first);
            }
            None => x = item(cx, x, bounds, y, "no file", theme::DIMMER, after_first),
        }
        after_first = true;
        x = item(
            cx,
            x,
            bounds,
            y,
            &format!("Ln {}, Col {}", caret.line, caret.column),
            theme::DIM,
            after_first,
        );
        if caret.selected {
            x = item(cx, x, bounds, y, "selection", theme::DIM, after_first);
        }
        if caret.carets > 1 {
            x = item(
                cx,
                x,
                bounds,
                y,
                &format!("{} carets", caret.carets),
                theme::ACCENT,
                after_first,
            );
        }
        if dirty {
            x = item(cx, x, bounds, y, "● unsaved", theme::NUMBER, after_first);
        }
        if let Some(message) = &app.message {
            let color = if message.starts_with("Save failed") || message.starts_with("cannot") {
                theme::NUMBER
            } else {
                theme::ACCENT
            };
            x = item(cx, x, bounds, y, message, color, after_first);
        }

        // The right-hand half, laid out from the right edge.
        let items: [(String, Color); 5] = [
            (lang.label().to_string(), lang.color()),
            ("UTF-8".to_string(), theme::DIM),
            ("LF".to_string(), theme::DIM),
            ("Spaces: 4".to_string(), theme::DIM),
            ("floom-editor".to_string(), theme::DIMMER),
        ];
        let mut total = 0.0;
        for (index, (text, _)) in items.iter().enumerate() {
            total += text_width(text, theme::SMALL);
            if index + 1 < items.len() {
                total += 14.0;
            }
        }
        let mut right = (bounds.x1 - 12.0 - total).max(x + 8.0);
        for (index, (text, color)) in items.iter().enumerate() {
            label(cx, right, y, text, theme::SMALL, FontWeight::Normal, *color);
            right += text_width(text, theme::SMALL);
            if index + 1 < items.len() {
                right += 7.0;
                separator(cx, bounds, right);
                right += 7.0;
            }
        }
    }
}

/// One vertical rule in a status bar: half the bar's height, centred.
fn separator(cx: &mut dyn PaintContext, bounds: Rect, x: f64) {
    cx.fill_rect(
        Rect {
            x0: x,
            y0: bounds.y0 + 7.0,
            x1: x + 1.0,
            y1: bounds.y1 - 7.0,
        },
        theme::BORDER,
    );
}

impl WidgetAny for StatusBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

// ---------------------------------------------------------------------------
// Editor pane
// ---------------------------------------------------------------------------

/// The editor, plus the window-level shortcuts the text does not claim.
///
/// A key the editor does not consume bubbles to this widget — it is the
/// editor's parent — and this is where `Cmd/Ctrl + S`, `W` and the numbered
/// tab jumps live. `Tab` never arrives: the host consumes it for focus
/// navigation before any widget sees it, which is a documented host rule and
/// why the demo does not pretend to indent on it.
#[derive(Clone)]
pub struct EditorPane {
    editor: TextEditor,
    shared: Shared,
}

impl EditorPane {
    pub fn new(editor: TextEditor, shared: Shared) -> Self {
        Self { editor, shared }
    }
}

impl Widget for EditorPane {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        let editor = self.editor.clone();
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || Box::new(editor.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().fill().stack(Axis::Vertical, 0.0))
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Key(key) = cx.event() else {
            return false;
        };
        if key.phase == KeyPhase::Up {
            return false;
        }
        let command =
            key.modifiers.contains(Modifiers::CONTROL) || key.modifiers.contains(Modifiers::META);
        if !command {
            return false;
        }
        match key.key {
            Key::Char('s') => {
                self.shared.save();
                true
            }
            Key::Char('w') => {
                if let Some(active) = self.shared.app().active {
                    self.shared.close(active);
                }
                true
            }
            Key::Char(digit @ '1'..='9') => {
                self.shared.activate(digit as usize - '1' as usize);
                true
            }
            _ => false,
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}
}

impl WidgetAny for EditorPane {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
