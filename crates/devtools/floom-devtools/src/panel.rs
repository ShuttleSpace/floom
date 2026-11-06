// SPDX-License-Identifier: MIT
//! The devtools panel — the visible half of [`crate::Inspector`].
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

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use floom_core::{
    Axis, Child, ChildKey, Color, ElementId, EventContext, FontWeight, Key, KeyEvent, KeyPhase,
    LayoutRequest, Modifiers, Rect, Widget,
};
use floom_widgets::{Button, Scroll, Stack, Surface, Text};

use crate::tree::row_child;
use crate::{Inspector, SemanticsEntry, UpdatePath};
use floom_inspector::TreeNodeInfo;

pub use crate::tree::RowNode;

/// The shortcut that toggles the panel, as a predicate over a key event.
///
/// F12 everywhere, plus `Ctrl+Shift+I` for the platforms where F12 belongs to
/// the OS (Windows volume, some Linux desktops). A predicate rather than a
/// stored key so the caller can supply its own binding — a debug build that
/// wants `~` instead, say — without this crate learning a keybinding registry.
pub type DevtoolsShortcut = Arc<dyn Fn(&KeyEvent) -> bool + Send + Sync + 'static>;

/// The default shortcut: F12, or `Ctrl+Shift+I`, or `Cmd+Shift+I` on macOS.
pub fn default_shortcut() -> DevtoolsShortcut {
    Arc::new(|event: &KeyEvent| {
        if event.phase != KeyPhase::Down {
            return false;
        }
        if event.key == Key::Named(floom_core::NamedKey::Function(12)) {
            return true;
        }
        // `Char('i')` rather than `Char('I')`: the platform layer lowercases the
        // character it maps, because shift is already carried by `modifiers`.
        // Both modifiers are accepted because the two desktop conventions split
        // on which one the platform calls "command" — the same panel, one
        // binding, on the machine the reader is actually using.
        let is_i = event.key == Key::Char('i');
        is_i && event.modifiers.contains(Modifiers::SHIFT)
            && (event.modifiers.contains(Modifiers::CONTROL)
                || event.modifiers.contains(Modifiers::META))
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
    /// The keys of the nodes whose children the reader has collapsed.
    ///
    /// Empty means everything is open, which is the right default for a tree
    /// that is a few dozen rows deep: Chrome opens collapsed, but a panel whose
    /// first frame hides everything but the root looks like it failed to read
    /// the tree at all. A set rather than a per-node flag because the state is
    /// the only thing that survives a rebuild — rows are rebuilt from the
    /// inspector every frame, and a flag on a row would be thrown away with it.
    collapsed: BTreeSet<String>,
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

impl DevtoolsTab {
    /// The tab's name in the tab bar, and the word the readout uses for it.
    pub fn label(self) -> &'static str {
        match self {
            DevtoolsTab::Tree => "Elements",
            DevtoolsTab::Style => "Style",
            DevtoolsTab::Layout => "Layout",
            DevtoolsTab::Semantics => "Semantics",
            DevtoolsTab::Updates => "Updates",
        }
    }

    /// Every tab, in the order the tab bar shows them.
    ///
    /// Tree first because it is the one that answers "what is on screen" without
    /// a selection, and Semantics/Updates last because they are the two a caller
    /// has to feed — a panel that opened on a dead tab would be a panel whose
    /// first impression is a button that does nothing.
    pub const ALL: [DevtoolsTab; 5] = [
        DevtoolsTab::Tree,
        DevtoolsTab::Style,
        DevtoolsTab::Layout,
        DevtoolsTab::Semantics,
        DevtoolsTab::Updates,
    ];

    /// The tabs [`elements_readout`] can fill from the tree alone.
    ///
    /// `elements_readout` composes the tree, the breadcrumb, and the selected
    /// element's style or layout — the three readouts an `Inspector` answers.
    /// Semantics and Updates need data the inspector does not hold (a semantics
    /// entry list and an invalidation path), so offering them here would put two
    /// buttons on the bar that switch to nothing.
    pub const ELEMENTS: [DevtoolsTab; 3] =
        [DevtoolsTab::Tree, DevtoolsTab::Style, DevtoolsTab::Layout];
}

impl DevtoolsState {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: DevtoolsTab::default(),
            selected: None,
            collapsed: BTreeSet::new(),
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
    ///
    /// The selection is deliberately left alone: Style and Layout are both about
    /// *the* selected element, so switching between them is a change of what is
    /// shown, not of what it is shown about. Picking a default here would also
    /// mean the state needed to know the tree's root, which it does not.
    pub fn set_tab(&mut self, tab: DevtoolsTab) {
        self.tab = tab;
    }

    /// The key of the element the readout is about, if one was picked.
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Picks the element the readout is about.
    pub fn select(&mut self, key: impl Into<String>) {
        self.selected = Some(key.into());
    }

    /// Forgets the selection, so Style and Layout have nothing to be about.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// The keys of the nodes whose children are collapsed.
    pub fn collapsed(&self) -> &BTreeSet<String> {
        &self.collapsed
    }

    /// Whether a node's children are currently hidden.
    pub fn is_collapsed(&self, key: &str) -> bool {
        self.collapsed.contains(key)
    }

    /// Flips one node between collapsed and expanded.
    pub fn toggle_collapsed(&mut self, key: impl Into<String>) {
        let key = key.into();
        if !self.collapsed.remove(&key) {
            self.collapsed.insert(key);
        }
    }
}

impl Default for DevtoolsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything a panel's rows are a function of, as one comparable value.
///
/// A type alias rather than a struct so a test can compare it to a tuple literal:
/// the point of the value is that `update` can tell "same rows" from "different
/// rows" with `==`, and a tuple does that without a second type to learn.
type MountedRowsKey = (bool, DevtoolsTab, Option<String>, BTreeSet<String>);

/// Builds the readout rows from the inspector's snapshots. Boxed as a trait
/// object because the panel outlives any one frame's borrow of the live tree.
type ReadoutBuilder = Arc<dyn Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static>;

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
    readout: ReadoutBuilder,
    /// Called with the panel's own bounds whenever it opens or resizes, so the
    /// app can position a "devtools is open" indicator of its own.
    on_change: Option<Arc<dyn Fn(bool) + Send + Sync + 'static>>,
    /// The visibility as of the last `update`, so a change is a change.
    reported: Option<bool>,
    /// The visibility the host last mounted children for.
    ///
    /// Distinct from `reported` on purpose: that one drives the caller's
    /// callback and `handle_key` advances it the moment the state flips, before
    /// any frame has run. This one may only be advanced by `update`, because
    /// `update` is the only place that can tell the host its children changed.
    /// It starts at `false` because that is what the host has actually mounted:
    /// a panel arrives with no rows under it, whatever its state cell says.
    mounted: bool,
    /// The readout the host last mounted children for: visibility, tab,
    /// selection and the collapsed set.
    ///
    /// Visibility alone is not enough. Switching tab changes every row below
    /// the breadcrumb, a pick changes which element the rows are about, and a
    /// click on a chevron changes which rows exist at all — all three arrive
    /// through the shared cell rather than through `update`'s arguments, so a
    /// panel that only compared visibility would keep drawing the previous
    /// tab's rows until something else happened to dirty it.
    mounted_rows: Option<MountedRowsKey>,
    /// The tabs to put on the bar, in order. Empty means no bar at all.
    ///
    /// Empty is the right default for [`DevtoolsPanel::new`]: the caller's
    /// readout decides what a tab means, and a panel that offered buttons for
    /// readouts the caller never wrote would be a panel whose buttons switch to
    /// nothing. [`DevtoolsPanel::with_tabs`] is how a caller that *does* fill
    /// several tabs says so.
    tabs: Vec<DevtoolsTab>,
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
    /// The tree node this row stands for, when it stands for one.
    ///
    /// `Some` makes the row an interactive [`crate::tree::TreeRow`] — chevron,
    /// click to select, right click for the menu. `None` keeps it a line of text.
    /// The node is carried as data rather than looked up at paint time so a
    /// readout stays a pure function of the inspector and the state.
    pub node: Option<RowNode>,
}

impl ReadoutRow {
    /// A plain label/value row at the top level.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            depth: 0,
            heading: false,
            node: None,
        }
    }

    /// A row nested `depth` levels deep.
    pub fn nested(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// A row that stands for a tree node, and can therefore be clicked.
    pub fn node(label: impl Into<String>, value: impl Into<String>, node: RowNode) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            depth: 0,
            heading: false,
            node: Some(node),
        }
    }

    /// A section heading, which takes no value.
    pub fn heading(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            depth: 0,
            heading: true,
            node: None,
        }
    }
}

