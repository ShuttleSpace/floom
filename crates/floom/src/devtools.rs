//! The devtools door: one line in, a keystroke away.
//!
//! ```ignore
//! let devtools = floom::devtools::attach(&mut host);   // once, at startup
//! // ... then press F12 at any time.
//! ```
//!
//! `attach` mounts the panel as an overlay above the app's own tree and hands
//! back the handle the app keeps: the same handle toggles the panel
//! programmatically (`show` / `hide`), which is what a "devtools" menu item, a
//! `?debug=1` query parameter, or a test that wants the panel open from the
//! start needs.
//!
//! ## Why the panel is an overlay, not part of the tree
//!
//! Because it is not the app's. It sits above everything the app drew, escapes
//! the clip of whatever container it was mounted next to, and takes the
//! keyboard while it is up — which is the whole point: a panel clipped by its
//! parent, or unable to answer a keystroke, is a panel you have to look at but
//! cannot use.
//!
//! ## The shortcut
//!
//! F12 everywhere, plus `Ctrl+Shift+I` where F12 belongs to the OS. See
//! [`default_shortcut`]. Escape closes the panel once it is up.
//!
//! [`default_shortcut`]: floom_devtools::panel::default_shortcut

use std::sync::{Arc, Mutex};

use floom_app::WidgetHost;
use floom_types::{OverlayKind, Rect};

pub use floom_devtools::Inspector;
pub use floom_devtools::panel::{
    DevtoolsPanel, DevtoolsShortcut, DevtoolsState, DevtoolsTab, ReadoutRow, default_shortcut,
    elements_readout,
};
pub use floom_devtools::pick::{
    HighlightBox, PickPhase, PickState, highlight_is_live, highlight_rect, pick_pointer,
};
pub use floom_hot_reload::describe;

/// A mounted devtools panel, and the state that opens and closes it.
///
/// This is what the one-line integration hands back. Keeping it is optional:
/// the shortcut works whether or not anyone holds the handle, because the state
/// lives in an `Arc` the panel reads every frame. The handle is for the callers
/// that want to open the panel from code.
pub struct Devtools {
    state: Arc<Mutex<DevtoolsState>>,
    shortcut: DevtoolsShortcut,
    /// The cell the composed readout reads, when the caller used
    /// [`attach_elements`]. `None` for a caller that supplied its own readout.
    tree_cell: Option<Arc<Mutex<Vec<floom_app::HostedNode>>>>,
}

impl Devtools {
    /// Shows the panel — the `show: true` of the integration.
    pub fn show(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.show();
        }
    }

    /// Hides the panel — the `show: false` of the integration.
    pub fn hide(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.hide();
        }
    }

    /// Flips the panel, the way the shortcut does.
    pub fn toggle(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.toggle();
        }
    }

    /// Whether the panel is showing.
    pub fn visible(&self) -> bool {
        self.state.lock().map(|s| s.visible()).unwrap_or(false)
    }

    /// Which readout is showing.
    pub fn tab(&self) -> DevtoolsTab {
        self.state.lock().map(|s| s.tab()).unwrap_or_default()
    }

    /// Selects a readout.
    pub fn set_tab(&self, tab: DevtoolsTab) {
        if let Ok(mut state) = self.state.lock() {
            state.set_tab(tab);
        }
    }

    /// Handles a key event for the shortcut.
    ///
    /// Returns whether the event was consumed; the caller should stop
    /// propagating a consumed event. This is the door the app's own event loop
    /// uses — `attach` cannot hook the loop itself, because the loop belongs to
    /// the platform backend rather than to `floom-app`.
    pub fn handle_key(&mut self, event: &floom_types::KeyEvent) -> bool {
        let mut panel = DevtoolsPanel::new(self.state.clone(), |_| Vec::<ReadoutRow>::new());
        panel.handle_key(event, &self.shortcut)
    }

    /// The panel's shortcut, so a caller can test it or log it.
    pub fn shortcut(&self) -> &DevtoolsShortcut {
        &self.shortcut
    }
}

/// Mounts the devtools panel above the app's tree. The one-line integration.
///
/// Call it once, after the app has mounted its own root: the panel docks to the
/// window's trailing edge and starts hidden, so an app that never presses F12
/// pays for one element and nothing else.
///
/// The `readout` closure builds the rows the panel shows, from the app's own
/// state — typically the inspector over the live tree the host retains. Pass
/// `|_| Vec::new()` for a panel that opens with the wiring proven and nothing
/// in it yet.
pub fn attach<F>(host: &mut WidgetHost, readout: F) -> Devtools
where
    F: Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
{
    attach_with(host, readout)
}

/// [`attach`], with the tree/style/layout readout already composed in.
///
/// The Chrome layout: the element tree, the breadcrumb to the selected node, and
/// that node's style or layout depending on the tab. The caller supplies the
/// host, and the panel reads it every frame — so what it shows is the tree as it
/// is, not a copy taken at startup.
pub fn attach_elements(host: &mut WidgetHost) -> Devtools {
    // The panel outlives any one frame's borrow of the host, and the host
    // outlives the panel, so the tree travels through a shared cell the caller
    // refreshes — `Devtools::refresh_tree`, once per frame. A panel reading a
    // snapshot taken at startup would show the tree as it was, which is worse
    // than showing nothing.
    let shared = std::sync::Arc::new(std::sync::Mutex::new(host.tree_nodes()));
    let readout_shared = shared.clone();
    let devtools = attach_with(host, move |state| {
        let nodes = readout_shared.lock().map(|n| n.clone()).unwrap_or_default();
        let live = describe(&nodes);
        let inspector = floom_devtools::Inspector::new(&live);
        elements_readout(&inspector, state)
    });
    devtools.with_tree_cell(shared)
}

