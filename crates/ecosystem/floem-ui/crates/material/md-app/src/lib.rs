use floem::prelude::*;
use floem::view::AnyView;
use floem::views::{Container, empty, Empty};
use floem_core_theme::layout::layout_signal;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_widget_runtime::theme_signal;

/// Material 3 application root — the layout provider frame.
///
/// Initializes the shared layout signal and paints the app background; the
/// supplied `child` (the shell bars + [`MdMain`](crate::MdMain)) fills the
/// viewport.
pub struct MdApp {
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl MdApp {
    pub fn new() -> Self {
        Self {
            theme: theme_signal(),
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

impl Default for MdApp {
    fn default() -> Self {
        Self::new()
    }
}
