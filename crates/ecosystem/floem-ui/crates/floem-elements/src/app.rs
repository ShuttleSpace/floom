//! Neutral `App` dispatch — re-exports the active Design System's application
//! root (the layout provider frame).

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_app::CupertinoApp as App;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_app::FluentApp as App;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_app::MdApp as App;

/// Runtime-dispatch `App` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::{AnyView, View};
    use floem::views::{empty, Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_app::MdApp;
    use floem_widget_cupertino_app::CupertinoApp;
    use floem_widget_fluent_app::FluentApp;

    pub struct App {
        child: Option<AnyView>,
    }

    impl App {
        pub fn new() -> Self {
            Self { child: None }
        }

        /// The application content (shell bars + main). Rendered full-viewport.
        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            match ds {
                DesignSystem::Material => MdApp::new().child(child).view().into_any(),
                DesignSystem::Cupertino => CupertinoApp::new().child(child).view().into_any(),
                DesignSystem::Fluent => FluentApp::new().child(child).view().into_any(),
            }
        }
    }

    impl Default for App {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::App;
