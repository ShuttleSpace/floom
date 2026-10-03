pub use floom_core_theme::component_tokens::EmptyStateTokens;
pub use floom_core_theme::props::empty_state::EmptyStateProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_empty_state::CupertinoEmptyState as EmptyState;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_empty_state::CupertinoEmptyStateStyle as EmptyStateStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_empty_state::FluentEmptyState as EmptyState;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_empty_state::FluentEmptyStateStyle as EmptyStateStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_empty_state::MdEmptyState as EmptyState;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_empty_state::MdEmptyStateStyle as EmptyStateStyle;

/// Runtime-dispatch `EmptyState` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::AnyView;
    use floom::view::View;
    use floom::views::{empty, Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floom_widget_md_empty_state::MdEmptyState;
    use floom_widget_cupertino_empty_state::CupertinoEmptyState;
    use floom_widget_fluent_empty_state::FluentEmptyState;

    pub struct EmptyState {
        icon: String,
        title: String,
        description: String,
        child: Option<AnyView>,
    }

    impl EmptyState {
        pub fn new() -> Self {
            Self {
                icon: String::new(),
                title: "No data".to_string(),
                description: String::new(),
                child: None,
            }
        }

        pub fn icon(mut self, icon: impl Into<String>) -> Self {
            self.icon = icon.into();
            self
        }

        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.title = title.into();
            self
        }

        pub fn description(mut self, description: impl Into<String>) -> Self {
            self.description = description.into();
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            match ds {
                DesignSystem::Material => MdEmptyState::new()
                    .icon(self.icon)
                    .title(self.title)
                    .description(self.description)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoEmptyState::new()
                    .icon(self.icon)
                    .title(self.title)
                    .description(self.description)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentEmptyState::new()
                    .icon(self.icon)
                    .title(self.title)
                    .description(self.description)
                    .child(child)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for EmptyState {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::EmptyState;
#[cfg(feature = "multi")]
pub use floom_widget_md_empty_state::MdEmptyStateStyle as EmptyStateStyle;
