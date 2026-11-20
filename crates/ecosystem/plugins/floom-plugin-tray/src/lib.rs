//! A tray icon with a menu: the app's presence in the system tray or menu bar.
//!
//! This is `tray_manager`'s question — an app that keeps running with its
//! windows closed still needs a way to be found — and the batch recorded in
//! `docs/plugins.md` §10.
//!
//! [`Tray::show`] puts an icon in the tray and a menu behind it;
//! [`Tray::events`] drains what the user did; dropping the [`Tray`] removes the
//! icon. The icon carries a text label rather than an image: a status item's
//! title is the one thing every arm can render without shipping assets, and
//! guessing at a per-platform image format is how an API becomes three APIs.
//!
//! # Why this is a plugin
//!
//! A tray needs no host handle to exist. macOS creates its own `NSStatusItem`,
//! Windows its own message-only window, Linux its own D-Bus object — none of
//! them asks the app for a window, a surface or a frame. That is the whole
//! difference from window control, which had to go into the platform layer
//! because the window belongs to the host. So this crate depends on nothing
//! above L4 and an app that does not name it links nothing.
//!
//! # What each arm needs from the app
//!
//! The tray does not run a loop; the app already has one, and this crate hands
//! its events to it:
//!
//! - **macOS** — the status item and its menu are created on the main thread,
//!   and `NSMenu` runs its own tracking loop while the menu is open. Call
//!   [`Tray::show`] from the main thread; call [`Tray::events`] from whatever
//!   runs after input.
//! - **Windows** — `Shell_NotifyIconW` posts its callback to a message-only
//!   window this crate creates. That window's messages are dispatched by
//!   whatever pumps the thread's message queue (a winit event loop does), so
//!   [`Tray::events`] only drains what the pump already delivered.
//! - **Linux** — `StatusNotifierItem` over D-Bus. zbus runs its own thread, so
//!   events arrive without the app pumping anything; [`Tray::events`] drains
//!   the queue it fills.
//!
//! # What is deliberately not here
//!
//! No images, no submenus, no dynamic menu or label updates, no per-item
//! checkboxes, and no left/right-click distinction on macOS — a status item
//! with a menu opens the menu, because that is what a menu bar icon does. Each
//! of these is a real feature with three shapes; adding one is a decision, not
//! a flag.

use std::sync::{Arc, Mutex};

/// One row of the tray menu.
pub enum MenuItem {
    /// A selectable row. `id` is what [`TrayEvent::Menu`] reports back.
    Item {
        id: u32,
        label: String,
        enabled: bool,
    },
    /// A horizontal rule between rows.
    Separator,
}

impl MenuItem {
    /// A selectable row, enabled.
    pub fn new(id: u32, label: impl Into<String>) -> Self {
        Self::Item {
            id,
            label: label.into(),
            enabled: true,
        }
    }

    /// The same row, greyed out: visible but not selectable. A row that is
    /// shown but cannot be chosen is how a menu says "not now, and here is
    /// where it would be".
    pub fn disabled(mut self) -> Self {
        if let Self::Item { enabled, .. } = &mut self {
            *enabled = false;
        }
        self
    }

    /// A horizontal rule.
    pub fn separator() -> Self {
        Self::Separator
    }
}

/// Something the user did to the tray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// A menu row with this id was chosen.
    Menu(u32),
    /// The icon itself was activated, where the platform has that gesture
    /// separately from opening the menu: a double-click on Windows, the
    /// `Activate` call of a `StatusNotifierItem` on Linux. macOS reports
    /// nothing here — its status item opens the menu instead.
    Activate,
}

/// The id a menu row is allowed to use.
///
/// The bound is Win32's: a command id travels in the low word of
/// `WM_COMMAND`, and `TrackPopupMenu` answers `0` for "the menu was dismissed"
/// — so a row with id 0 could not be told apart from a cancel. Rather than
/// let the Windows arm reinterpret two ids the other arms accept, the shared
/// constructor refuses them.
const MAX_ID: u32 = 0xFFFF;

