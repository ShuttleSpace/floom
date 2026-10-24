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

pub use floom_devtools::panel::{
    DevtoolsPanel, DevtoolsShortcut, DevtoolsState, DevtoolsTab, ReadoutRow, default_shortcut,
};

/// A mounted devtools panel, and the state that opens and closes it.
///
/// This is what the one-line integration hands back. Keeping it is optional:
/// the shortcut works whether or not anyone holds the handle, because the state
/// lives in an `Arc` the panel reads every frame. The handle is for the callers
/// that want to open the panel from code.
pub struct Devtools {
    state: Arc<Mutex<DevtoolsState>>,
    shortcut: DevtoolsShortcut,
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
        let mut panel = DevtoolsPanel::new(
            self.state.clone(),
            |_| Vec::<ReadoutRow>::new(),
        );
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
    }
}
