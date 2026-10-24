//! A tab bar plus the panel for the active tab.
//!
//! A selectable strip and the content it selects. Which indicator, how tall the strip and what the inactive label looks like are three kits' answers to the same question.
//!
//! It lived in `floom-widgets/src/compose.rs` beside the layout vocabulary
//! (`Stack`, `List`, `Grid`, `Conditional`, `Crossfade`). The two are not the
//! same kind of thing: a `Stack` arranges children along an axis, which is
//! arithmetic; this decides *what a control looks like*, which is a product
//! decision. So it moved up a layer with the tree, the counter and the select,
//! and the dependency runs one way -- this crate composes the engine's
//! vocabulary rather than the other way round.

use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, PaintContext,
    SemanticsContext, UpdateContext, Widget,
};

use floom_widgets::primitives::SemanticsHint;
use floom_widgets::{Button, Stack};

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
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}
