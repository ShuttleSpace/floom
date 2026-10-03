//! Platform-agnostic menu types.
//!
//! This module provides unified menu types that work across all platforms,
//! using muda on native platforms and wasm_stubs on wasm32.

// Android and iOS have no native menu backend (muda). Reuse the wasm stub menu
// types so the `Menu`/`Submenu` API surface still compiles; both mobile
// platforms use the in-UI context menu renderer instead of a native menu bar.
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(
        target_os = "android",
        target_os = "ios",
        target_env = "ohos"
    ))
))]
pub use muda::{
    CheckMenuItem, Icon, IconMenuItem, IsMenuItem, Menu, MenuId, MenuItem, NativeIcon,
    PredefinedMenuItem, Submenu, accelerator::Accelerator,
};

#[cfg(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios",
    target_env = "ohos"
))]
pub use crate::platform::wasm_stubs::{
    Accelerator, CheckMenuItem, Icon, IconMenuItem, IsMenuItem, Menu, MenuId, MenuItem,
    MenuItemKind, NativeIcon, PredefinedMenuItem, Submenu,
};

// MenuItemKind is only used in context_menu.rs, which is only compiled on Linux/FreeBSD/wasm32
#[cfg(all(
    any(target_os = "linux", target_os = "freebsd"),
    not(target_env = "ohos")
))]
pub use muda::MenuItemKind;
