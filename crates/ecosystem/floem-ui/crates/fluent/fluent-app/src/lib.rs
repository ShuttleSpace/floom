use floem::prelude::*;
use floem::view::AnyView;
use floem::views::{Container, Empty};
use floem_core_theme::layout::layout_signal;
use floem_core_theme::theme::{MultiTheme, Theme};
/// Material 3 application root — the layout provider frame.
///
/// Initializes the shared layout signal and paints the app background; the
/// supplied `child` (the shell bars + [`FluentMain`](crate::FluentMain)) fills the
/// viewport.
pub struct FluentApp {
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentApp {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            child: None,
        }
    }

    /// The application content (shell bars + main). Rendered full-viewport.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Build the application root frame.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        // Initialize the process-wide layout signal for the whole app.
        let _ = layout_signal();
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height_pct(100.0)
                .background(t.colors().background.to_peniko())
        })
    }
}

impl Default for FluentApp {
    fn default() -> Self {
        Self::new()
    }
}
