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
    DevtoolsPanel, DevtoolsShortcut, DevtoolsState, DevtoolsTab, ReadoutRow, console_rows,
    default_shortcut, elements_readout, layout_rows, performance_rows, semantics_rows, style_rows,
    tree_rows, update_rows,
};
pub use floom_devtools::pick::{
    HighlightBox, PickPhase, PickShortcut, PickState, default_pick_shortcut, highlight_is_live,
    highlight_rect, pick_pointer,
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
    /// The overlay the panel is mounted in, so its visibility can reach the
    /// overlay stack rather than only the widget.
    ///
    /// A hidden panel still has bounds — a full-height drawer down the trailing
    /// edge — and the stack hit-tests against bounds. Without this the panel
    /// would eat every press down that column from the first frame, hidden or
    /// not, and the app under it would be unclickable on its right side.
    overlay: floom_types::OverlayHandle,
    /// What had the keyboard before the panel opened.
    ///
    /// Opening a non-modal overlay moves focus into it (`open_overlay` promises
    /// that), but a devtools panel is chrome: it must not take the keyboard from
    /// the app at startup. So the focus it displaced is remembered here and
    /// handed back as soon as the panel is mounted.
    previous_focus: Option<floom_types::ElementId>,
    /// The visibility the overlay was last told about, so [`Devtools::sync`] only
    /// touches the host when something actually changed. `None` until the first
    /// sync, which is what makes the first one always apply.
    synced_visible: Option<bool>,
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

    /// Handles a key event for the shortcut and for Escape.
    ///
    /// Returns whether the event was consumed; the caller should stop
    /// propagating a consumed event. This is the door the app's own event loop
    /// uses — `attach` cannot hook the loop itself, because the loop belongs to
    /// the platform backend rather than to `floom-app`.
    ///
    /// Escape closes the panel only while it is up, and only from here rather
    /// than from the widget: the widget's own `event` sees a key only when the
    /// keyboard was routed to the panel's overlay root, which is true exactly
    /// while the panel holds focus — and the panel deliberately gives focus back
    /// to the app, so a user who clicked the app first would find Escape did
    /// nothing. Handling it here means the panel closes whether or not it has
    /// the keyboard, which is what a drawer is expected to do.
    pub fn handle_key(&mut self, event: &floom_types::KeyEvent) -> bool {
        // Only the shortcut predicate runs here. The panel that is actually
        // mounted owns the flip, because it is the widget the host will ask for
        // `children()` — building a second panel here would flip the shared
        // state and then throw away the only object that could have told the
        // host its children changed. The mounted panel notices the flip in its
        // own `update` and marks itself dirty there.
        if (self.shortcut)(event) {
            if let Ok(mut state) = self.state.lock() {
                state.toggle();
            }
            return true;
        }
        if !self.visible() {
            return false;
        }
        if event.phase == floom_types::KeyPhase::Down
            && event.key == floom_types::Key::Named(floom_types::NamedKey::Escape)
        {
            self.hide();
            return true;
        }
        false
    }

    /// The panel's shortcut, so a caller can test it or log it.
    pub fn shortcut(&self) -> &DevtoolsShortcut {
        &self.shortcut
    }

    /// The overlay the panel is mounted in.
    ///
    /// Exposed because the panel is chrome rather than content: an app that
    /// wants to restyle, resize or re-raise it needs the handle the host uses,
    /// and deriving it by counting overlays is a guess that breaks the first
    /// time anything else opens one.
    pub fn overlay(&self) -> floom_types::OverlayHandle {
        self.overlay
    }

    /// The shared state cell, so a picker can select the element it picked.
    ///
    /// The panel reads this cell every frame; writing to it from the picker's
    /// commit is what makes the pick show up in the readout.
    pub fn state(&self) -> &Arc<Mutex<DevtoolsState>> {
        &self.state
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
    // No tab bar. The caller's readout decides what a tab means, and a bar over
    // a readout that ignores `state.tab()` would be a row of buttons that
    // switch to nothing. A caller whose readout *does* answer several tabs
    // passes them with [`attach_with_tabs`].
    attach_with(host, readout, &[])
}

/// [`attach`] with a tab bar, for a caller whose readout answers several tabs.
///
/// The readout must branch on [`DevtoolsState::tab`] — the bar only flips that
/// value; it cannot know which of its rows belong to which tab. `tabs` is what
/// appears on the bar, in order, so a caller that fills two readouts passes two
/// and does not get three dead buttons.
pub fn attach_with_tabs<F>(
    host: &mut WidgetHost,
    readout: F,
    tabs: impl IntoIterator<Item = DevtoolsTab>,
) -> Devtools
where
    F: Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
{
    let tabs: Vec<DevtoolsTab> = tabs.into_iter().collect();
    attach_with(host, readout, &tabs)
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
    }, &DevtoolsTab::ELEMENTS);
    devtools.with_tree_cell(shared)
}

