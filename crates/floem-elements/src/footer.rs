pub use floem_core_theme::component_tokens::FooterTokens;
pub use floem_core_theme::props::footer::FooterProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_footer::CupertinoFooter as Footer;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_footer::CupertinoFooterStyle as FooterStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_footer::FluentFooter as Footer;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_footer::FluentFooterStyle as FooterStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_footer::MdFooter as Footer;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_footer::MdFooterStyle as FooterStyle;

/// Runtime-dispatch `Footer` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::empty;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_footer::MdFooter;
    use floem_widget_cupertino_footer::CupertinoFooter;
    use floem_widget_fluent_footer::FluentFooter;

    pub struct Footer {
        height: f32,
        elevation: u8,
        bordered: bool,
        pad: f32,
        child: Option<AnyView>,
    }

    impl Footer {
        pub fn new() -> Self {
            Self {
                height: 56.0,
                elevation: 0,
                bordered: true,
                pad: 16.0,
                child: None,
            }
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = height;
            self
        }

        pub fn elevation(mut self, elevation: u8) -> Self {
            self.elevation = elevation;
            self
        }

        pub fn bordered(mut self, bordered: bool) -> Self {
            self.bordered = bordered;
            self
        }

        pub fn pad(mut self, pad: f32) -> Self {
            self.pad = pad;
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| empty().into_any());
            match ds {
                DesignSystem::Material => MdFooter::new()
                    .height(self.height)
                    .elevation(self.elevation)
                    .bordered(self.bordered)
                    .pad(self.pad)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoFooter::new()
                    .height(self.height)
                    .elevation(self.elevation)
                    .bordered(self.bordered)
                    .pad(self.pad)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentFooter::new()
                    .height(self.height)
                    .elevation(self.elevation)
                    .bordered(self.bordered)
                    .pad(self.pad)
                    .child(child)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Footer {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Footer;
#[cfg(feature = "multi")]
pub use floem_widget_md_footer::MdFooterStyle as FooterStyle;
