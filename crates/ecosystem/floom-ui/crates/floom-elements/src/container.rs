pub use floom_core_theme::component_tokens::ContainerTokens;
pub use floom_core_theme::props::container::ContainerProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `Container` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_container::CupertinoContainer as Container;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_container::CupertinoContainerStyle as ContainerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_container::FluentContainer as Container;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_container::FluentContainerStyle as ContainerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_container::MdContainer as Container;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_container::MdContainerStyle as ContainerStyle;

/// Runtime-dispatch `Container` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::AnyView;
    use floom::view::View;
    use floom::views::{empty, Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floom_widget_md_container::MdContainer;
    use floom_widget_cupertino_container::CupertinoContainer;
    use floom_widget_fluent_container::FluentContainer;

    pub struct Container {
        child: Option<AnyView>,
        max_width: f32,
        padding: f32,
        fluid: bool,
    }

    impl Container {
        pub fn new() -> Self {
            Self {
                child: None,
                max_width: 1200.0,
                padding: 16.0,
                fluid: false,
            }
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn max_width(mut self, max_width: f32) -> Self {
            self.max_width = max_width;
            self
        }

        pub fn padding(mut self, padding: f32) -> Self {
            self.padding = padding;
            self
        }

        pub fn fluid(mut self, fluid: bool) -> Self {
            self.fluid = fluid;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            match ds {
                DesignSystem::Material => MdContainer::new()
                    .child(child)
                    .max_width(self.max_width)
                    .padding(self.padding)
                    .fluid(self.fluid)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoContainer::new()
                    .child(child)
                    .max_width(self.max_width)
                    .padding(self.padding)
                    .fluid(self.fluid)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentContainer::new()
                    .child(child)
                    .max_width(self.max_width)
                    .padding(self.padding)
                    .fluid(self.fluid)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Container {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Container;
