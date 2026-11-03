//! Fluent carousel, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{current_index, set_index, Carousel, CarouselStyle};
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::carousel::CarouselProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const BORDER_ALPHA: u8 = 80;
const BORDER_WIDTH: f32 = 1.0;
const ARROW_WIDTH: f32 = 40.0;
const ARROW_FONT_RATIO: f32 = 0.6;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentCarouselStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    /// v1 reported this and the view never used it -- see the module note.
    pub on_surface_variant: Color,
    pub primary: Color,
    pub border_radius: f32,
    pub height: f32,
    pub dot_size: f32,
    pub arrow_size: f32,
    pub outline: Color,
    pub border_width: f32,
    pub border_alpha: u8,
    pub dot_gap: f32,
    pub dot_inactive_opacity: f32,
    pub arrow_font_size: f32,
    pub arrow_width: f32,
    /// The token's slide gap. v1 never read it -- see the module note.
    pub slide_gap: f32,
}

pub struct FluentCarousel {
    props: CarouselProps,
    theme: RwSignal<MultiTheme>,
    slides: Vec<Child>,
    height: f32,
    flags: Vec<SharedFlag>,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl FluentCarousel {
    pub fn new() -> Self {
        Self {
            props: CarouselProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            slides: Vec::new(),
            height: 0.0,
            flags: Vec::new(),
            on_change: None,
        }
    }

    pub fn slides(mut self, slides: Vec<Child>) -> Self {
        self.flags = slides.iter().map(|_| shared_flag(false)).collect();
        if let Some(first) = self.flags.first() {
            write_flag(first, true);
        }
        self.slides = slides;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// v1 carried this and never ran it -- see the module note.
    pub fn auto_play(mut self, auto_play: bool) -> Self {
        self.props.auto_play = auto_play;
        self
    }

    /// v1 carried this and never ran it -- see the module note.
    pub fn interval_ms(mut self, interval_ms: u64) -> Self {
        self.props.interval_ms = interval_ms;
        self
    }

    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    pub fn show_dots(mut self, show_dots: bool) -> Self {
        self.props.show_dots = show_dots;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn(usize) + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_change = Some(Box::new({
            let shared = shared.clone();
            move |index| shared(index)
        }));
        self.on_change = Some(shared);
        self
    }

    pub fn index(&self) -> usize {
        current_index(&self.flags)
    }

    pub fn set_index(&self, index: usize) {
        set_index(&self.flags, index);
    }

    pub fn props(&self) -> &CarouselProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentCarouselStyle {
        let theme = self.theme.get();
        let ComponentTokens::Carousel(tokens) = theme.component(ComponentId::Carousel) else {
            unreachable!("ComponentId::Carousel must resolve to CarouselTokens")
        };
        let height = if self.height > 0.0 {
            self.height
        } else {
            tokens.height
        };
        FluentCarouselStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            border_radius: tokens.border_radius,
            height,
            dot_size: tokens.dot_size,
            arrow_size: tokens.arrow_size,
            outline: theme.colors().outline,
            border_width: BORDER_WIDTH,
            border_alpha: BORDER_ALPHA,
            dot_gap: tokens.dot_gap,
            dot_inactive_opacity: tokens.dot_inactive_opacity,
            arrow_font_size: tokens.arrow_size * ARROW_FONT_RATIO,
            arrow_width: ARROW_WIDTH,
            slide_gap: tokens.slide_gap,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let mut carousel = Carousel::new()
            .style(CarouselStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                primary: to_paint(style.primary),
                border_radius: style.border_radius,
                height: style.height,
                dot_size: style.dot_size,
                arrow_size: style.arrow_size,
                outline: to_paint(style.outline),
                border_width: style.border_width,
                border_alpha: style.border_alpha,
                dot_gap: style.dot_gap,
                dot_inactive_opacity: style.dot_inactive_opacity,
                arrow_font_size: style.arrow_font_size,
                arrow_width: style.arrow_width,
                slide_gap: style.slide_gap,
            })
            .slides(self.slides.clone())
            .flags(self.flags.clone())
            .show_arrows(self.props.show_arrows)
            .show_dots(self.props.show_dots);
        if let Some(on_change) = self.on_change.clone() {
            carousel = carousel.on_change(move |index| on_change(index));
        }
        carousel.build()
    }
}

impl Default for FluentCarousel {
    fn default() -> Self {
        Self::new()
    }
}
