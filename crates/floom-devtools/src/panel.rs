//! The devtools panel — the visible half of [`crate::Inspector`].
//!
//! Electron's devtools are one line of code and a keystroke away: `require('electron-devtools')`
//! in, then Ctrl+Shift+I (or the menu item) toggles the panel. The same deal,
//! spelled for Floom:
//!
//! ```ignore
//! floom::devtools::attach(&mut host);      // one line, once, at startup
//! // ... then press F12 at any time to show/hide the panel.
//! ```
//!
//! ## What this module owns
//!
//! Three things the headless inspector deliberately did not:
//!
//! 1. **A surface to draw on.** The panel is a real widget tree built from
//!    engine primitives ([`Surface`], [`Stack`], [`Text`]), so it survives in
//!    the same host as the app it inspects — no second window, no second
//!    scene, no port.
//! 2. **The toggle.** [`DevtoolsPanel::handle_key`] consumes the shortcut
//!    (F12 by default, `Ctrl+Shift+I` on the platforms where F12 is taken by
//!    the OS) and flips [`DevtoolsPanel::visible`].
//! 3. **The readout.** Each frame the panel rebuilds its rows from the
//!    inspector's own snapshots — tree, style, layout, semantics — so what it
//!    shows is the retained tree as it is, not a copy taken at attach time.
//!
//! ## Why the panel lives here and not in `floom-app`
//!
//! The panel is a widget, and widgets belong to the engine. `floom-app` is the
//! host: it mounts what it is handed and dispatches what it receives. Putting
//! the panel in the host would make every app carry a devtools panel whether
//! or not the `devtools` feature is on, which is precisely what the feature
//! exists to prevent.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use floom_core::{
    Axis, Child, ChildKey, Color, ElementId, EventContext, Key, KeyEvent, KeyPhase, LayoutRequest,
    Modifiers, Rect, Widget,
};
use floom_widgets::{Stack, Surface, Text};

use crate::{Inspector, SemanticsEntry, UpdatePath};
use floom_inspector::TreeNodeInfo;

/// The shortcut that toggles the panel, as a predicate over a key event.
///
/// F12 everywhere, plus `Ctrl+Shift+I` for the platforms where F12 belongs to
/// the OS (Windows volume, some Linux desktops). A predicate rather than a
/// stored key so the caller can supply its own binding — a debug build that
/// wants `~` instead, say — without this crate learning a keybinding registry.
pub type DevtoolsShortcut = Arc<dyn Fn(&KeyEvent) -> bool + Send + Sync + 'static>;

/// The default shortcut: F12, or Ctrl+Shift+I.
pub fn default_shortcut() -> DevtoolsShortcut {
    Arc::new(|event: &KeyEvent| {
        if event.phase != KeyPhase::Down {
            return false;
        }
        if event.key == Key::Named(floom_core::NamedKey::Function(12)) {
            return true;
        }
        event.key == Key::Char('i')
            && event.modifiers.contains(Modifiers::CONTROL)
            && event.modifiers.contains(Modifiers::SHIFT)
    })
}

/// What the panel shows, and whether it shows at all.
///
/// This is the state the caller owns: one `Arc<Mutex<DevtoolsState>>`, handed to
/// both the panel widget (which reads it) and the app's key handler (which
/// flips `visible`). It is a plain mutex-guarded struct rather than a signal
/// because the toggle comes from *outside* the widget tree — from the event
/// loop, before any widget is consulted — and the engine's signals are
/// `Rc`-based and cannot cross that boundary.
///
/// It is `Clone` so a readout can take one by value: the panel reads the shared
/// cell, and a caller composing rows from the same state wants the same answer
/// without holding the lock across the read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolsState {
    visible: bool,
    /// Which inspector readout the panel is showing.
    tab: DevtoolsTab,
    /// The element the readout is about, as a live-tree key.
    selected: Option<String>,
}

/// The readouts the panel can show, in the order the tabs appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DevtoolsTab {
    /// The element tree.
    #[default]
    Tree,
    /// The selected element's resolved style, across interaction states.
    Style,
    /// The selected element's layout request.
    Layout,
    /// The semantics tree.
    Semantics,
    /// The bindings targeting an element and the ancestors they invalidate.
    Updates,
}