/// [`attach_elements`], with the caller's own rows on top.
///
/// The composition `run_desktop_with_devtools` uses: the caller's rows first —
/// what the app knows about itself — then the engine's tree, breadcrumb and
/// whichever readout the tab asks for. It exists as its own entry because the
/// two halves have different owners: the app owns the first, the engine owns the
/// second, and neither should have to know about the other.
pub fn attach_with_rows<F>(host: &mut WidgetHost, extra: F) -> Devtools
where
    F: Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
{
    let shared = std::sync::Arc::new(std::sync::Mutex::new(host.tree_nodes()));
    let readout_shared = shared.clone();
    let devtools = attach_with(host, move |state| {
        let mut rows = extra(state);
        let nodes = readout_shared.lock().map(|n| n.clone()).unwrap_or_default();
        let live = describe(&nodes);
        let inspector = floom_devtools::Inspector::new(&live);
        rows.extend(elements_readout(&inspector, state));
        rows
    }, &DevtoolsTab::ELEMENTS);
    devtools.with_tree_cell(shared)
}

fn attach_with<F>(host: &mut WidgetHost, readout: F, tabs: &[DevtoolsTab]) -> Devtools
where
    F: Fn(&DevtoolsState) -> Vec<ReadoutRow> + Send + Sync + 'static,
{
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let panel = DevtoolsPanel::new(state.clone(), readout).with_tabs(tabs.iter().copied());
    let viewport = host.size();
    let bounds = floom_devtools::panel::panel_bounds(Rect::from_origin_size(
        floom_types::Point::new(0.0, 0.0),
        viewport,
    ));
    // Opening a non-modal overlay moves focus into it. That is right for a
    // dialog and wrong for chrome, so the focus it displaced is remembered and
    // put back immediately: the app must be able to type from the first frame.
    let previous_focus = host.focused();
    let overlay = host.open_overlay(OverlayKind::Drawer, false, bounds, panel);
    match previous_focus {
        Some(focused) => host.set_focus(focused),
        None => host.clear_focus(),
    }
    Devtools {
        state,
        shortcut: default_shortcut(),
        tree_cell: None,
        overlay,
        previous_focus,
        // A panel starts hidden, and its first sync has to reach the stack —
        // otherwise the bounds of a drawer that paints nothing are still a
        // claim on every press in its column.
        synced_visible: None,
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

    /// Pushes the panel's visibility into the host's overlay stack.
    ///
    /// Two things have to move when the panel opens or closes, and neither of
    /// them is the widget: whether the overlay answers the pointer, and where
    /// the keyboard is.
    ///
    /// - **Hidden must be unclickable.** The drawer's bounds span the full
    ///   height of the trailing edge, and the stack hit-tests against bounds, so
    ///   a hidden panel that stayed interactive would eat every press down that
    ///   column. It is not closed — closing it would throw away the only widget
    ///   that can rebuild the panel — so its hit region is turned off instead.
    /// - **Closing must hand the keyboard back.** The panel is chrome. While it
    ///   is up it may hold focus (a tab was clicked, or the user tabbed into
    ///   it); once it is down, focus that still names a node inside it is
    ///   stranded, because a hidden panel has no children and the key router
    ///   drops every event aimed at a node with no children to route it to.
    ///
    /// Only visibility is compared, because that is the only thing that changes
    /// what the host has to do. Tab and selection changes are the panel's own
    /// business — its `update` sees them and marks itself dirty.
    ///
    /// Call once per frame from whatever owns the loop; `run_desktop_with_devtools`
    /// does. An embedder that drives its own loop calls it after
    /// [`Devtools::handle_key`], in the same place it calls
    /// [`Devtools::refresh_tree`].
    pub fn sync(&mut self, host: &mut WidgetHost) {
        let visible = self.visible();
        if self.synced_visible == Some(visible) {
            return;
        }
        self.synced_visible = Some(visible);
        // A panel that is not dismissible is one whose close would strand the
        // content: the host owns it and there is no opener to hand it back to.
        host.set_overlay_dismissible(self.overlay, false);
        host.set_overlay_hit(self.overlay, visible);
        if !visible {
            // Focus that landed inside the panel is stranded by hiding it.
            // Focus that is somewhere else — the app's own field, usually — is
            // left exactly where it is: the panel must not steal the keyboard
            // on the way out any more than on the way in.
            let inside = host
                .focused()
                .and_then(|focused| host.node_for(focused))
                .is_some_and(|node| host.overlay_contains(self.overlay, node));
            if inside {
                match self.previous_focus {
                    Some(previous) => host.set_focus(previous),
                    None => host.clear_focus(),
                }
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
    // visible over everything including the devtools. It is *passthrough*: it
    // covers the window so it can outline anything in it, and a covering overlay
    // that also took clicks would make the app unclickable while the box was up
    // — including unclickable for the pick it exists to serve.
    let overlay = host.open_overlay_passthrough(
        OverlayKind::Popup,
        bounds,
        HighlightBox::new(None, false),
    );
    PickHandle {
        pick,
        overlay,
        panel_state: panel_state.clone(),
        shortcut: default_pick_shortcut(),
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
    /// The key chord that arms the picker.
    shortcut: PickShortcut,
}

impl PickHandle {
    /// Arms the picker: the next click selects.
    pub fn arm(&self) {
        if let Ok(mut pick) = self.pick.lock() {
            pick.arm();
        }
    }

    /// Handles a key event for the pick shortcut.
    ///
    /// Returns whether the event was consumed. The caller should stop
    /// propagating a consumed event — the same rule the panel's shortcut
    /// follows, and for the same reason: `Ctrl+Shift+C` in a text field must
    /// arm the picker rather than type.
    pub fn handle_key(&self, event: &floom_types::KeyEvent) -> bool {
        if !(self.shortcut)(event) {
            return false;
        }
        self.arm();
        true
    }

    /// The picker's shortcut, so a caller can test it or log it.
    pub fn shortcut(&self) -> &PickShortcut {
        &self.shortcut
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

    /// Selects the committed pick in the panel, if a click has committed one.
    ///
    /// Returns whether a selection was written. The picker holds the panel's
    /// state cell, so a caller that already owns a `PickHandle` does not need a
    /// second handle on the panel to make a pick show up in the readout.
    ///
    /// **Once per pick.** A pick outlives the click — it is what the highlight
    /// keeps sitting on — so it is not cleared on commit, and a caller that
    /// commits on every pointer event (which is what a frame loop with no
    /// cheaper signal does) would re-select the last picked element on every
    /// later click, overwriting whatever the user selected since. The second
    /// call after a pick returns `false` and writes nothing.
    pub fn commit_selection(&self) -> bool {
        let Ok(mut pick) = self.pick.lock() else {
            return false;
        };
        let Some(key) = pick.take_uncommitted() else {
            return false;
        };
        let Ok(mut state) = self.panel_state.lock() else {
            return false;
        };
        state.select(key);
        true
    }

    /// The element the highlight should be drawn around, if any.
    ///
    /// A hover wins while picking — the box follows the pointer — and the
    /// committed pick is what it falls back to once picking stops. The caller
    /// compares this across frames to know whether the box needs retargeting,
    /// which is what lets a frame be requested only when it actually moved.
    pub fn highlight_target(&self) -> Option<String> {
        let pick = self.pick.lock().ok()?;
        match pick.phase() {
            PickPhase::Hovering => pick
                .hovered()
                .or_else(|| pick.picked())
                .map(String::from),
            _ => pick.picked().map(String::from),
        }
    }

    /// Whether the highlight is following a hover rather than sitting on a pick.
    pub fn highlight_is_live(&self) -> bool {
        highlight_is_live(&self.pick)
    }

    /// Points the highlight box at the picked or hovered element.
    ///
    /// The box follows the pointer, so it has to be retargeted rather than
    /// rebuilt. Called once per frame while the panel is up; a frame with no
    /// picked or hovered element clears the box, so a highlight that stops
    /// following does not stay painted on the last element forever.
    ///
    /// The host resolves both halves — the key to a live node, the node to its
    /// box — because it owns the tree and the layout. Taking a closure for that
    /// would hand the caller a borrow of the same host this method needs
    /// mutably, which is why the earlier signature could not be used from the
    /// frame loop at all.
    pub fn sync_highlight(&self, host: &mut WidgetHost) {
        let rect = self.highlight_rect(|key| {
            let node = host.node_by_key(key)?;
            Some(host.bounds(node))
        });
        let live = self.highlight_is_live();
        let Some(node) = host.overlay_node_id(self.overlay) else {
            return;
        };
        if let Some(box_widget) = host.widget_mut::<HighlightBox>(node) {
            box_widget.set_rect(rect, live);
        }
    }

    /// Closes the highlight overlay.
    pub fn close_highlight(&self, host: &mut WidgetHost) {
        host.close_overlay(self.overlay);
    }

    /// The overlay the highlight box lives in.
    ///
    /// The handle is not needed to *use* the picker — [`Self::sync_highlight`]
    /// retargets the box without it — but it is how a caller checks what got
    /// mounted: the box's widget is reachable from the overlay's node, and the
    /// overlay's stacking is what makes the box visible over the panel.
    pub fn overlay(&self) -> floom_types::OverlayHandle {
        self.overlay
    }

    /// The box the highlight should draw, from the host's layout.
    pub fn highlight_rect<F>(&self, bounds_of: F) -> Option<Rect>
    where
        F: FnOnce(&str) -> Option<Rect>,
    {
        highlight_rect(&self.pick, bounds_of)
    }
}
