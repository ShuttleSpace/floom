use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::props::theme_provider::ThemeProviderProps;

/// Material [`ThemeProvider`]: owns a process-wide dark-mode [`RwSignal`] and
/// shares it with its subtree through [`MdThemeProvider::dark_signal`].
///
/// NOTE: because Rust/Floem uses static dispatch, this provider cannot perform
/// subtree-scoped re-theming the way a dynamically-typed UI framework would.
/// Instead it hands descendants a typed signal they read and write; any widget
/// that honors dark mode must read `dark_signal()` and react to it.
pub struct MdThemeProvider {
    props: ThemeProviderProps,
    dark: RwSignal<bool>,
    child: Option<AnyView>,
}

impl MdThemeProvider {
    pub fn new() -> Self {
        let props = ThemeProviderProps::default();
        Self {
            dark: RwSignal::new(props.dark),
            props,
            child: None,
        }
    }

    /// Initial dark-mode flag propagated to descendants.
    pub fn dark(mut self, dark: bool) -> Self {
        self.props.dark = dark;
        self.dark.set(dark);
        self
    }

    /// The subtree wrapped by this provider.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Read/write handle for the dark-mode flag, shared with descendants.
    pub fn dark_signal(&self) -> RwSignal<bool> {
        self.dark
    }

    /// Immutable access to the resolved provider props.
    pub fn props(&self) -> &ThemeProviderProps {
        &self.props
    }

    pub fn view(self) -> impl View {
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child)
    }
}

impl Default for MdThemeProvider {
    fn default() -> Self {
        Self::new()
    }
}
