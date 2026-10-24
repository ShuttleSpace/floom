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
            let text = format!("{}: {}", row.label, row.value);
            stack = stack.push_keyed(index as u64, move || {
                Box::new(Text::new(text.clone()).size(12.0))
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
        ReadoutRow {
            label: "elements".into(),
            value: inspector.tree_snapshot().nodes.len().to_string(),
        },
        ReadoutRow {
            label: "semantics".into(),
            value: inspector.semantics().len().to_string(),
        },
    ]
}

/// The semantics rows a panel can list.
pub fn semantics_rows(entries: &[SemanticsEntry]) -> Vec<ReadoutRow> {
    entries
        .iter()
        .map(|e| ReadoutRow {
            label: format!("{:?}", e.role),
            value: if e.label.is_empty() {
                format!("#{}", e.element.raw())
            } else {
                e.label.clone()
            },
        })
        .collect()
}

/// The rows for one element's update path: the bindings targeting it and the
/// ancestors a layout invalidation reaches.
pub fn update_rows(path: &UpdatePath) -> Vec<ReadoutRow> {
    let mut rows: Vec<ReadoutRow> = path
        .bindings
        .iter()
        .map(|b| ReadoutRow {
            label: "binding".into(),
            value: match &b.source {
                Some(source) => {
                    format!("{} -> {} ({} runs)", source.label, source.property, b.runs)
                }
                None => format!("{:?} ({} runs)", b.id, b.runs),
            },
        })
        .collect();
    for ancestor in &path.ancestors {
        rows.push(ReadoutRow {
            label: "ancestor".into(),
            value: ancestor.as_str().to_string(),
        });
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