fn attach_with<F>(host: &mut WidgetHost, readout: F) -> Devtools
where
    F: Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
{
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let panel = DevtoolsPanel::new(state.clone(), readout);
    let viewport = host.size();
    let bounds = floom_devtools::panel::panel_bounds(Rect::from_origin_size(
        floom_types::Point::new(0.0, 0.0),
        viewport,
    ));
    host.open_overlay(OverlayKind::Drawer, false, bounds, panel);
    Devtools {
        state,
        shortcut: default_shortcut(),
        tree_cell: None,
    }
}

impl Devtools {
    /// Attaches the cell the composed readout reads. Internal: `attach_elements`
    /// is the only caller, and it is the only thing that knows the cell exists.
    fn with_tree_cell(mut self, cell: Arc<Mutex<Vec<floom_app::HostedNode>>>) -> Self {
        self.tree_cell = Some(cell);
        self
    }

    /// Re-reads the host's tree into the cell the composed readout reads.
    ///
    /// Call once per frame while the panel is up. Without it the panel shows the
    /// tree as it was when `attach_elements` ran — which is a stale answer, and
    /// a stale answer is worse than none because it looks current.
    pub fn refresh_tree(&self, host: &WidgetHost) {
        if let Some(cell) = &self.tree_cell {
            if let Ok(mut nodes) = cell.lock() {
                *nodes = host.tree_nodes();
            }
        }
    }
}

/// Mounts the pick: Ctrl+Shift+C arms it, and the next click selects whatever
/// is under the pointer.
///
/// The second half of Chrome's inspector. `attach` gives the panel somewhere to
/// show; this gives the pointer somewhere to point. A click while picking
/// selects the element *without* pressing it, because the picker consumes the
/// event before the tree sees it.
///
/// The highlight is mounted as one more overlay, above the panel, so the box is
/// visible over everything including the devtools itself.
pub fn attach_picker(host: &mut WidgetHost, panel_state: &Arc<Mutex<DevtoolsState>>) -> PickHandle {
    let pick = Arc::new(Mutex::new(PickState::new()));
    let viewport = host.size();
    let bounds = Rect::from_origin_size(floom_types::Point::new(0.0, 0.0), viewport);
    // The highlight is an overlay of its own, above the panel, so the box is
    // visible over everything including the devtools.
    let overlay = host.open_overlay(
        OverlayKind::Popup,
        false,
        bounds,
        HighlightBox::new(None, false),
    );
    PickHandle {
        pick,
        overlay,
        panel_state: panel_state.clone(),
    }
}

/// What [`attach_picker`] hands back: the picker's state and the panel's.
///
/// The caller keeps it so its event loop can arm the picker and retarget the
/// highlight as the pointer moves.
pub struct PickHandle {
    pick: Arc<Mutex<PickState>>,
    /// The overlay the highlight box lives in, so it can be retargeted.
    overlay: floom_types::OverlayHandle,
    panel_state: Arc<Mutex<DevtoolsState>>,
}

impl PickHandle {
    /// Arms the picker: the next click selects.
    pub fn arm(&self) {
        if let Ok(mut pick) = self.pick.lock() {
            pick.arm();
        }
    }

    /// Whether the picker is waiting for a click.
    pub fn armed(&self) -> bool {
        self.pick.lock().map(|p| p.armed()).unwrap_or(false)
    }

    /// Handles a pointer event while picking.
    ///
    /// `node_key` maps the point to the live-tree key under it; the host owns
    /// that mapping, because it owns the tree. Returns whether the event was
    /// consumed — a consumed one should be dropped, so the app never sees a
    /// click that was meant to inspect rather than to press.
    pub fn pointer<F>(&self, event: &floom_types::PointerEvent, node_key: F) -> bool
    where
        F: FnOnce(floom_types::Point) -> Option<String>,
    {
        pick_pointer(&self.pick, event, node_key)
    }

    /// The element a click committed to, if any.
    pub fn picked(&self) -> Option<String> {
        self.pick
            .lock()
            .ok()
            .and_then(|p| p.picked().map(String::from))
    }

    /// Whether the highlight is following a hover rather than sitting on a pick.
    pub fn highlight_is_live(&self) -> bool {
        highlight_is_live(&self.pick)
    }

    /// Points the highlight box at the picked or hovered element.
    ///
    /// `bounds_of` maps a live-tree key to the node's box; the host owns that
    /// mapping, because it owns the layout. Called once per frame while the
    /// panel is up — the box follows the pointer, so it has to be retargeted
    /// rather than rebuilt.
    pub fn sync_highlight<F>(&self, host: &mut WidgetHost, bounds_of: F)
    where
        F: FnOnce(&str) -> Option<Rect>,
    {
        let Some(key) = self
            .pick
            .lock()
            .ok()
            .and_then(|p| p.picked().or_else(|| p.hovered()).map(str::to_string))
        else {
            return;
        };
        let Some(rect) = bounds_of(&key) else {
            return;
        };
        let live = self.highlight_is_live();
        let Some(node) = host.overlay_node_id(self.overlay) else {
            return;
        };
        if let Some(box_widget) = host.widget_mut::<HighlightBox>(node) {
            box_widget.set_rect(Some(rect), live);
        }
    }

    /// Closes the highlight overlay.
    pub fn close_highlight(&self, host: &mut WidgetHost) {
        host.close_overlay(self.overlay);
    }

    /// The box the highlight should draw, from the host's layout.
    pub fn highlight_rect<F>(&self, bounds_of: F) -> Option<Rect>
    where
        F: FnOnce(&str) -> Option<Rect>,
    {
        highlight_rect(&self.pick, bounds_of)
    }
}