impl DevtoolsPanel {
    pub fn new(
        state: Arc<Mutex<DevtoolsState>>,
        readout: impl Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
    ) -> Self {
        // A panel built while the state already says "shown" is mounted with its
        // rows: the host reconciles the overlay's subtree as it opens it. So the
        // baseline is the visibility at construction, not a constant `false`.
        let mounted = state.lock().map(|s| s.visible()).unwrap_or(false);
        let mounted_rows = {
            let guard = state.lock().ok();
            let visible = guard.as_ref().is_some_and(|s| s.visible());
            let tab = guard.as_ref().map(|s| s.tab()).unwrap_or_default();
            let selected = guard
                .as_ref()
                .and_then(|s| s.selected().map(str::to_string));
            let collapsed = guard
                .as_ref()
                .map(|s| s.collapsed().clone())
                .unwrap_or_default();
            if visible {
                Some((true, tab, selected, collapsed))
            } else {
                Some((false, DevtoolsTab::default(), None, BTreeSet::new()))
            }
        };
        Self {
            state,
            readout: Arc::new(readout),
            on_change: None,
            reported: None,
            mounted,
            mounted_rows,
            tabs: Vec::new(),
        }
    }

    /// Puts a tab bar above the readout, with one button per tab in `tabs`.
    ///
    /// A click flips [`DevtoolsState::tab`] in the shared cell; the panel's next
    /// `update` sees the change and rebuilds the rows, so the readout closure
    /// only has to read `state.tab()` — it never has to know a tab bar exists.
    ///
    /// Pass [`DevtoolsTab::ELEMENTS`] for [`elements_readout`]: the tabs it can
    /// actually fill. [`DevtoolsTab::ALL`] is for a caller whose own readout
    /// answers all five.
    pub fn with_tabs(mut self, tabs: impl IntoIterator<Item = DevtoolsTab>) -> Self {
        self.tabs = tabs.into_iter().collect();
        self
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

    /// What the rows are a function of, as one comparable value.
    ///
    /// Visibility, tab, selection and the collapsed set: everything `rows()`
    /// reads from the shared cell that the host cannot see from outside. A
    /// hidden panel has no rows at all, so it collapses to `(false, ..)` rather
    /// than carrying a stale tab.
    fn mounted_rows_key(&self, visible: bool) -> MountedRowsKey {
        if !visible {
            return (false, DevtoolsTab::default(), None, BTreeSet::new());
        }
        let Ok(state) = self.state.lock() else {
            return (visible, DevtoolsTab::default(), None, BTreeSet::new());
        };
        (
            visible,
            state.tab(),
            state.selected().map(str::to_string),
            state.collapsed().clone(),
        )
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
        // Opening or closing the panel changes what `children()` declares — no
        // rows when hidden, the whole readout when shown — so the host has to be
        // told to consult it again. `mark_dirty` alone is a repaint of the
        // previous frame's children, which for a panel that just opened is
        // nothing at all: the panel would report itself visible while the host
        // still mounted no rows.
        //
        // Tracked separately from `reported` because `handle_key` advances that
        // one the moment the state flips, before any frame has run — and a flip
        // the host has not yet seen still has to be marked.
        //
        // The tab and the selection are compared too: both change the rows
        // without changing visibility, and neither comes through an argument to
        // `update`. A panel that only watched visibility would show the previous
        // tab until an unrelated repaint happened to rebuild it.
        let rows = self.mounted_rows_key(visible);
        if self.mounted_rows.as_ref() != Some(&rows) || self.mounted != visible {
            self.mounted = visible;
            self.mounted_rows = Some(rows);
            cx.mark_children_dirty(id);
            cx.mark_layout_dirty(id);
            cx.mark_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        if !self.visible() {
            return Vec::new();
        }
        let rows = self.rows();
        let mut stack = Stack::new(Axis::Vertical);
        let mut next_key = 0u64;
        // The tab bar first, then the rows: the bar is chrome, the rows are the
        // answer, and a bar below the rows would be a bar the reader has to
        // scroll to reach.
        if !self.tabs.is_empty() {
            let tabs = self.tabs.clone();
            let state = self.state.clone();
            stack = stack.push_keyed(next_key, move || {
                let active = state.lock().map(|s| s.tab()).unwrap_or_default();
                let mut bar = Stack::new(Axis::Horizontal).gap(4.0);
                for (index, tab) in tabs.iter().copied().enumerate() {
                    let state = state.clone();
                    let selected = tab == active;
                    // The button is built here rather than captured: `Button`
                    // owns a `Box<dyn Fn()>` callback and is deliberately not
                    // `Clone`, while the builder closure has to be replayable.
                    bar = bar.push_keyed(index as u64, move || {
                        let state = state.clone();
                        let mut button = Button::new(tab.label())
                            .focusable(false)
                            .radius(3.0)
                            .color(if selected {
                                tab_active_fill()
                            } else {
                                tab_idle_fill()
                            })
                            .text_color(if selected {
                                Color::WHITE
                            } else {
                                Color::rgb(0x9a, 0xa2, 0xb2)
                            });
                        button = button.on_click(move || {
                            if let Ok(mut state) = state.lock() {
                                state.set_tab(tab);
                            }
                        });
                        Box::new(button)
                    });
                }
                Box::new(bar)
            });
            next_key += 1;
        }
        for (index, row) in rows.iter().enumerate() {
            let key = next_key + index as u64;
            // A tree row draws itself: the selected one needs a filled
            // background behind its label, and this host paints a flat scene in
            // node order, so no sibling box can be drawn behind another
            // sibling's text.
            if let Some(node) = &row.node {
                stack = stack.child(row_child(
                    key,
                    self.state.clone(),
                    node.clone(),
                    row.value.clone(),
                    row.depth,
                ));
                continue;
            }
            // Every other row is a line of text. The indent is part of the
            // string rather than a margin so a row is one element, not two: a
            // readout can be a hundred rows, and a wrapper per row would double
            // the tree for a number.
            let indent = "  ".repeat(row.depth);
            let text = if row.heading {
                row.label.clone()
            } else if row.value.is_empty() {
                format!("{indent}{}", row.label)
            } else {
                format!("{indent}{}  {}", row.label, row.value)
            };
            let size = if row.heading { 12.0 } else { 11.0 };
            let heading = row.heading;
            stack = stack.push_keyed(key, move || {
                // The panel is dark and `Text` defaults to black: a row drawn in
                // the default ink is a row nobody can read.
                Box::new(
                    Text::new(text.clone())
                        .size(size)
                        .weight(if heading {
                            FontWeight::SemiBold
                        } else {
                            FontWeight::Normal
                        })
                        .color(if heading {
                            Color::rgb(0xf2, 0xf4, 0xf8)
                        } else {
                            Color::rgb(0xd8, 0xdc, 0xe4)
                        }),
                )
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
                        .padding(floom_core::Insets::uniform(6.0))
                        .push_layout(LayoutRequest::new().fill(), move || {
                            let stack = stack.clone();
                            // Scrolled, because the readout is longer than the
                            // panel: without this the Style and Layout sections
                            // are appended *below* the visible area and the tab
                            // looks like it did nothing at all.
                            Box::new(Scroll::new().child(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().fill(),
                                move || Box::new((*stack).clone()),
                            )))
                        }),
                )
            },
        )]
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        // Escape closes the panel, the way a devtools drawer does.
        if let floom_core::Event::Key(event) = cx.event()
            && event.phase == KeyPhase::Down
            && event.key == Key::Named(floom_core::NamedKey::Escape)
            && let Ok(mut state) = self.state.lock()
        {
            state.hide();
            return true;
        }
        false
    }
}