impl DevtoolsState {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: DevtoolsTab::default(),
            selected: None,
        }
    }

    /// Shows the panel. Also the `show: true` of the one-line integration.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hides the panel. Also the `show: false` of the one-line integration.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Flips visibility. What the shortcut does.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Which readout is showing.
    pub fn tab(&self) -> DevtoolsTab {
        self.tab
    }

    /// Selects a readout.
    pub fn set_tab(&mut self, tab: DevtoolsTab) {
        self.tab = tab;
        if self.selected.is_none() {
            // A readout about an element needs one; the root is the only one
            // guaranteed to exist, and it is the useful default anyway.
            self.selected = None;
        }
    }

    /// The key of the element the readout is about, if one was picked.
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Picks the element the readout is about.
    pub fn select(&mut self, key: impl Into<String>) {
        self.selected = Some(key.into());
    }
}

impl Default for DevtoolsState {
    fn default() -> Self {
        Self::new()
    }
}

/// The visible panel, mounted as an overlay above the app's own tree.
///
/// The panel is inert while [`DevtoolsState::visible`] is false: it mounts no
/// children and paints nothing, so an app that never opens the devtools pays
/// one element in the tree and nothing else.
pub struct DevtoolsPanel {
    state: Arc<Mutex<DevtoolsState>>,
    /// Builds the readout rows from the inspector's snapshots.
    ///
    /// A closure rather than an `Inspector` because the panel outlives any one
    /// frame's borrow of the live tree: the host owns the tree, the panel owns
    /// itself, and a shared cell is the only thing both can name.
    readout: Arc<dyn Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static>,
    /// Called with the panel's own bounds whenever it opens or resizes, so the
    /// app can position a "devtools is open" indicator of its own.
    on_change: Option<Arc<dyn Fn(bool) + Send + Sync + 'static>>,
    /// The visibility as of the last `update`, so a change is a change.
    reported: Option<bool>,
}

/// One line of the panel's readout.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadoutRow {
    /// The row's label, drawn first and dimmer.
    pub label: String,
    /// The row's value, drawn after the label.
    pub value: String,
    /// How deep the row sits in the tree it is part of. `0` is a top-level row;
    /// each level indents by one step, the way Chrome's element tree nests.
    pub depth: usize,
    /// Whether the row is a heading rather than a value.
    ///
    /// A heading is drawn bold and takes no value: it separates the panel's
    /// sections — "Tree", "Styles", "Layout" — so a reader can find the part
    /// they want without reading the whole panel.
    pub heading: bool,
}

impl ReadoutRow {
    /// A plain label/value row at the top level.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            depth: 0,
            heading: false,
        }
    }

    /// A row nested `depth` levels deep.
    pub fn nested(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// A section heading, which takes no value.
    pub fn heading(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            depth: 0,
            heading: true,
        }
    }
}

impl DevtoolsPanel {
    pub fn new(
        state: Arc<Mutex<DevtoolsState>>,
        readout: impl Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
    ) -> Self {
        Self {
            state,
            readout: Arc::new(readout),
            on_change: None,
            reported: None,
        }
    }

    /// Called with the panel's visibility whenever it changes.
    pub fn on_visibility_change(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The panel's rows right now, from the caller's readout.
    fn rows(&self) -> Vec<ReadoutRow> {
        let Ok(state) = self.state.lock() else {
            return Vec::new();
        };
        if !state.visible() {
            return Vec::new();
        }
        (self.readout)(&state)
    }

    /// Handles a key event for the shortcut.
    ///
    /// Returns whether the event was consumed. The caller — the event loop, or
    /// a key-handling widget above this one — should stop propagating a
    /// consumed event, so pressing F12 while a text field has focus toggles
    /// the devtools rather than typing into the field.
    pub fn handle_key(&mut self, event: &KeyEvent, shortcut: &DevtoolsShortcut) -> bool {
        if !shortcut(event) {
            return false;
        }
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        let before = state.visible();
        state.toggle();
        let after = state.visible();
        drop(state);
        if before != after {
            // Mark it reported here, so the host's next `update` does not
            // report the same change a second time.
            self.reported = Some(after);
            if let Some(on_change) = &self.on_change {
                on_change(after);
            }
        }
        true
    }

    /// Whether the panel is showing.
    pub fn visible(&self) -> bool {
        self.state.lock().map(|s| s.visible()).unwrap_or(false)
    }
}

impl Widget for DevtoolsPanel {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> ElementId {
        cx.alloc_child()
    }
    fn update(&mut self, cx: &mut dyn floom_core::UpdateContext, id: ElementId) {
        let visible = self.visible();
        if self.reported != Some(visible) {
            self.reported = Some(visible);
            if let Some(on_change) = &self.on_change {
                on_change(visible);
            }
        }
        let _ = (cx, id);
    }

