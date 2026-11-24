// SPDX-License-Identifier: MIT
//! The Windows accessibility bridge: the tree in front of UI Automation.
//!
//! Same shape as the macOS one on purpose -- an adapter that converts, a queue
//! for the OS's actions, and a push on change -- so the two can be read side by
//! side. What differs is only what each OS needs: UIA wants an `HWND` and is
//! initialised on the thread that owns the window.

use std::sync::Arc;

use accesskit::{ActionHandler, TreeUpdate};

use super::ActionQueue;

/// Actions from the OS, parked until the frame loop drains them.
///
/// UI Automation calls arrive on whichever thread it likes; a handler that ran
/// the tree inline would mutate widgets from the wrong thread.
struct Queue(ActionQueue);

impl ActionHandler for Queue {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        if let Ok(mut queue) = self.0.lock() {
            queue.push(request);
        }
    }
}

pub struct WindowsBridge {
    adapter: accesskit_windows::Adapter,
}

impl std::fmt::Debug for WindowsBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WindowsBridge")
    }
}

impl WindowsBridge {
    /// Attach to a window that owns its thread.
    ///
    /// # Safety
    ///
    /// `hwnd` must belong to a live window; the adapter keeps it.
    pub unsafe fn attach(hwnd: *mut core::ffi::c_void, actions: ActionQueue) -> Self {
        Self {
            adapter: accesskit_windows::Adapter::new(
                accesskit_windows::HWND(hwnd),
                true,
                Queue(actions),
            ),
        }
    }

    /// Hand the tree over, if UI Automation has asked for it.
    ///
    /// Nothing is pushed before the first client arrives: `update_if_active`
    /// returning `None` is the normal state of a window nobody is reading.
    pub fn push(&mut self, update: TreeUpdate) {
        if let Some(events) = self.adapter.update_if_active(|| update) {
            events.raise();
        }
    }
}

/// The window's `HWND`, if this window has one yet.
pub fn hwnd(window: &dyn winit::window::Window) -> Option<*mut core::ffi::c_void> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut core::ffi::c_void),
        _ => None,
    }
}

/// Wire a bridge to a window, or say why not.
pub fn attach(window: &dyn winit::window::Window, actions: ActionQueue) -> Option<WindowsBridge> {
    let hwnd = hwnd(window)?;
    // SAFETY: the handle came from a window that is alive for as long as the
    // platform holds it, and the bridge is dropped with the window.
    Some(unsafe { WindowsBridge::attach(hwnd, Arc::clone(&actions)) })
}
