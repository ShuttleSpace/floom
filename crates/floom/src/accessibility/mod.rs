//! Experimental accessibility (a11y) subsystem.
//!
//! Floom's accessibility model is a **platform-neutral tree** of
//! [`AccessNode`]s. Each node carries a [`AccessRole`], boolean
//! [`AccessState`]s, the [`AccessAction`]s it supports, geometry, and its
//! children. Floom core builds this tree from view/layout state and hands
//! diffable [`AccessTreeUpdate`]s to a platform [`AccessibilityBackend`],
//! which bridges them to the native screen reader:
//!
//! - macOS / iOS: `NSAccessibility` (`NSAccessibilityElement`)
//! - Android: `AccessibilityNodeInfo` / `AccessibilityNodeProvider`
//! - Linux: AT-SPI2 (over D-Bus)
//! - Windows: UI Automation (UIA)
//!
//! This module defines only the neutral protocol and a [`MockAccessibility`]
//! test backend. Real platform bridges are later steps; until a backend is
//! wired in, [`crate::embedder::BackendCapabilities::accessibility`] stays
//! `false`.
//!
//! The design deliberately mirrors the embedder crate (`crates/floom/src/embedder/`):
//! a data-oriented protocol plus Rust adapter traits, independent of any
//! specific platform or windowing library.

pub mod attrs;
pub mod backend;
pub mod bridge;
pub mod node;
pub mod role;
pub mod state;
pub mod tree;

pub use attrs::AccessibilityAttrs;
pub use backend::{AccessibilityBackend, MockAccessibility, PendingAction};
pub use node::{AccessNode, AccessNodeId};
pub use role::AccessRole;
pub use state::{AccessAction, AccessState};
pub use tree::{AccessTree, AccessTreeUpdate};

// ─────────────────────────────────────────────────────────────────────
// Platform backends
//
// Each desktop/mobile platform implements [`AccessibilityBackend`] against its
// native a11y API. They are compiled only for their target so mobile/web
// builds pull no extra dependencies. `native_backend_for_window` returns the
// right backend for the current target (or `None` when no bridge exists yet —
// e.g. wasm, where the tree is still kept in `WindowState::accessibility`).
// ─────────────────────────────────────────────────────────────────────
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) mod translate;

#[cfg(target_os = "macos")]
pub(crate) mod backend_nsaccessibility;
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
pub(crate) mod backend_atspi;
#[cfg(target_os = "windows")]
pub(crate) mod backend_uia;
#[cfg(target_os = "ios")]
pub(crate) mod backend_ios;
#[cfg(target_env = "ohos")]
pub(crate) mod backend_ohos;
#[cfg(target_os = "android")]
pub(crate) mod backend_android;

/// Construct the platform-native accessibility backend for a newly created
/// window, or `None` when no bridge is available on this target.
///
/// On macOS this needs the content-view pointer; on Windows the `HWND`.
#[cfg(target_os = "macos")]
pub(crate) fn native_backend_for_window(
    ns_view: *mut std::ffi::c_void,
) -> Option<std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>>> {
    crate::accessibility::backend_nsaccessibility::MacOSAccessibilityBackend::new(ns_view)
        .map(|b| {
            let backend: std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>> =
                std::rc::Rc::new(std::cell::RefCell::new(b));
            backend
        })
}

/// Construct the platform-native accessibility backend for a newly created
/// window, or `None` when no bridge is available on this target.
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
pub(crate) fn native_backend_for_window(
) -> Option<std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>>> {
    crate::accessibility::backend_atspi::LinuxAccessibilityBackend::new()
        .map(|b| {
            let backend: std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>> =
                std::rc::Rc::new(std::cell::RefCell::new(b));
            backend
        })
}

/// Construct the platform-native accessibility backend for a newly created
/// window, or `None` when no bridge is available on this target.
#[cfg(target_os = "windows")]
pub(crate) fn native_backend_for_window(
    hwnd: *mut std::ffi::c_void,
) -> Option<std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>>> {
    crate::accessibility::backend_uia::WindowsAccessibilityBackend::new(hwnd)
        .map(|b| {
            let backend: std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>> =
                std::rc::Rc::new(std::cell::RefCell::new(b));
            backend
        })
}

/// Construct the platform-native accessibility backend for a newly created
/// window, or `None` when no bridge is available on this target.
///
/// On iOS this needs the window's `UIView` pointer; VoiceOver anchors its
/// `UIAccessibilityElement` tree to it.
#[cfg(target_os = "ios")]
pub(crate) fn native_backend_for_window(
    ui_view: *mut std::ffi::c_void,
) -> Option<std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>>> {
    crate::accessibility::backend_ios::IosAccessibilityBackend::new(ui_view).map(|b| {
        let backend: std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>> =
            std::rc::Rc::new(std::cell::RefCell::new(b));
        backend
    })
}

/// Construct the platform-native accessibility backend for a newly created
/// window, or `None` when no bridge is available on this target.
///
/// On OpenHarmony the bridge is the N-API platform bridge (no native view
/// pointer is needed — the a11y tree is pushed to ArkTS, which registers it
/// with the OHOS Accessibility Kit). Like iOS, this is single-window today.
#[cfg(target_env = "ohos")]
pub(crate) fn native_backend_for_window(
) -> Option<std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>>> {
    crate::accessibility::backend_ohos::OhosAccessibilityBackend::new().map(|b| {
        let backend: std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>> =
            std::rc::Rc::new(std::cell::RefCell::new(b));
        backend
    })
}

/// Construct the platform-native accessibility backend for a newly created
/// window, or `None` when no bridge is available on this target.
///
/// On Android the bridge is a transparent `AccessibilityNodeProvider` overlay
/// that floom injects into the `Activity`'s decor view (see
/// `backend_android.rs` / `FloomAccessibilityView.java`). It needs no native
/// view pointer — it reaches the `Activity` through the shared JNI handle in
/// `crate::platform::android_jni`. Single-window today, like iOS/OHOS.
#[cfg(target_os = "android")]
pub(crate) fn native_backend_for_window(
) -> Option<std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>>> {
    crate::accessibility::backend_android::AndroidAccessibilityBackend::new().map(|b| {
        let backend: std::rc::Rc<std::cell::RefCell<dyn AccessibilityBackend>> =
            std::rc::Rc::new(std::cell::RefCell::new(b));
        backend
    })
}