    fn children(&self) -> Vec<Child> {
        if !self.visible() {
            return Vec::new();
        }
        let rows = self.rows();
        let mut stack = Stack::new(Axis::Vertical);
        for (index, row) in rows.iter().enumerate() {
            // One indent step per level of depth. A heading carries no value and
            // is drawn heavier; a row's label and value are separated, and the
            // value is dimmed so the eye lands on the label first.
            let indent = "  ".repeat(row.depth);
            let text = if row.heading {
                row.label.clone()
            } else if row.value.is_empty() {
                format!("{indent}{}", row.label)
            } else {
                format!("{indent}{}  {}", row.label, row.value)
            };
            let size = if row.heading { 12.0 } else { 11.0 };
            stack = stack.push_keyed(index as u64, move || {
                Box::new(Text::new(text.clone()).size(size))
            });
        }
        // The child builder is an `Fn`, so it is called more than once and every
        // call needs its own stack; the rows it holds are cheap to clone.
        let stack = std::sync::Arc::new(stack);
        let outer = stack.clone();
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || {
                let stack = outer.clone();
                Box::new(
                    Surface::new()
                        .background(panel_background())
                        .push(move || Box::new((*stack).clone())),
                )
            },
        )]
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        // Escape closes the panel, the way a devtools drawer does.
        if let floom_core::Event::Key(event) = cx.event() {
            if event.phase == KeyPhase::Down
                && event.key == Key::Named(floom_core::NamedKey::Escape)
            {
                if let Ok(mut state) = self.state.lock() {
                    state.hide();
                    return true;
                }
            }
        }
        false
    }
}

/// The Elements panel, whole: the tree, the breadcrumb, and the selected
/// element's style and layout.
///
/// This is the composition a caller hands to [`DevtoolsPanel::new`] when they
/// want the Chrome layout rather than a custom one. It reads the tab out of
/// `state`, so the panel's own tab switching drives which half is shown.
pub fn elements_readout(inspector: &Inspector<'_>, state: &DevtoolsState) -> Vec<ReadoutRow> {
    let selected = state.selected();
    let mut rows = vec![ReadoutRow::heading("Elements")];
    rows.extend(tree_rows(inspector, selected));
    if let Some(key) = selected {
        rows.extend(breadcrumb_rows(inspector, key));
        match state.tab() {
            DevtoolsTab::Style => rows.extend(style_rows(inspector, key)),
            DevtoolsTab::Layout => rows.extend(layout_rows(inspector, key)),
            _ => {}
        }
    }
    rows
}

/// The Performance panel: the frames the profiler recorded, with the budget
/// verdict, then the allocation and cache summaries.
///
/// The rows are newest frame first, the way Chrome's timeline reads: the frame
/// that just happened is the one you are asking about.
pub fn performance_rows(snapshot: &floom_inspector::PerformanceSnapshot) -> Vec<ReadoutRow> {
    let mut rows = vec![ReadoutRow::heading("Performance")];
    for frame in snapshot.frames.iter().rev().take(8) {
        let total: u64 = frame.stages.iter().map(|s| s.duration_ns).sum();
        let ms = total as f64 / 1_000_000.0;
        let verdict = if frame.budget_exceeded { "OVER" } else { "ok" };
        rows.push(ReadoutRow::new(
            format!("frame {}", frame.index),
            format!("{ms:.2} ms  {verdict}"),
        ));
        // One row per stage, indented under its frame, so a slow stage is the
        // thing that stands out rather than a number in a sum.
        for stage in &frame.stages {
            let stage_ms = stage.duration_ns as f64 / 1_000_000.0;
            rows.push(ReadoutRow::new(&stage.stage, format!("{stage_ms:.2} ms")).nested(1));
        }
    }
    if snapshot.frames.is_empty() {
        rows.push(ReadoutRow::new("(no frames recorded)", ""));
    }
    let alloc = &snapshot.allocations;
    rows.push(ReadoutRow::heading("Allocations"));
    rows.push(ReadoutRow::new("live", alloc.live().to_string()));
    rows.push(ReadoutRow::new("bytes", alloc.bytes_allocated.to_string()));
    for cache in &snapshot.caches {
        let rate = cache
            .hit_rate()
            .map(|r| format!("{:.0}%", r * 100.0))
            .unwrap_or_else(|| "idle".into());
        rows.push(ReadoutRow::new(
            format!("cache {}", cache.name),
            format!("{} entries, {rate} hits", cache.entries),
        ));
    }
    rows
}

