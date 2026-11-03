//! Fluent infinite scroller, ported to the Floom 2.0 widget API.

use std::sync::{Arc, Mutex};

use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::InfiniteScrollerView;
use floom_widgets::{shared_flag, write_flag, SharedFlag, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::infinite_scroller::InfiniteScrollerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

pub type SharedHeight = floom_ui_composites::SharedHeight;

pub fn shared_height(height: f32) -> SharedHeight {
    Arc::new(Mutex::new(height))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentInfiniteScrollerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub loading_indicator_size: f32,
    pub footer_padding: f32,
}

pub struct FluentInfiniteScroller {
    props: InfiniteScrollerProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
    content_height: SharedHeight,
    loading: SharedFlag,
    viewport: f32,
    on_load_more: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl FluentInfiniteScroller {
    pub fn new() -> Self {
        Self {
            props: InfiniteScrollerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
            content_height: shared_height(0.0),
            loading: shared_flag(false),
            viewport: 360.0,
            on_load_more: None,
        }
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn content_height_cell(mut self, height: SharedHeight) -> Self {
        self.content_height = height;
        self
    }

    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = shared_height(height);
        self
    }

    pub fn viewport(mut self, height: f32) -> Self {
        self.viewport = height;
        self
    }

    pub fn threshold(mut self, threshold: f32) -> Self {
        self.props.threshold = threshold;
        self
    }

    pub fn on_load_more(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_load_more = Some(Arc::new(callback));
        self
    }

    pub fn loading_flag(&self) -> SharedFlag {
        self.loading.clone()
    }

    pub fn loading(self, loading: bool) -> Self {
        write_flag(&self.loading, loading);
        self
    }

    pub fn props(&self) -> &InfiniteScrollerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentInfiniteScrollerStyle {
        let theme = self.theme.get();
        let ComponentTokens::InfiniteScroller(tokens) =
            theme.component(ComponentId::InfiniteScroller)
        else {
            unreachable!("ComponentId::InfiniteScroller must resolve to InfiniteScrollerTokens")
        };
        FluentInfiniteScrollerStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            loading_indicator_size: tokens.loading_indicator_size,
            footer_padding: tokens.footer_padding,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let viewport = self.viewport;
        let fill = to_paint(style.surface);
        let outline = to_paint(with_alpha(style.outline, 80));
        let ink = to_paint(style.on_surface_variant);
        let loading_height = style.loading_indicator_size + style.footer_padding * 2.0;
        let idle_height = style.footer_padding;
        let content_height = self.content_height.clone();
        let loading = self.loading.clone();
        let on_load_more = self.on_load_more.clone();
        let child = self.child.clone();
        let threshold = self.props.threshold;

        Surface::new()
            .request(LayoutRequest::new().height(viewport).width_pct(100.0))
            .background(fill)
            .border(1.0, outline)
            .push(move || {
                let mut scroller = InfiniteScrollerView::new()
                    .content(
                        child.clone(),
                        content_height.clone(),
                        loading.clone(),
                        loading_height,
                        idle_height,
                        ink,
                    )
                    .threshold(threshold)
                    .viewport(viewport);
                if let Some(on_load_more) = on_load_more.clone() {
                    scroller = scroller.on_load_more(move || on_load_more());
                }
                Box::new(scroller)
            })
    }
}

impl Default for FluentInfiniteScroller {
    fn default() -> Self {
        Self::new()
    }
}
