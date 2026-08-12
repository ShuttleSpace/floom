pub use floem_core_theme::component_tokens::ImageTokens;
pub use floem_core_theme::props::image::ImageProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_image::CupertinoImage as Image;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_image::CupertinoImageStyle as ImageStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_image::FluentImage as Image;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_image::FluentImageStyle as ImageStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_image::MdImage as Image;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_image::MdImageStyle as ImageStyle;

/// Runtime-dispatch `Image` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_image::MdImage;
    use floem_widget_cupertino_image::CupertinoImage;
    use floem_widget_fluent_image::FluentImage;

    pub struct Image {
        src: String,
        alt: String,
        width: Option<f32>,
        height: Option<f32>,
        rounded: bool,
        cover: bool,
        theme: Option<MultiTheme>,
    }

    impl Image {
        pub fn new() -> Self {
            Self {
                src: String::new(),
                alt: String::new(),
                width: None,
                height: None,
                rounded: false,
                cover: false,
                theme: None,
            }
        }

        pub fn src(mut self, src: impl Into<String>) -> Self {
            self.src = src.into();
            self
        }

        pub fn alt(mut self, alt: impl Into<String>) -> Self {
            self.alt = alt.into();
            self
        }

        pub fn width(mut self, width: f32) -> Self {
            self.width = Some(width);
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = Some(height);
            self
        }

        pub fn rounded(mut self, rounded: bool) -> Self {
            self.rounded = rounded;
            self
        }

        pub fn cover(mut self, cover: bool) -> Self {
            self.cover = cover;
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdImage::new()
                        .src(self.src)
                        .alt(self.alt)
                        .rounded(self.rounded)
                        .cover(self.cover);
                    if let Some(w) = self.width {
                        c = c.width(w);
                    }
                    if let Some(h) = self.height {
                        c = c.height(h);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoImage::new()
                        .src(self.src)
                        .alt(self.alt)
                        .rounded(self.rounded)
                        .cover(self.cover);
                    if let Some(w) = self.width {
                        c = c.width(w);
                    }
                    if let Some(h) = self.height {
                        c = c.height(h);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentImage::new()
                        .src(self.src)
                        .alt(self.alt)
                        .rounded(self.rounded)
                        .cover(self.cover);
                    if let Some(w) = self.width {
                        c = c.width(w);
                    }
                    if let Some(h) = self.height {
                        c = c.height(h);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for Image {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Image;
#[cfg(feature = "multi")]
pub use floem_widget_md_image::MdImageStyle as ImageStyle;
