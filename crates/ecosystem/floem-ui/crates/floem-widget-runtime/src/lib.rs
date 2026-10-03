//! Shared runtime helpers for floem-elements components.
//!
//! The theme signal is delegated to [`floem_core_theme::global_theme`] so that
//! every component (regardless of which crate it lives in) reads from one
//! app-wide theme instance. [`theme_provider`] lets an application install a
//! theme at the root of the view tree.
//!
//! If more than one Design System feature is enabled at once, the priority
//! Fluent > Cupertino > Material is applied so this module resolves to exactly
//! one `theme_signal`/`set_theme`/`theme_provider` definition set.

use floem::IntoView;
use floem::view::View;
use floem::views::{Container};
use floem::reactive::RwSignal;
use floem_core_theme::global_theme::{
    design_system_signal as design_system_signal_impl,
    dark_signal as dark_signal_impl,
    is_dark as is_dark_impl,
    set_dark as set_dark_impl,
    set_design_system as set_design_system_impl,
    set_theme as set_theme_impl,
    theme_signal as theme_signal_impl,
};
use floem_core_theme::layout::{layout_signal as layout_signal_impl, register_layout as register_layout_impl};
use floem_core_theme::theme::MultiTheme;

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
type SelectedTheme = MultiTheme;

/// Returns a copy of the app-wide theme signal.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn theme_signal() -> RwSignal<SelectedTheme> {
    theme_signal_impl()
}

/// Replace the app-wide theme so every component re-themes automatically.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn set_theme(theme: SelectedTheme) {
    set_theme_impl(theme);
}

/// Installs `theme` app-wide and renders `child` inside a plain container.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn theme_provider<V: IntoView + 'static>(theme: SelectedTheme, child: V) -> impl View {
    set_theme(theme);
    Container::new(child)
}

/// The three Design Systems floem-ui supports. Re-exported so application code
/// and the gallery switch without depending on `floem_core_theme` directly.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub use floem_core_theme::global_theme::DesignSystem;

/// Returns a copy of the process-wide active Design System signal.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn design_system_signal() -> RwSignal<DesignSystem> {
    design_system_signal_impl()
}

/// Set the active Design System at runtime (re-themes the whole app).
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn set_design_system(ds: DesignSystem) {
    set_design_system_impl(ds);
}

/// Returns a copy of the process-wide dark-mode signal. Flipping it re-themes
/// the whole app because every component reads its palette through
/// [`Theme::colors`](floem_core_theme::theme::Theme::colors), which branches on
/// this flag.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn dark_signal() -> RwSignal<bool> {
    dark_signal_impl()
}

/// Enable or disable dark mode app-wide.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn set_dark(dark: bool) {
    set_dark_impl(dark);
}

/// Read the current dark-mode flag.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn is_dark() -> bool {
    is_dark_impl()
}

/// Re-export of the layout provider types so application code depends on a
/// single facade regardless of the active Design System feature.
pub use floem_core_theme::layout::{LayoutSlot, LayoutState};

/// Reusable Material-style press ripple. Wrap any custom component with
/// [`Ripple::new`] (or the [`ripple`] convenience function) to give it a
/// pointer-down ink effect, with theme-aware color and corner radius.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
mod ripple;
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub use ripple::{ripple, Ripple};

/// Returns a copy of the app-wide layout signal (see
/// [`floem_core_theme::layout::layout_signal`]).
pub fn layout_signal() -> RwSignal<LayoutState> {
    layout_signal_impl()
}

/// Report the rendered size of a shell piece (see
/// [`floem_core_theme::layout::register_layout`]).
pub fn register_layout(slot: LayoutSlot, value: f32) {
    register_layout_impl(slot, value);
}

/// Ensures the layout signal exists and renders `child` inside a plain
/// container. `App` already does this; this helper is for manual composition.
pub fn layout_provider<V: IntoView + 'static>(child: V) -> impl View {
    let _ = layout_signal();
    Container::new(child)
}