/// The Elements panel, whole: one tab's worth of readout at a time.
///
/// The tabs are exclusive rather than cumulative, which is the whole reason the
/// panel has a tab bar: Chrome shows the tree *or* the styles of the selected
/// element, not both, and a panel that appended everything below everything else
/// was a panel whose Style and Layout tabs looked like they did nothing — the
/// rows were there, past the bottom of the viewport, behind a tree long enough to
/// fill it.
///
/// The breadcrumb leads every tab, so a reader who switched to Style still knows
/// which element they are looking at.
pub fn elements_readout(inspector: &Inspector<'_>, state: &DevtoolsState) -> Vec<ReadoutRow> {
    let selected = state.selected();
    let mut rows = vec![ReadoutRow::heading("Elements")];
    match state.tab() {
        DevtoolsTab::Tree => {
            rows.extend(tree_rows(inspector, selected, state.collapsed()));
            if let Some(key) = selected {
                rows.extend(breadcrumb_rows(inspector, key));
            }
        }
        DevtoolsTab::Style | DevtoolsTab::Layout => {
            let Some(key) = selected else {
                rows.push(ReadoutRow::new(
                    "No element selected",
                    "click a row in Elements, or press the pick shortcut",
                ));
                return rows;
            };
            rows.extend(breadcrumb_rows(inspector, key));
            if state.tab() == DevtoolsTab::Style {
                rows.extend(style_rows(inspector, key));
            } else {
                rows.extend(layout_rows(inspector, key));
            }
        }
        // The two tabs a caller has to feed. Nothing here invents rows for them:
        // a panel that drew an empty Semantics heading would be claiming a
        // readout it does not have.
        DevtoolsTab::Semantics | DevtoolsTab::Updates => {}
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
pub fn console_rows(reports: &[floom_core::error::Report]) -> Vec<ReadoutRow> {
    let mut rows = vec![ReadoutRow::heading("Console")];
    if reports.is_empty() {
        rows.push(ReadoutRow::new("(no errors caught)", ""));
    }
    for report in reports.iter().rev().take(12) {
        rows.push(
            ReadoutRow::new(format!("{:?}", report.severity), report.message.clone()).nested(1),
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

/// The fill of the tab the panel is showing.
fn tab_active_fill() -> Color {
    Color::rgb(0x3a, 0x3f, 0x4a)
}

/// The fill of a tab the panel is not showing.
///
/// A faint wash rather than nothing at all: a tab with no fill reads as a label,
/// and the whole point of the bar is that each name is a thing that can be
/// pressed. It stays well below the active fill so the active one is the one the
/// eye lands on.
fn tab_idle_fill() -> Color {
    Color::rgba(255, 255, 255, 14)
}

/// The readout a panel shows when the caller has supplied nothing better.
///
/// It answers "is it wired up?" — the tree's element count and the semantics
/// entries — which is enough to tell a working integration from a silent one.
pub fn default_readout(inspector: &Inspector<'_>) -> Vec<ReadoutRow> {
    vec![
        ReadoutRow::new(
            "elements",
            inspector.tree_snapshot().nodes.len().to_string(),
        ),
        ReadoutRow::new("semantics", inspector.semantics().len().to_string()),
    ]
}

// --- the tree readout -------------------------------------------------------
//
// Chrome's element panel is a tree you expand; this is the same shape as rows,
// with the nesting carried in `ReadoutRow::depth`. A row is selected by its
// key, and the selected one is marked, so a reader can see where they are.

/// The element tree as rows, nested by depth and cut where the reader collapsed
/// it.
///
/// `selected` marks one node the way Chrome's selection does, so the panel can
/// show which element the style and layout readouts are about; `collapsed` is the
/// set of nodes whose children are hidden.
///
/// One rule outranks the set: the ancestors of the selected node are always
/// treated as expanded. Selecting a node from the Style tab and then switching
/// back to a tree where that node is inside a collapsed parent would show a
/// selection with nothing selected, and collapsing is a display choice rather
/// than a promise to hide the thing the panel is about.
pub fn tree_rows(
    inspector: &Inspector<'_>,
    selected: Option<&str>,
    collapsed: &BTreeSet<String>,
) -> Vec<ReadoutRow> {
    let snapshot = inspector.tree_snapshot();
    let by_parent: std::collections::HashMap<Option<u64>, Vec<&TreeNodeInfo>> = snapshot
        .nodes
        .iter()
        .fold(std::collections::HashMap::new(), |mut acc, node| {
            acc.entry(node.parent).or_default().push(node);
            acc
        });

    // The ancestors of the selection, by id. Walked up from the selected node
    // rather than down from the root: the tree can be deep and only one path
    // matters.
    let mut ancestors: std::collections::HashSet<u64> = std::collections::HashSet::new();
    if let Some(key) = selected {
        let mut cursor = snapshot
            .nodes
            .iter()
            .find(|node| node.key.as_deref() == Some(key));
        while let Some(node) = cursor {
            ancestors.insert(node.id);
            cursor = node
                .parent
                .and_then(|parent| snapshot.nodes.iter().find(|n| n.id == parent));
        }
    }

    let mut rows = Vec::new();
    // The root's own row first: a tree that starts at the root's children hides
    // the one node every other node hangs from.
    let Some(root) = snapshot.nodes.iter().find(|n| n.id == snapshot.root) else {
        return rows;
    };
    rows.push(tree_row(
        root, &by_parent, selected, collapsed, &ancestors, 0,
    ));
    // Depth-first from the root's children, so the order a browser's inspector
    // shows. The stack carries a `show` flag instead of consulting the collapsed
    // set when it pops: a collapsed ancestor has to hide its whole subtree, and a
    // node deeper down has no way to know it was reached through one.
    let mut stack: Vec<(&TreeNodeInfo, usize, bool)> = vec![(root, 1, true)];
    while let Some((parent, depth, show)) = stack.pop() {
        let Some(children) = by_parent.get(&Some(parent.id)) else {
            continue;
        };
        for node in children.iter().rev() {
            if show {
                rows.push(tree_row(
                    node, &by_parent, selected, collapsed, &ancestors, depth,
                ));
            }
            let key = node.key.as_deref().unwrap_or("");
            let open = show && (!collapsed.contains(key) || ancestors.contains(&node.id));
            stack.push((node, depth + 1, open));
        }
    }
    rows
}

/// One tree row: the node's key, and the node's kind.
///
/// `depth` is the row's only indent. A row that stands for a node carries a
/// [`RowNode`], which is what makes the panel build it as a clickable
/// [`crate::tree::TreeRow`] rather than a line of text.
fn tree_row(
    node: &TreeNodeInfo,
    by_parent: &std::collections::HashMap<Option<u64>, Vec<&TreeNodeInfo>>,
    selected: Option<&str>,
    collapsed: &BTreeSet<String>,
    ancestors: &std::collections::HashSet<u64>,
    depth: usize,
) -> ReadoutRow {
    let key = node.key.as_deref().unwrap_or("?");
    let has_children = by_parent
        .get(&Some(node.id))
        .is_some_and(|children| !children.is_empty());
    let expanded = !collapsed.contains(key) || ancestors.contains(&node.id);
    ReadoutRow {
        label: key.to_string(),
        value: node.kind.clone(),
        depth,
        heading: false,
        node: Some(RowNode {
            key: key.to_string(),
            has_children,
            expanded,
            selected: selected == Some(key),
        }),
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
        return vec![
            ReadoutRow::heading("Breadcrumb"),
            ReadoutRow::new("(nothing selected)", ""),
        ];
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
        return vec![
            ReadoutRow::heading("Styles"),
            ReadoutRow::new("(nothing selected)", ""),
        ];
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
        return vec![
            ReadoutRow::heading("Layout"),
            ReadoutRow::new("(nothing selected)", ""),
        ];
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
