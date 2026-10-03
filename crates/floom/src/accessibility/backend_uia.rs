//! Windows UI Automation backend (via `accesskit_windows`).
//!
//! Bridges Floem's neutral [`AccessTreeUpdate`] to UI Automation. We feed the
//! tree to `accesskit_windows::Adapter`, created from the window `HWND`. The
//! adapter answers `WM_GETOBJECT`; because we cannot use `accesskit_winit`
//! (we depend on a winit fork rather than upstream `winit`), we subclass the
//! window ourselves and forward that message from our window procedure to
//! [`Adapter::handle_wm_getobject`]. Actions from Narrator arrive via the
//! [`accesskit::ActionHandler`] and are forwarded to Floem core through a
//! channel (drained by `WindowState`).

use std::sync::{mpsc, Arc, Mutex};

use accesskit::{
    ActionData, ActionHandler, ActionRequest, ActivationHandler, TreeUpdate,
};
use accesskit_windows::{Adapter, HWND};
use windows_sys::Win32::Foundation::{HWND as Hwnd, LRESULT, LPARAM, WPARAM};

/// Signature of a Win32 window procedure, used to transmute the `usize` we
/// stash as the original procedure back into a callable pointer.
type WndProc = unsafe extern "system" fn(Hwnd, u32, WPARAM, LPARAM) -> LRESULT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWLP_USERDATA, GWLP_WNDPROC,
    WM_GETOBJECT, WM_NCDESTROY,
};

use crate::accessibility::translate::build_tree_update;
use crate::accessibility::{
    AccessAction, AccessNodeId, AccessTree, AccessTreeUpdate, AccessibilityBackend, PendingAction,
};

/// Routes Narrator action requests into the action channel.
struct WindowsActionHandler {
    tx: mpsc::Sender<PendingAction>,
}

impl ActionHandler for WindowsActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        let node = AccessNodeId(request.target_node.0);
        let action = match request.action {
            accesskit::Action::Focus => AccessAction::Focus,
            accesskit::Action::Click => AccessAction::Default,
            accesskit::Action::Increment => AccessAction::Increment,
            accesskit::Action::Decrement => AccessAction::Decrement,
            accesskit::Action::ShowContextMenu => AccessAction::ShowContextMenu,
            accesskit::Action::ScrollIntoView => AccessAction::ScrollIntoView,
            accesskit::Action::Expand => AccessAction::Expand,
            accesskit::Action::Collapse => AccessAction::Collapse,
            accesskit::Action::SetValue => AccessAction::SetValue,
            _ => return,
        };
        let payload = match request.data {
            Some(ActionData::Value(v)) => Some(v.into()),
            _ => None,
        };
        let _ = self.tx.send((node, action, payload));
    }
}

/// Activation handler: provides the initial tree the first time Narrator
/// queries. It reads a cached `TreeUpdate` kept up to date by the backend so
/// it never needs to lock the live [`AccessTree`] — that would risk a
/// lock-order deadlock with [`Adapter::update_if_active`], which the backend
/// calls while holding the tree.
struct WindowsActivationHandler {
    cached: Arc<Mutex<Option<TreeUpdate>>>,
}

impl ActivationHandler for WindowsActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.cached.lock().ok().and_then(|g| g.clone())
    }
}

/// Per-window subclass payload. Lives for the window's lifetime, reachable
/// from the window procedure through `GWLP_USERDATA`.
struct SubclassData {
    /// Original window procedure we must chain to for every other message.
    original_proc: usize,
    /// Shared adapter; also borrowed by [`WindowsAccessibilityBackend`] to push
    /// tree updates and focus changes.
    adapter: Arc<Mutex<Adapter>>,
    /// Activation handler sharing the cached initial tree.
    activation: Arc<Mutex<WindowsActivationHandler>>,
}

