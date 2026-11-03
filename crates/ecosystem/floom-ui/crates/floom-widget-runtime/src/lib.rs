
use floom_signal::RwSignal;

// `DesignSystem`, `MultiTheme` and the three theme mutators only exist when a
// design-system feature is on, so their imports are gated to match.
use floom_core_theme::global_theme::{
    dark_signal as dark_signal_impl, is_dark as is_dark_impl, set_dark as set_dark_impl,
};
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
use floom_core_theme::global_theme::{
    design_system_signal as design_system_signal_impl, set_design_system as set_design_system_impl,
    set_theme as set_theme_impl, theme_signal as theme_signal_impl, DesignSystem,
};
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
use floom_core_theme::theme::MultiTheme;

use floom_core_theme::layout::{
    layout_signal as layout_signal_impl, register_layout as register_layout_impl,
};

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

// The press ripple now lives in the engine crate (`floom_widgets::Ripple`),
// ported from the v1 wrapper this crate used to hold.
//
// The v1 version was a `View` wrapper that needed a `LayoutChangedListener` to
// learn its own box before it could size the ink. 2.0 hands `paint` and `event`
// `cx.bounds()` directly, so that listener is gone, and the two-phase
// pointer-driven animation it needed is expressible through the declarative
// contract -- see `floom-widgets/src/ripple.rs` and
// `floom-app/tests/event_animation.rs` for the proof and the control.
//
// Re-exported here rather than reimplemented, because 91 of the 99 crates that
// depend on this one reach it through this path.
pub use floom_widgets::{ripple, Phase as RipplePhase, Ripple};
