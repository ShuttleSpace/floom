/// Neutral, design-system-agnostic props for the carousel.
///
/// Slide content is passed through the widget builder (e.g. `slides(Vec<AnyView>)`)
/// rather than these props, because `AnyView` is not `Clone` and props must be.
pub struct CarouselProps {
    /// Automatically advance slides on a timer.
    pub auto_play: bool,
    /// Auto-advance interval in milliseconds.
    pub interval_ms: u64,
    /// Show previous/next arrow controls.
    pub show_arrows: bool,
    /// Show the dot/page indicator.
    pub show_dots: bool,
    /// Called with the new active index when the slide changes.
    pub on_change: Option<Box<dyn Fn(usize)>>,
}

impl Clone for CarouselProps {
    fn clone(&self) -> Self {
        Self {
            auto_play: self.auto_play,
            interval_ms: self.interval_ms,
            show_arrows: self.show_arrows,
            show_dots: self.show_dots,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for CarouselProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CarouselProps")
            .field("auto_play", &self.auto_play)
            .field("interval_ms", &self.interval_ms)
            .field("show_arrows", &self.show_arrows)
            .field("show_dots", &self.show_dots)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(usize)"))
            .finish()
    }
}

impl Default for CarouselProps {
    fn default() -> Self {
        Self {
            auto_play: false,
            interval_ms: 4000,
            show_arrows: true,
            show_dots: true,
            on_change: None,
        }
    }
}
