pub use floem_core_theme::component_tokens::CarouselTokens;
pub use floem_core_theme::props::carousel::CarouselProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_carousel::CupertinoCarousel as Carousel;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_carousel::CupertinoCarouselStyle as CarouselStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_carousel::FluentCarousel as Carousel;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_carousel::FluentCarouselStyle as CarouselStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_carousel::MdCarousel as Carousel;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_carousel::MdCarouselStyle as CarouselStyle;

/// Runtime-dispatch `Carousel` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::{AnyView, View};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::carousel::CarouselProps;

    use floem_widget_md_carousel::MdCarousel;
    use floem_widget_cupertino_carousel::CupertinoCarousel;
    use floem_widget_fluent_carousel::FluentCarousel;

    pub struct Carousel {
        slides: Vec<AnyView>,
        height: f32,
        auto_play: bool,
        interval_ms: u64,
        show_arrows: bool,
        show_dots: bool,
        on_change: Option<Box<dyn Fn(usize)>>,
    }

    impl Carousel {
        pub fn new() -> Self {
            let props = CarouselProps::default();
            Self {
                slides: Vec::new(),
                height: 0.0,
                auto_play: props.auto_play,
                interval_ms: props.interval_ms,
                show_arrows: props.show_arrows,
                show_dots: props.show_dots,
                on_change: None,
            }
        }

        pub fn slides(mut self, slides: Vec<AnyView>) -> Self {
            self.slides = slides;
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = height;
            self
        }

        pub fn auto_play(mut self, auto_play: bool) -> Self {
            self.auto_play = auto_play;
            self
        }

        pub fn interval_ms(mut self, interval_ms: u64) -> Self {
            self.interval_ms = interval_ms;
            self
        }

        pub fn show_arrows(mut self, show_arrows: bool) -> Self {
            self.show_arrows = show_arrows;
            self
        }

        pub fn show_dots(mut self, show_dots: bool) -> Self {
            self.show_dots = show_dots;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self
                .on_change
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdCarousel::new()
                    .slides(self.slides)
                    .height(self.height)
                    .auto_play(self.auto_play)
                    .interval_ms(self.interval_ms)
                    .show_arrows(self.show_arrows)
                    .show_dots(self.show_dots)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoCarousel::new()
                    .slides(self.slides)
                    .height(self.height)
                    .auto_play(self.auto_play)
                    .interval_ms(self.interval_ms)
                    .show_arrows(self.show_arrows)
                    .show_dots(self.show_dots)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentCarousel::new()
                    .slides(self.slides)
                    .height(self.height)
                    .auto_play(self.auto_play)
                    .interval_ms(self.interval_ms)
                    .show_arrows(self.show_arrows)
                    .show_dots(self.show_dots)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Carousel {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Carousel;
#[cfg(feature = "multi")]
pub use floem_widget_md_carousel::MdCarouselStyle as CarouselStyle;
