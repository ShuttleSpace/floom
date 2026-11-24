// SPDX-License-Identifier: MIT
//! macOS side of the desktop accessibility bridge.
//!
//! Hands the retained semantics frame to `NSAccessibility` through
//! `accesskit_macos`. The action handler runs on the main thread inside
//! AppKit's call, but the widget tree lives one layer up, so actions are queued
//! and drained on the next poll — the same shape the Android side uses for its
//! `AccessibilityNodeProvider`.

use accesskit::{ActionHandler, ActionRequest};

use super::ActionQueue;

/// Queues AppKit's actions for the next poll.
struct Queue(ActionQueue);

impl ActionHandler for Queue {
    fn do_action(&mut self, request: ActionRequest) {
        if let Ok(mut queue) = self.0.lock() {
            queue.push(request);
        }
    }
}

/// One window's `NSAccessibility` adapter.
pub struct MacBridge {
    adapter: accesskit_macos::Adapter,
}

impl MacBridge {
    /// Build an adapter over `view`, an `NSView` the caller owns.
    ///
    /// Must be called on the main thread. The view is the window's content
    /// view; AppKit reaches the accessibility tree through it.
    pub fn attach(view: *mut std::ffi::c_void, actions: ActionQueue) -> Self {
        // SAFETY: the caller passes a live `NSView` from winit's window handle,
        // on the main thread, and keeps the window alive while this adapter
        // exists.
        let adapter = unsafe { accesskit_macos::Adapter::new(view, true, Queue(actions)) };
        Self { adapter }
    }

    /// Push `update` if AppKit is already reading the tree.
    pub fn push(&mut self, update: accesskit::TreeUpdate) {
        if let Some(events) = self.adapter.update_if_active(|| update) {
            events.raise();
        }
    }
}

/// The `NSView` a winit window draws into.
pub fn ns_view(window: &dyn winit::window::Window) -> Option<*mut std::ffi::c_void> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(appkit) => Some(appkit.ns_view.as_ptr()),
        _ => None,
    }
}
