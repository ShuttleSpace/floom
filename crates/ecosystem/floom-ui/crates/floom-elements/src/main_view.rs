//! Neutral `Main` dispatch — re-exports the active Design System's main content
//! region (auto-insets itself from the shared layout signal).

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_main::CupertinoMain as Main;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_main::FluentMain as Main;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_main::MdMain as Main;

/// Runtime-dispatch `Main` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::{AnyView, View};
    use floom::views::{empty, Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floom_widget_md_main::MdMain;
    use floom_widget_cupertino_main::CupertinoMain;
    use floom_widget_fluent_main::FluentMain;

    pub struct Main {
        child: Option<AnyView>,
    }

    impl Main {
        pub fn new() -> Self {
            Self { child: None }
        }

        /// The scrollable/static content of the application.
        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            match ds {
                DesignSystem::Material => MdMain::new().child(child).view().into_any(),
                DesignSystem::Cupertino => CupertinoMain::new().child(child).view().into_any(),
                DesignSystem::Fluent => FluentMain::new().child(child).view().into_any(),
            }
        }
    }

    impl Default for Main {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Main;
