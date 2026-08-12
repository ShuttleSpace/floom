//! App-wide layout signal — the "layout provider".
//!
//! Vuetify's `v-app` exposes a single layout object that the application shell
//! pieces (system bar, app bar, footer, bottom navigation, navigation drawer)
//! write their sizes into, and that `v-main` reads to offset its content so it
//! never hides under a fixed bar or an open drawer.
//!
//! This module provides the Floem equivalent:
//!
//! * [`LayoutSlot`] — which piece of the shell is reporting a size.
//! * [`LayoutState`] — the registered sizes plus derived [`top`](LayoutState::top) /
//!   [`bottom`](LayoutState::bottom) / [`left`](LayoutState::left) /
//!   [`right`](LayoutState::right) insets.
//! * [`layout_signal`] — a process-wide `RwSignal<LayoutState>` (mirrors
//!   [`floem_core_theme::global_theme::theme_signal`]).
//! * [`register_layout`] — called by each shell primitive from its `view()` so the
//!   size lands in the shared signal automatically.
//!
//! `RwSignal` is `!Sync`, so the shared instance lives in a `thread_local` cell
//! (the UI runs on a single thread). The signal is DS-agnostic: the same
//! accessor is used by the Material, Cupertino, and Fluent builds.

use std::cell::OnceCell;

use floem::reactive::{RwSignal, SignalUpdate};

/// Which piece of the application shell is reporting its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutSlot {
    /// Thin status strip at the very top (`SystemBar`).
    SystemBar,
    /// Primary top bar (`AppBar`).
    AppBar,
    /// Bottom content bar (`Footer`).
    Footer,
    /// Bottom navigation rail (`BottomNavigation`).
    BottomNavigation,
    /// Left-side navigation drawer, when open (`NavigationDrawer`).
    NavDrawerLeft,
    /// Right-side navigation drawer, when open (`NavigationDrawer`).
    NavDrawerRight,
}

/// Registered shell sizes. Each field is the rendered size (px) of that piece;
/// `0.0` means it is absent / closed.
///
/// The derived [`top`](LayoutState::top) / [`bottom`](LayoutState::bottom) /
/// [`left`](LayoutState::left) / [`right`](LayoutState::right) accessors are what
/// `Main` consumes as content insets.
#[derive(Debug, Clone, Default)]
pub struct LayoutState {
    pub system_bar: f32,
    pub app_bar: f32,
    pub footer: f32,
    pub bottom_navigation: f32,
    pub nav_drawer_left: f32,
    pub nav_drawer_right: f32,
}

impl LayoutState {
    /// Combined top inset: system bar + app bar.
    pub fn top(&self) -> f32 {
        self.system_bar + self.app_bar
    }
    /// Combined bottom inset: footer + bottom navigation.
    pub fn bottom(&self) -> f32 {
        self.footer + self.bottom_navigation
    }
    /// Left inset: open left navigation drawer width.
    pub fn left(&self) -> f32 {
        self.nav_drawer_left
    }
    /// Right inset: open right navigation drawer width.
    pub fn right(&self) -> f32 {
        self.nav_drawer_right
    }
}

thread_local! {
    static LAYOUT: OnceCell<RwSignal<LayoutState>> = const { OnceCell::new() };
}

fn layout_global() -> RwSignal<LayoutState> {
    LAYOUT.with(|c| *c.get_or_init(|| RwSignal::new(LayoutState::default())))
}

/// Returns a copy of the process-wide layout signal.
///
/// Every shell primitive and `Main` reads/writes this one signal, so a bar
/// registering its size re-insets `Main` automatically.
pub fn layout_signal() -> RwSignal<LayoutState> {
    layout_global()
}

/// Report (or clear) the rendered size of a shell piece.
///
/// Primitives call this from `view()` — statically for fixed-size pieces
/// (app bar, footer, …) or inside a `create_effect` for reactive pieces
/// (a navigation drawer that opens/closes).
pub fn register_layout(slot: LayoutSlot, value: f32) {
    layout_global().update(|s| match slot {
        LayoutSlot::SystemBar => s.system_bar = value,
        LayoutSlot::AppBar => s.app_bar = value,
        LayoutSlot::Footer => s.footer = value,
        LayoutSlot::BottomNavigation => s.bottom_navigation = value,
        LayoutSlot::NavDrawerLeft => s.nav_drawer_left = value,
        LayoutSlot::NavDrawerRight => s.nav_drawer_right = value,
    });
}
