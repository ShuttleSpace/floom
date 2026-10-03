pub use floom_core_theme::component_tokens::FooterTokens;
pub use floom_core_theme::props::footer::FooterProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_footer::CupertinoFooter as Footer;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_footer::CupertinoFooterStyle as FooterStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_footer::FluentFooter as Footer;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_footer::FluentFooterStyle as FooterStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_footer::MdFooter as Footer;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_footer::MdFooterStyle as FooterStyle;

/// Runtime-dispatch `Footer` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::AnyView;
    use floom::view::View;
    use floom::views::{empty, Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floom_widget_md_footer::MdFooter;
    use floom_widget_cupertino_footer::CupertinoFooter;
    use floom_widget_fluent_footer::FluentFooter;

    pub struct Footer {
        /// `None` defers to the active design system's `FooterTokens::height`.
        height: Option<f32>,
        elevation: u8,
        bordered: bool,
        /// `None` defers to the active design system's `FooterTokens::pad`.
        pad: Option<f32>,
        child: Option<AnyView>,
    }

    impl Footer {
        pub fn new() -> Self {
            Self {
                height: None,
                elevation: 0,
                bordered: true,
                pad: None,
                child: None,
            }
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = Some(height);
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
            self.pad = Some(pad);
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            // `height`/`pad` are only forwarded when the caller set them
            // explicitly; otherwise each design system resolves its own
            // `FooterTokens` (M3 56/16, Cupertino 44/12, Fluent 48/16).
            let (height, pad) = (self.height, self.pad);
            let (elevation, bordered) = (self.elevation, self.bordered);
            match ds {
                DesignSystem::Material => {
                    let mut b = MdFooter::new().elevation(elevation).bordered(bordered);
                    if let Some(h) = height {
                        b = b.height(h);
                    }
                    if let Some(p) = pad {
                        b = b.pad(p);
                    }
                    b.child(child).view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoFooter::new().elevation(elevation).bordered(bordered);
                    if let Some(h) = height {
                        b = b.height(h);
                    }
                    if let Some(p) = pad {
                        b = b.pad(p);
                    }
                    b.child(child).view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentFooter::new().elevation(elevation).bordered(bordered);
                    if let Some(h) = height {
                        b = b.height(h);
                    }
                    if let Some(p) = pad {
                        b = b.pad(p);
                    }
                    b.child(child).view().into_any()
                }
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
pub use floom_widget_md_footer::MdFooterStyle as FooterStyle;
