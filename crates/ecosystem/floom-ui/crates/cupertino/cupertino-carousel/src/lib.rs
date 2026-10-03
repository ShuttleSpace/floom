use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::taffy::style::Overflow;
use floom::view::AnyView;
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::carousel::CarouselProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoCarouselStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub border_radius: f32,
    pub height: f32,
    pub dot_size: f32,
    pub arrow_size: f32,
}

pub struct CupertinoCarousel {
    props: CarouselProps,
    theme: RwSignal<MultiTheme>,
    slides: Vec<AnyView>,
    height: f32,
}

impl CupertinoCarousel {
    pub fn new() -> Self {
        Self {
            props: CarouselProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            slides: Vec::new(),
            height: 0.0,
        }
    }

    /// Provide the slide contents. `AnyView` is owned (not `Clone`), so pass the full set here.
    pub fn slides(mut self, slides: Vec<AnyView>) -> Self {
        self.slides = slides;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn auto_play(mut self, auto_play: bool) -> Self {
        self.props.auto_play = auto_play;
        self
    }

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

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> CupertinoCarouselStyle {
        let theme = self.theme.get();
        let ComponentTokens::Carousel(tokens) = theme.component(ComponentId::Carousel) else {
            unreachable!("ComponentId::Carousel must resolve to CarouselTokens")
        };
        let h = if self.height > 0.0 {
            self.height
        } else {
            tokens.height
        };
        CupertinoCarouselStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            border_radius: tokens.border_radius,
            height: h,
            dot_size: tokens.dot_size,
            arrow_size: tokens.arrow_size,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let slides = self.slides;
        let n = slides.len();
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.props.on_change);
        let show_arrows = self.props.show_arrows;
        let show_dots = self.props.show_dots;

        let tokens = match theme.get().component(ComponentId::Carousel) {
            ComponentTokens::Carousel(t) => t,
            _ => unreachable!(),
        };
        let dot_size = tokens.dot_size;
        let dot_gap = tokens.dot_gap;
        let dot_inactive = tokens.dot_inactive_opacity;
        let arrow_size = tokens.arrow_size;
        let border_radius = tokens.border_radius;
        let height = if self.height > 0.0 {
            self.height
        } else {
            tokens.height
        };

        let index = RwSignal::new(0usize);

        let mut children: Vec<AnyView> = Vec::new();

        for (i, slide) in slides.into_iter().enumerate() {
            let index = index;
            children.push(
                slide
                    .style(move |s| {
                        s.absolute()
                            .inset_left_pct(0.0)
                            .inset_top_pct(0.0)
                            .width_pct(100.0)
                            .height_pct(100.0)
                            .apply_if(i != index.get(), |s| s.hide())
                    })
                    .into_any(),
            );
        }

        if show_arrows && n > 1 {
            let index = index;
            let on_change_prev = on_change.clone();
            let prev = Container::new(Label::new("‹").style(move |s| {
                s.font_size(arrow_size * 0.6)
                    .color(theme.get().colors().on_surface.to_peniko())
            }))
            .style(move |s| {
                s.absolute()
                    .inset_left_pct(0.0)
                    .inset_top_pct(0.0)
                    .height_pct(100.0)
                    .width(40.0)
                    .items_center()
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                let cur = index.get();
                index.set(if cur == 0 { n - 1 } else { cur - 1 });
                if let Some(cb) = on_change_prev.as_ref() {
                    cb(index.get());
                }
                EventPropagation::Continue
            })
            .into_any();
            children.push(prev);

            let index = index;
            let on_change_next = on_change.clone();
            let next = Container::new(Label::new("›").style(move |s| {
                s.font_size(arrow_size * 0.6)
                    .color(theme.get().colors().on_surface.to_peniko())
            }))
            .style(move |s| {
                s.absolute()
                    .inset_right_pct(0.0)
                    .inset_top_pct(0.0)
                    .height_pct(100.0)
                    .width(40.0)
                    .items_center()
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                let cur = index.get();
                index.set(if cur >= n - 1 { 0 } else { cur + 1 });
                if let Some(cb) = on_change_next.as_ref() {
                    cb(index.get());
                }
                EventPropagation::Continue
            })
            .into_any();
            children.push(next);
        }

        if show_dots && n > 0 {
            let index = index;
            let on_change = on_change.clone();
            let dots: Vec<AnyView> = (0..n)
                .map(|i| {
                    let idx = index;
                    let on_change = on_change.clone();
                    Empty::new()
                        .style(move |s| {
                            let active = i == idx.get();
                            let t = theme.get();
                            s.width(dot_size)
                                .height(dot_size)
                                .border_radius(dot_size / 2.0)
                                .margin(dot_gap / 2.0)
                                .background(if active {
                                    t.colors().primary.to_peniko()
                                } else {
                                    with_alpha(
                                        t.colors().on_surface,
                                        (dot_inactive * 255.0) as u8,
                                    )
                                    .to_peniko()
                                })
                                .cursor(CursorStyle::Pointer)
                        })
                        .on_event(listener::Click, move |_, _| {
                            idx.set(i);
                            if let Some(cb) = on_change.as_ref() {
                                cb(i);
                            }
                            EventPropagation::Continue
                        })
                        .into_any()
                })
                .collect();
            let dots_row = Stack::horizontal_from_iter(dots)
                .style(|s| s.justify_center().items_center().gap(0.0));
            let dots_overlay = Container::new(dots_row)
                .style(move |s| {
                    s.absolute()
                        .inset_bottom_pct(2.0)
                        .inset_left_pct(0.0)
                        .inset_right_pct(0.0)
                        .height(dot_size * 2.0)
                        .items_center()
                        .justify_center()
                })
                .into_any();
            children.push(dots_overlay);
        }

        let area = Stack::new(children).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height(height)
                .border_radius(border_radius)
                .overflow_x(Overflow::Clip)
                .overflow_y(Overflow::Clip)
                .background(t.colors().surface.to_peniko())
                .border(1.0)
                .border_color(with_alpha(t.colors().outline, 80).to_peniko())
        });

        area
    }
}

impl Default for CupertinoCarousel {
    fn default() -> Self {
        Self::new()
    }
}