/// The Console panel: what the error boundary caught, newest first.
///
/// The boundary is the framework's own record of "something failed and the app
/// kept running", which is exactly what a console is for — and it is the one
/// diagnostic source that has no other surface.
pub fn console_rows(reports: &[floom_error::Report]) -> Vec<ReadoutRow> {
    let mut rows = vec![ReadoutRow::heading("Console")];
    if reports.is_empty() {
        rows.push(ReadoutRow::new("(no errors caught)", ""));
    }
    for report in reports.iter().rev().take(12) {
        rows.push(
            ReadoutRow::new(
                format!("{:?}", report.severity),
                report.message.clone(),
            )
            .nested(1),
        );
        if !report.context.is_empty() {
            rows.push(ReadoutRow::new("context", report.context.clone()).nested(2));
        }
        if !report.location.is_empty() {
            rows.push(ReadoutRow::new("at", report.location.clone()).nested(2));
        }
    }
    rows
}

/// A float the way a devtools panel prints one: no trailing zeros, and no
/// `.0` on a whole number, so `12` stays `12`.
fn fmt_f32(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// The panel's own fill: a dark, near-opaque slab that reads as "tooling" over
/// whatever the app drew.
fn panel_background() -> floom_core::Brush {
    floom_core::Brush::from(Color::rgba(24, 26, 32, 242))
}

/// The readout a panel shows when the caller has supplied nothing better.
///
/// It answers "is it wired up?" — the tree's element count and the semantics
/// entries — which is enough to tell a working integration from a silent one.
pub fn default_readout(inspector: &Inspector<'_>) -> Vec<ReadoutRow> {
    vec![
        ReadoutRow::new("elements", inspector.tree_snapshot().nodes.len().to_string()),
        ReadoutRow::new("semantics", inspector.semantics().len().to_string()),
    ]
}

// --- the tree readout -------------------------------------------------------
//
// Chrome's element panel is a tree you expand; this is the same shape as rows,
// with the nesting carried in `ReadoutRow::depth`. A row is selected by its
// key, and the selected one is marked, so a reader can see where they are.

/// The selection marker for a tree row.
///
/// A filled triangle when the row is the selected one, a space of the same
/// width when it is not: the column has to line up whether or not anything is
/// selected, and the indent comes from `ReadoutRow::depth` alone. A marker that
/// carried its own indent would double it.
fn marker(selected: Option<&str>, key: &str) -> char {
    if selected == Some(key) {
        '▸'
    } else {
        ' '
    }
}

/// The element tree as rows, nested by depth.
///
/// `selected` marks one node the way Chrome's selection does, so the panel can
/// show which element the style and layout readouts are about.
pub fn tree_rows(inspector: &Inspector<'_>, selected: Option<&str>) -> Vec<ReadoutRow> {
    let snapshot = inspector.tree_snapshot();
    let by_parent: std::collections::HashMap<Option<u64>, Vec<&TreeNodeInfo>> =
        snapshot.nodes.iter().fold(
            std::collections::HashMap::new(),
            |mut acc, node| {
                acc.entry(node.parent).or_default().push(node);
                acc
            },
        );

    let mut rows = Vec::new();
    // The root's own row first: a tree that starts at the root's children hides
    // the one node every other node hangs from.
    if let Some(root) = snapshot.nodes.iter().find(|n| n.id == snapshot.root) {
        rows.push(tree_row(root, selected, 0));
    }
    // Depth-first from the root's children, so the order a browser's inspector
    // shows.
    let mut stack: Vec<(Option<u64>, usize)> = vec![(Some(snapshot.root), 1)];
    while let Some((parent, depth)) = stack.pop() {
        let Some(children) = by_parent.get(&parent) else {
            continue;
        };
        // Push in reverse so the first child is visited first.
        for node in children.iter().rev() {
            rows.push(tree_row(node, selected, depth));
            stack.push((Some(node.id), depth + 1));
        }
    }
    rows
}

/// One tree row: the marker column, the node's key, and the node's kind.
///
/// `depth` is the row's only indent — the marker is one character wide and adds
/// none, so a marked row and an unmarked one line up.
fn tree_row(node: &TreeNodeInfo, selected: Option<&str>, depth: usize) -> ReadoutRow {
    let key = node.key.as_deref().unwrap_or("?");
    ReadoutRow {
        label: format!("{}{key}", marker(selected, key)),
        value: node.kind.clone(),
        depth,
        heading: false,
    }
}

/// The breadcrumb from the root to the selected node — Chrome's
/// `html > body > div.button` line, one row.
pub fn breadcrumb_rows(inspector: &Inspector<'_>, key: &str) -> Vec<ReadoutRow> {
    let snapshot = inspector.tree_snapshot();
    let Some(node) = snapshot
        .nodes
        .iter()
        .find(|n| n.key.as_deref() == Some(key))
    else {
        return vec![ReadoutRow::heading("Breadcrumb"), ReadoutRow::new("(nothing selected)", "")];
    };
    // `path_to_root` walks *up* — selected first, root last. A breadcrumb reads
    // the other way, the way a browser's does, so the path is reversed.
    let mut path = snapshot.path_to_root(node.id).unwrap_or_default();
    path.reverse();
    let names: Vec<String> = path
        .iter()
        .filter_map(|id| {
            snapshot
                .nodes
                .iter()
                .find(|n| n.id == *id)
                .and_then(|n| n.key.clone())
        })
        .collect();
    vec![
        ReadoutRow::heading("Breadcrumb"),
        ReadoutRow::new("path", names.join(" > ")),
    ]
}

// --- the style readout ------------------------------------------------------

/// The resolved style of one element, across the inspected states.
///
/// This is Chrome's "Computed" pane: every property that resolves, in every
/// state the element can be in, so a hover style is visible without hovering.
pub fn style_rows(inspector: &Inspector<'_>, key: &str) -> Vec<ReadoutRow> {
    let snapshot = inspector.style_snapshot();
    let node_key = inspector.tree_snapshot().nodes.iter().find_map(|n| {
        if n.key.as_deref() == Some(key) {
            Some(n.id)
        } else {
            None
        }
    });
    let Some(id) = node_key else {
        return vec![ReadoutRow::heading("Styles"), ReadoutRow::new("(nothing selected)", "")];
    };

    let mut rows = vec![ReadoutRow::heading("Styles")];
    for entry in snapshot.entries.iter().filter(|e| e.id == id) {
        let style = &entry.style;
        rows.push(ReadoutRow::heading(&entry.state).nested(1));
        let mut props: Vec<ReadoutRow> = Vec::new();
        if let Some(c) = &style.background {
            props.push(ReadoutRow::new(
                "background",
                format!("#{:02X}{:02X}{:02X}@{}", c.r, c.g, c.b, c.a),
            ));
        }
        if let Some(c) = &style.foreground {
            props.push(ReadoutRow::new(
                "color",
                format!("#{:02X}{:02X}{:02X}@{}", c.r, c.g, c.b, c.a),
            ));
        }
        if let Some(v) = style.opacity {
            props.push(ReadoutRow::new("opacity", fmt_f32(v)));
        }
        if let Some(v) = style.font_size {
            props.push(ReadoutRow::new("font-size", fmt_f32(v)));
        }
        if let Some(v) = style.padding {
            props.push(ReadoutRow::new("padding", fmt_f32(v)));
        }
        if let Some(v) = style.radius {
            props.push(ReadoutRow::new("radius", fmt_f32(v)));
        }
        if let Some(v) = style.border_width {
            props.push(ReadoutRow::new("border-width", fmt_f32(v)));
        }
        if let Some(c) = &style.border_color {
            props.push(ReadoutRow::new(
                "border-color",
                format!("#{:02X}{:02X}{:02X}@{}", c.r, c.g, c.b, c.a),
            ));
        }
        if let Some(v) = style.elevation {
            props.push(ReadoutRow::new("elevation", fmt_f32(v)));
        }
        if props.is_empty() {
            props.push(ReadoutRow::new("(no resolved properties)", ""));
        }
        rows.extend(props.into_iter().map(|r| r.nested(2)));
    }
    rows
}

// --- the layout readout -----------------------------------------------------

/// The layout box of one element — Chrome's box-model overlay, as rows.
///
/// The element's own request rather than its measured box: a request is what
/// the widget *asked* for, which is the thing a layout bug is usually about.
pub fn layout_rows(inspector: &Inspector<'_>, key: &str) -> Vec<ReadoutRow> {
    let snapshot = inspector.layout_snapshot();
    let id = inspector.tree_snapshot().nodes.iter().find_map(|n| {
        if n.key.as_deref() == Some(key) {
            Some(n.id)
        } else {
            None
        }
    });
    let Some(id) = id else {
        return vec![ReadoutRow::heading("Layout"), ReadoutRow::new("(nothing selected)", "")];
    };
    let mut rows = vec![ReadoutRow::heading("Layout")];
    if let Some(entry) = snapshot.entries.iter().find(|e| e.id == id) {
        let layout = &entry.layout;
        rows.push(ReadoutRow::new(
            "size",
            format!(
                "{} x {}",
                layout.width.map(fmt_f32).unwrap_or_else(|| "auto".into()),
                layout.height.map(fmt_f32).unwrap_or_else(|| "auto".into()),
            ),
        ));
        if let Some(v) = layout.flex_grow {
            rows.push(ReadoutRow::new("flex-grow", fmt_f32(v)));
        }
        if let Some(v) = &layout.axis {
            rows.push(ReadoutRow::new("axis", v.clone()));
        }
        if let Some(v) = layout.gap {
            rows.push(ReadoutRow::new("gap", fmt_f32(v)));
        }
        if let Some(v) = &layout.main_align {
            rows.push(ReadoutRow::new("main-align", v.clone()));
        }
        if let Some(v) = &layout.cross_align {
            rows.push(ReadoutRow::new("cross-align", v.clone()));
        }
        if let Some(v) = &layout.position {
            rows.push(ReadoutRow::new("position", v.clone()));
        }
        if let Some(inset) = &layout.inset {
            let parts: Vec<String> = inset
                .iter()
                .map(|v| v.map(fmt_f32).unwrap_or_else(|| "auto".into()))
                .collect();
            rows.push(ReadoutRow::new("inset", format!("[{}]", parts.join(", "))));
        }
    } else {
        rows.push(ReadoutRow::new("(no layout request)", ""));
    }
    rows
}

/// The semantics rows a panel can list.
pub fn semantics_rows(entries: &[SemanticsEntry]) -> Vec<ReadoutRow> {
    entries
        .iter()
        .map(|e| {
            ReadoutRow::new(
                format!("{:?}", e.role),
                if e.label.is_empty() {
                    format!("#{}", e.element.raw())
                } else {
                    e.label.clone()
                },
            )
        })
        .collect()
}

/// The rows for one element's update path: the bindings targeting it and the
/// ancestors a layout invalidation reaches.
pub fn update_rows(path: &UpdatePath) -> Vec<ReadoutRow> {
    let mut rows: Vec<ReadoutRow> = path
        .bindings
        .iter()
        .map(|b| {
            ReadoutRow::new(
                "binding",
                match &b.source {
                    Some(source) => {
                        format!("{} -> {} ({} runs)", source.label, source.property, b.runs)
                    }
                    None => format!("{:?} ({} runs)", b.id, b.runs),
                },
            )
        })
        .collect();
    for ancestor in &path.ancestors {
        rows.push(ReadoutRow::new("ancestor", ancestor.as_str().to_string()));
    }
    rows
}

/// The panel's own bounds, in the viewport's logical pixels.
///
/// A docked panel on the window's trailing edge, the way a browser's inspector
/// docks to the right: the top-left corner is the app's, the rest is tooling.
pub fn panel_bounds(viewport: Rect) -> Rect {
    let width = (viewport.width() * 0.34).clamp(280.0, 520.0);
    let height = viewport.height();
    Rect::from_origin_size(
        floom_types::Point::new(viewport.width() - width, 0.0),
        floom_types::Size::new(width, height),
    )
}

/// The panel is a decorative composite: it publishes no semantics of its own.
///
/// An inspector panel is not content — assistive technology should read the app
/// it is inspecting, not the tool inspecting it. So the hint is `None`, which
/// is what an unlabelled node reports, rather than an invented empty one.
impl floom_core::WidgetAny for DevtoolsPanel {
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
