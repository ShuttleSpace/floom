pub use floem_core_theme::component_tokens::SpacerTokens;
pub use floem_core_theme::props::spacer::SpacerProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_spacer::CupertinoSpacer as Spacer;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_spacer::CupertinoSpacerStyle as SpacerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_spacer::FluentSpacer as Spacer;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_spacer::FluentSpacerStyle as SpacerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_spacer::MdSpacer as Spacer;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_spacer::MdSpacerStyle as SpacerStyle;

/// Runtime-dispatch `Spacer` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_spacer::MdSpacer;
    use floem_widget_cupertino_spacer::CupertinoSpacer;
    use floem_widget_fluent_spacer::FluentSpacer;

    pub struct Spacer {
        grow: f32,
    }

    impl Spacer {
        pub fn new() -> Self {
            Self { grow: 1.0 }
        }

        pub fn grow(mut self, grow: f32) -> Self {
            self.grow = grow;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdSpacer::new().grow(self.grow).view().into_any(),
                DesignSystem::Cupertino => CupertinoSpacer::new().grow(self.grow).view().into_any(),
                DesignSystem::Fluent => FluentSpacer::new().grow(self.grow).view().into_any(),
            }
        }
    }

    impl Default for Spacer {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Spacer;
#[cfg(feature = "multi")]
pub use floem_widget_md_spacer::MdSpacerStyle as SpacerStyle;
