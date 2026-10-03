use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::taffy::style::Overflow;
use floom::ui_events::{ScrollDelta, pointer::PointerScrollEvent};
use floom::view::AnyView;
use floom::views::{dyn_view, Empty, Label, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::infinite_scroller::InfiniteScrollerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoInfiniteScrollerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub loading_indicator_size: f32,
    pub footer_padding: f32,
}

pub struct CupertinoInfiniteScroller {
    props: InfiniteScrollerProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
    content_height: RwSignal<f32>,
    loading: RwSignal<bool>,
    viewport: f32,
}

impl CupertinoInfiniteScroller {
    pub fn new() -> Self {
        Self {
            props: InfiniteScrollerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            child: None,
            content_height: RwSignal::new(0.0),
            loading: RwSignal::new(false),
            viewport: 360.0,
        }
    }

    /// Set the scrollable content.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Total scrollable content height. Pass a signal so the host can grow it as more
    /// items are appended — otherwise near-end detection cannot recompute the max offset.
    pub fn content_height_signal(mut self, h: RwSignal<f32>) -> Self {
        self.content_height = h;
        self
    }

    /// Convenience: fixed content height for static content.
    pub fn content_height(self, h: f32) -> Self {
        self.content_height.set(h);
        self
    }

    /// Visible viewport height (default 360).
    pub fn viewport(mut self, h: f32) -> Self {
        self.viewport = h;
        self
    }

    /// Distance from the bottom (px) that triggers `on_load_more` (default 80).
    pub fn threshold(mut self, t: f32) -> Self {
        self.props.threshold = t;
        self
    }

    pub fn on_load_more(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_load_more = Some(Box::new(callback));
        self
    }

    /// Component-owned loading flag (mirrors `open_signal()`). Host sets true while
    /// fetching, false when done; the footer indicator and the re-fire guard read it.
    pub fn loading_signal(&self) -> RwSignal<bool> {
        self.loading
    }

    /// Initial loading state.
    pub fn loading(self, loading: bool) -> Self {
        self.loading.set(loading);
        self
    }

    pub fn resolved_style(&self) -> CupertinoInfiniteScrollerStyle {
        let theme = self.theme.get();
        let ComponentTokens::InfiniteScroller(tokens) = theme.component(ComponentId::InfiniteScroller)
        else {
            unreachable!("ComponentId::InfiniteScroller must resolve to InfiniteScrollerTokens")
        };
        CupertinoInfiniteScrollerStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            loading_indicator_size: tokens.loading_indicator_size,
            footer_padding: tokens.footer_padding,
        }
    }

    pub fn view(self) -> impl View {
        let style = self.resolved_style();
        let theme = self.theme;
        let on_load_more: Rc<Option<Box<dyn Fn()>>> = Rc::new(self.props.on_load_more);
        let threshold = self.props.threshold;
        let content_height = self.content_height;
        let loading = self.loading;
        let viewport = self.viewport;
        let indicator = style.loading_indicator_size;
        let footer_pad = style.footer_padding;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let offset = RwSignal::new(0.0_f32);

        let footer = dyn_view(move || {
            if loading.get() {
                Container::new(Label::new("Loading…"))
                    .style(move |s| {
                        s.height(indicator + footer_pad * 2.0)
                            .width_pct(100.0)
                            .items_center()
                            .justify_center()
                            .font_size(13.0)
                            .color(theme.get().colors().on_surface_variant.to_peniko())
                    })
                    .into_any()
            } else {
                Empty::new()
                    .style(move |s| s.height(footer_pad))
                    .into_any()
            }
        });

        let content_stack = Stack::vertical((child, footer)).style(move |s| {
            s.width_pct(100.0)
                .height(content_height.get())
                .translate_y(-offset.get())
        });

        let viewport_view = Container::new(content_stack)
            .style(move |s| {
                s.height(viewport)
                    .width_pct(100.0)
                    .overflow_x(Overflow::Clip)
                    .overflow_y(Overflow::Clip)
                    .background(style.surface.to_peniko())
                    .border(1.0)
                    .border_color(with_alpha(style.outline, 80).to_peniko())
            })
            .on_event(listener::PointerWheel, move |_, e: &PointerScrollEvent| {
                let dy = match e.delta {
                    ScrollDelta::PixelDelta(v) => v.y as f32,
                    ScrollDelta::LineDelta(_, y) => y * 40.0,
                    ScrollDelta::PageDelta(_, y) => y * viewport,
                };
                let max = (content_height.get() - viewport).max(0.0);
                let next = (offset.get() - dy).clamp(0.0, max);
                offset.set(next);
                let at_end = max > 0.0 && next >= max - threshold;
                if at_end && !loading.get() {
                    if let Some(cb) = on_load_more.as_ref() {
                        cb();
                    }
                }
                EventPropagation::Continue
            });

        viewport_view
    }
}

impl Default for CupertinoInfiniteScroller {
    fn default() -> Self {
        Self::new()
    }
}