/// A tray icon and its menu.
///
/// Not `Send` on **any** target, including the one where the platform object
/// would allow it: a tray is created and dropped by the thread that owns the
/// event loop, and a `Send`-only-on-Linux tray would let the same code compile
/// on Linux and fail on the Mac. The marker below is what makes the contract
/// uniform; on macOS and Windows the objects are single-threaded anyway (AppKit
/// main-thread-only, `HWND` tied to its creating thread).
pub struct Tray {
    inner: imp::Tray,
    _not_send: std::marker::PhantomData<*const ()>,
}

impl Tray {
    /// Put `label` in the tray with `items` as its menu.
    ///
    /// `label` is the text the platform shows: a menu-bar title on macOS, a
    /// tooltip on Windows, the item's `Title` on Linux. An empty menu is
    /// allowed and means "icon only" — on macOS the icon then has no menu, and
    /// clicking it does nothing rather than opening an empty one.
    pub fn show(label: &str, items: &[MenuItem]) -> Result<Self, String> {
        validate(label, items)?;
        imp::Tray::show(label, items).map(|inner| Self {
            inner,
            _not_send: std::marker::PhantomData,
        })
    }

    /// Take the events that arrived since the last call. Never blocks.
    pub fn events(&mut self) -> Vec<TrayEvent> {
        self.inner.events()
    }
}

/// Shared validation, so an id or a label that one arm cannot represent is
/// refused on every arm instead of silently meaning something different.
fn validate(label: &str, items: &[MenuItem]) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err(
            "tray: the label cannot be empty — on macOS it is the status item's \
             whole width, and an empty one is an icon nobody can see"
                .into(),
        );
    }
    let mut seen: Vec<u32> = Vec::new();
    for item in items {
        let MenuItem::Item { id, label, .. } = item else {
            continue;
        };
        if *id == 0 || *id > MAX_ID {
            return Err(format!(
                "tray: menu id {id} is outside 1..={MAX_ID} — 0 is how Win32 \
                 reports a dismissed menu, and the upper bound is where its \
                 command word ends"
            ));
        }
        if label.is_empty() {
            return Err(format!("tray: menu row {id} has no label"));
        }
        if seen.contains(id) {
            return Err(format!(
                "tray: two menu rows share id {id} — a click could not say which"
            ));
        }
        seen.push(*id);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
mod linux;
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
use linux as imp;

#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_env = "ohos")]
use ohos as imp;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod unsupported;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
use unsupported as imp;

/// The queue every arm hands [`Tray::events`]. A `Mutex` rather than a channel
/// because the writers are a menu action, a window procedure and a D-Bus
/// method — none of them async, and all of them on the same machine.
type Queue = Arc<Mutex<Vec<TrayEvent>>>;

fn push(queue: &Queue, event: TrayEvent) {
    if let Ok(mut queue) = queue.lock() {
        queue.push(event);
    }
}

fn drain(queue: &Queue) -> Vec<TrayEvent> {
    match queue.lock() {
        Ok(mut queue) => std::mem::take(&mut *queue),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_label_is_refused() {
        let err = Tray::show("", &[MenuItem::new(1, "Quit")])
            .err()
            .expect("refused");
        assert!(err.contains("label cannot be empty"), "{err}");
    }

    #[test]
    fn id_zero_is_refused_because_win32_uses_it_for_cancel() {
        let err = Tray::show("app", &[MenuItem::new(0, "Quit")])
            .err()
            .expect("refused");
        assert!(err.contains("outside 1..=65535"), "{err}");
    }

    #[test]
    fn an_id_past_the_command_word_is_refused() {
        let err = Tray::show("app", &[MenuItem::new(0x1_0000, "Quit")])
            .err()
            .expect("refused");
        assert!(err.contains("outside 1..=65535"), "{err}");
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let err = Tray::show("app", &[MenuItem::new(7, "Show"), MenuItem::new(7, "Quit")])
            .err()
            .expect("refused");
        assert!(err.contains("share id 7"), "{err}");
    }

    #[test]
    fn a_row_without_a_label_is_refused() {
        let err = Tray::show("app", &[MenuItem::new(3, "")])
            .err()
            .expect("refused");
        assert!(err.contains("row 3 has no label"), "{err}");
    }

    #[test]
    fn a_valid_menu_passes_validation_with_separators_and_disabled_rows() {
        let items = [
            MenuItem::new(1, "Show"),
            MenuItem::separator(),
            MenuItem::new(2, "Quit").disabled(),
        ];
        assert!(validate("app", &items).is_ok());
    }
}
