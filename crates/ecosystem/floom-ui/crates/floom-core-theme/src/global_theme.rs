//! App-wide theme signal.
//!
//! Every component reads its visual tokens from a single theme signal so that
//! theming flows from one place (an app-level [`set_theme`] call or a
//! [`ThemeProvider`](floom_widget_runtime::theme_provider)) into every component.
//!
//! [`RwSignal`] is `!Sync`, so the shared instance is kept in a `thread_local`
//! cell (the UI runs on a single thread). Exactly one Design System feature
//! (`material` / `cupertino` / `fluent`) is active at compile time, which
//! selects the concrete theme type returned here.
//!
//! If more than one feature somehow ends up enabled (for example the `material`
//! default plus an explicit `cupertino`), the priority Fluent > Cupertino >
//! Material is applied so the module always resolves to exactly one definition
//! set instead of emitting duplicate-symbol errors.

use std::cell::OnceCell;

use floom_signal::create_global_rw_signal;
use floom_signal::{RwSignal, SignalGet, SignalUpdate};

use crate::theme::MultiTheme;

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
type SelectedTheme = MultiTheme;

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
thread_local! {
    static GLOBAL: OnceCell<RwSignal<SelectedTheme>> = const { OnceCell::new() };
}

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
fn global() -> RwSignal<SelectedTheme> {
    // `create_global_rw_signal` binds the signal to the runtime's root scope,
    // which lives for the whole process and is never disposed. A plain
    // `RwSignal::new` would bind to whatever scope is active on first touch
    // (e.g. a `dyn_view` child scope) and get removed from the runtime when that
    // scope re-runs — after which every `.get()` panics in floom's
    // `reactive/src/read.rs`.
    GLOBAL.with(|c| {
        *c.get_or_init(|| create_global_rw_signal(SelectedTheme::new_for(DesignSystem::default())))
    })
}

/// Returns a copy of the process-wide theme signal.
///
/// All components created without an explicit theme share this signal, so
/// changing it re-themes the whole application.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn theme_signal() -> RwSignal<SelectedTheme> {
    global()
}

/// Replace the app-wide theme. Every component reading the shared signal
/// updates automatically.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn set_theme(theme: SelectedTheme) {
    global().set(theme);
}

/// The three Design Systems floom-ui supports.
///
/// At compile time exactly one is selected via the `material` / `cupertino` /
/// `fluent` features, which decides the concrete component and theme types.
/// This enum additionally lets a *running* application report and switch the
/// active Design System at runtime — the gallery uses it to re-skin without
/// recompiling. A runtime switch only affects components constructed
/// afterwards (or views that rebuild when the signal changes).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DesignSystem {
    /// Material 3 (default).
    #[default]
    Material,
    /// Apple-style Cupertino.
    Cupertino,
    /// Microsoft Fluent.
    Fluent,
}

impl DesignSystem {
    /// Human-readable label for UI controls.
    pub fn label(&self) -> &'static str {
        match self {
            DesignSystem::Material => "Material",
            DesignSystem::Cupertino => "Cupertino",
            DesignSystem::Fluent => "Fluent",
        }
    }
}

thread_local! {
    static DS_GLOBAL: OnceCell<RwSignal<DesignSystem>> = const { OnceCell::new() };
}

fn ds_global() -> RwSignal<DesignSystem> {
    // Bound to the never-disposed root scope (see `global()` above).
    DS_GLOBAL.with(|c| *c.get_or_init(|| create_global_rw_signal(DesignSystem::default())))
}

/// Returns a copy of the process-wide active Design System signal.
pub fn design_system_signal() -> RwSignal<DesignSystem> {
    ds_global()
}

/// Set the active Design System at runtime.
///
/// Also replaces the global theme signal with a fresh `MultiTheme` so that
/// every component reading the shared theme signal re-styles automatically
/// (the `MultiTheme` itself reads the active DS live, but bumping the signal
/// value is what actually triggers Floom's reactive style re-evaluation).
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn set_design_system(ds: DesignSystem) {
    ds_global().set(ds);
    global().set(MultiTheme::new_for(ds));
}

/// Read the currently active Design System.
pub fn current_design_system() -> DesignSystem {
    ds_global().get()
}

thread_local! {
    static DARK: OnceCell<RwSignal<bool>> = const { OnceCell::new() };
}

fn dark_global() -> RwSignal<bool> {
    // Bound to the never-disposed root scope (see `global()` above).
    DARK.with(|c| *c.get_or_init(|| create_global_rw_signal(false)))
}

/// Returns a copy of the process-wide dark-mode signal.
///
/// Every component reads its color palette through [`crate::theme::Theme::colors`],
/// which branches on this flag, so flipping it re-themes the whole app.
pub fn dark_signal() -> RwSignal<bool> {
    dark_global()
}

/// Enable or disable dark mode for the whole app.
pub fn set_dark(dark: bool) {
    dark_global().set(dark);
}

/// Read the current dark-mode flag.
pub fn is_dark() -> bool {
    dark_global().get()
}