/// Our subclass window procedure. Forwards `WM_GETOBJECT` to the accesskit
/// adapter (so Narrator can connect) and otherwise chains to the original
/// procedure. Restores the original procedure and frees [`SubclassData`] on
/// `WM_NCDESTROY`.
///
/// # Safety
/// Standard Win32 subclass: called by the OS on the window's thread. `hwnd`'s
/// `GWLP_USERDATA` holds a valid `Arc<SubclassData>` pointer for the window's
/// lifetime (until `WM_NCDESTROY`).
unsafe extern "system" fn subclass_proc(
    hwnd: Hwnd,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const SubclassData;
    if ptr.is_null() {
        return 0;
    }

    if msg == WM_GETOBJECT {
        let data = unsafe { &*ptr };
        let mut adapter_guard = data.adapter.lock().expect("a11y adapter mutex poisoned");
        let mut activation_guard =
            data.activation.lock().expect("a11y activation mutex poisoned");
        let handled = unsafe {
            adapter_guard.handle_wm_getobject(
                accesskit_windows::WPARAM(wparam as usize),
                accesskit_windows::LPARAM(lparam as isize),
                &mut *activation_guard,
            )
        };
        if let Some(result) = handled {
            let lresult: accesskit_windows::LRESULT = result.into();
            return lresult.0;
        }
        // accesskit did not handle it; fall through to the original proc.
    } else if msg == WM_NCDESTROY {
        // Restore the original window procedure and free our `SubclassData`.
        let original = unsafe { (*ptr).original_proc };
        // Reconstruct the owning `Arc` so it is dropped exactly once.
        let _owned = unsafe { Arc::from_raw(ptr) };
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, original as isize);
        }
        let prev: WndProc = unsafe { std::mem::transmute(original) };
        return unsafe { CallWindowProcW(Some(prev), hwnd, msg, wparam, lparam) };
    }

    let data = unsafe { &*ptr };
    let prev: WndProc = unsafe { std::mem::transmute(data.original_proc) };
    unsafe { CallWindowProcW(Some(prev), hwnd, msg, wparam, lparam) }
}

/// Bridges Floem's neutral accessibility tree to Windows UIA via accesskit.
pub(crate) struct WindowsAccessibilityBackend {
    /// Shared with the subclass window procedure.
    adapter: Arc<Mutex<Adapter>>,
    /// Neutral cache powering `focused_node` / `node_at_point`.
    tree: AccessTree,
    /// Latest tree update, served by the activation handler on first query.
    cached_tree: Arc<Mutex<Option<TreeUpdate>>>,
    /// Receiver side of the action channel (see [`WindowsActionHandler`]).
    action_rx: mpsc::Receiver<PendingAction>,
}

impl WindowsAccessibilityBackend {
    /// Build the UIA adapter from the raw `HWND`, then subclass the window so
    /// `WM_GETOBJECT` reaches the adapter.
    ///
    /// # Safety
    /// `hwnd` must be a valid, live window handle created on the current
    /// thread (we are invoked from `WindowHandle::new`).
    pub(crate) fn new(hwnd: *mut std::ffi::c_void) -> Option<Self> {
        if hwnd.is_null() {
            return None;
        }
        let (tx, rx) = mpsc::channel();
        let hwnd_typed = HWND(hwnd as *mut _);
        let adapter = Arc::new(Mutex::new(Adapter::new(
            hwnd_typed,
            true,
            WindowsActionHandler { tx },
        )));
        let cached_tree: Arc<Mutex<Option<TreeUpdate>>> = Arc::new(Mutex::new(None));
        let activation = Arc::new(Mutex::new(WindowsActivationHandler {
            cached: cached_tree.clone(),
        }));

        // Subclass the window: stash a `SubclassData` in `GWLP_USERDATA` and
        // replace the window procedure with ours.
        let hwnd_sys = hwnd as Hwnd;
        let original = unsafe { GetWindowLongPtrW(hwnd_sys, GWLP_WNDPROC) } as usize;
        let data = Arc::new(SubclassData {
            original_proc: original,
            adapter: adapter.clone(),
            activation: activation.clone(),
        });
        let raw = Arc::into_raw(data) as isize;
        unsafe {
            // Store our payload first, then install the subclass proc so the
            // proc always finds valid data.
            SetWindowLongPtrW(hwnd_sys, GWLP_USERDATA, raw);
            SetWindowLongPtrW(hwnd_sys, GWLP_WNDPROC, subclass_proc as usize as isize);
        }

        Some(Self {
            adapter,
            tree: AccessTree::new(),
            cached_tree,
            action_rx: rx,
        })
    }

    fn push(&mut self) {
        let update = build_tree_update(&self.tree);
        // Cache a clone so the activation handler can serve the first query
        // without locking the adapter.
        if let Ok(mut g) = self.cached_tree.lock() {
            *g = Some(update.clone());
        }
        if let Ok(mut adapter) = self.adapter.lock() {
            if let Some(events) = adapter.update_if_active(move || update) {
                events.raise();
            }
        }
    }
}

impl AccessibilityBackend for WindowsAccessibilityBackend {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        self.tree.apply(update);
        self.push();
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        self.tree.focus
    }

    fn node_at_point(&self, point: peniko::kurbo::Point) -> Option<AccessNodeId> {
        self.tree.node_at_point(point)
    }

    fn poll_actions(&mut self) -> Vec<PendingAction> {
        self.action_rx.try_recv().into_iter().collect()
    }

    fn update_focus_state(&mut self, focused: bool) {
        // Keep Narrator's notion of window focus in sync with Floem's.
        if let Ok(mut adapter) = self.adapter.lock() {
            if let Some(events) = adapter.update_window_focus_state(focused) {
                events.raise();
            }
        }
    }
}
