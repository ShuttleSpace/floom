//! Shared runtime helpers for floem-elements components.
//!
//! The theme signal is delegated to [`floom_core_theme::global_theme`] so that
//! every component (regardless of which crate it lives in) reads from one
//! app-wide theme instance.
//!
//! If more than one Design System feature is enabled at once, the priority
//! Fluent > Cupertino > Material is applied so this module resolves to exactly
//! one `theme_signal`/`set_theme` definition set.
//!
//! # Ported to the 2.0 API
//!
//! Theme state is just an `RwSignal`, and the 2.0 prelude already re-exports
//! `RwSignal` from `floom-legacy-reactive`, so this crate needs no widget layer
//! and no `floom` (v1) dependency. See `docs/porting-floem-ui-to-2.0.md`.

use floom_legacy_reactive::RwSignal;

// `DesignSystem`, `MultiTheme` and the three theme mutators only exist when a
// design-system feature is on, so their imports are gated to match.
use floom_core_theme::global_theme::{
    dark_signal as dark_signal_impl,
    is_dark as is_dark_impl,
    set_dark as set_dark_impl,
};
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
use floom_core_theme::global_theme::{
    design_system_signal as design_system_signal_impl,
    set_design_system as set_design_system_impl,
    set_theme as set_theme_impl,
    theme_signal as theme_signal_impl,
    DesignSystem,
};
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
use floom_core_theme::theme::MultiTheme;

use floom_core_theme::layout::{layout_signal as layout_signal_impl, register_layout as register_layout_impl};

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

/// Installs `theme` app-wide.
///
/// In the v1 API this also took a child and returned `impl View`, wrapping it
/// in a `Container`. 2.0 has no equivalent wrapper: `Container` is a leaf
/// widget (paint, background, padding, filters) with no `.child()` builder,
/// because composition moved to the parent via
/// `Child::new(ChildKey, LayoutRequest, || Box::new(..))`. Wrapping is therefore
/// the caller's job, and installing the theme is all that is left to do here.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn theme_provider(theme: SelectedTheme) {
    set_theme(theme);
}

/// The three Design Systems floom-ui supports. Re-exported so application code
/// and the gallery switch without depending on `floom_core_theme` directly.
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
/// [`Theme::colors`](floom_core_theme::theme::Theme::colors), which branches on
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
pub use floom_core_theme::layout::{LayoutSlot, LayoutState};

/// Returns a copy of the app-wide layout signal (see
/// [`floom_core_theme::layout::layout_signal`]).
pub fn layout_signal() -> RwSignal<LayoutState> {
    layout_signal_impl()
}

/// Report the rendered size of a shell piece (see
/// [`floom_core_theme::layout::register_layout`]).
pub fn register_layout(slot: LayoutSlot, value: f32) {
    register_layout_impl(slot, value);
}

/// Ensures the layout signal exists.
///
/// This used to take a child and return `impl View`; as with
/// [`theme_provider`], 2.0 has no wrapper widget to return, so it now just
/// materialises the signal. `App` already does this.
pub fn layout_provider() {
    let _ = layout_signal();
}

// `Ripple` is not yet ported. The v1 implementation was a `View` wrapper that
// animated a growing ink circle on pointer-down and faded it on pointer-up,
// clipped to the wrapper's rounded bounds. 2.0 has the pieces it needs --
// `WidgetHost::bounds` / `dispatch` / `animate`, and `Widget::clip` -- but the
// animation contract is *declarative* (a widget returns one `AnimationSpec`
// from `Widget::animation` and the host owns the running state), which does not
// express a two-phase pointer-driven interaction: expand on press, fade on
// release. That gap is the first item in Stage B of
// `docs/porting-floem-ui-to-2.0.md`.
//
// The ten components that use it (`{material,cupertino,fluent}-card`,
// `-list_item`, `-icon_button`, and `material-button`) will not build until it
// lands; everything else in this crate is live.