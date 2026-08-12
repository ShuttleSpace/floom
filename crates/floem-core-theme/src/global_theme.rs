//! App-wide theme signal.
//!
//! Every component reads its visual tokens from a single theme signal so that
//! theming flows from one place (an app-level [`set_theme`] call or a
//! [`ThemeProvider`](floem_widget_runtime::theme_provider)) into every component.
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

use floem::reactive::{RwSignal, SignalGet, SignalUpdate};

use crate::theme::MultiTheme;

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
type SelectedTheme = MultiTheme;

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
thread_local! {
    static GLOBAL: OnceCell<RwSignal<SelectedTheme>> = const { OnceCell::new() };
}

#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
fn global() -> RwSignal<SelectedTheme> {
    GLOBAL.with(|c| *c.get_or_init(|| RwSignal::new(SelectedTheme::new_for(DesignSystem::default()))))
}

/// Returns a copy of the process-wide theme signal.
///
/// All components created without an explicit theme share this signal, so
/// changing it re-themes the whole application.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn theme_signal() -> RwSignal<SelectedTheme> {
    global()
}

/// Replace the app-wide theme. Every component reading the shared signal
/// updates automatically.
#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]
pub fn set_theme(theme: SelectedTheme) {
    global().set(theme);
}

/// The three Design Systems floem-ui supports.
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
    DS_GLOBAL.with(|c| *c.get_or_init(|| RwSignal::new(DesignSystem::default())))
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
/// value is what actually triggers Floem's reactive style re-evaluation).
pub fn set_design_system(ds: DesignSystem) {
    ds_global().set(ds);
    global().set(MultiTheme::new_for(ds));
}

/// Read the currently active Design System.
pub fn current_design_system() -> DesignSystem {
    ds_global().get()
}
