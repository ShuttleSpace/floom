//! Fluent virtual scroller, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::VirtualScroller;
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::virtual_scroller::VirtualScrollerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

pub type RowBuilder = floom_ui_composites::WidgetBuilder;

const DEFAULT_VIEWPORT: f32 = 360.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentVirtualScrollerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub border_radius: f32,
    pub item_height: f32,
    pub row_gap: f32,
    pub font_size: f32,
    pub viewport_height: f32,
    pub overscan: usize,
}

pub struct FluentVirtualScroller {
    props: VirtualScrollerProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<RowBuilder>,
    height: f32,
    on_reach_end: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl FluentVirtualScroller {
    pub fn new() -> Self {
        Self {
            props: VirtualScrollerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            items: Vec::new(),
            height: 0.0,
            on_reach_end: None,
        }
    }

    pub fn items(mut self, items: Vec<RowBuilder>) -> Self {
        self.items = items;
        self
    }

    pub fn item(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.items.push(Arc::new(build));
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn item_height(mut self, height: f32) -> Self {
        self.props.item_height = height;
        self
    }

    pub fn row_gap(mut self, gap: f32) -> Self {
        self.props.row_gap = gap;
        self
    }

    pub fn overscan(mut self, overscan: usize) -> Self {
        self.props.overscan = overscan;
        self
    }

    pub fn on_reach_end(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_reach_end = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &VirtualScrollerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentVirtualScrollerStyle {
        let theme = self.theme.get();
        let ComponentTokens::VirtualScroller(tokens) =
            theme.component(ComponentId::VirtualScroller)
        else {
            unreachable!("ComponentId::VirtualScroller must resolve to VirtualScrollerTokens")
        };
        let item_height = if self.props.item_height > 0.0 {
            self.props.item_height
        } else {
            tokens.item_height
        };
        let row_gap = if self.props.row_gap > 0.0 {
            self.props.row_gap
        } else {
            tokens.row_gap
        };
        FluentVirtualScrollerStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            border_radius: 12.0,
            item_height,
            row_gap,
            font_size: tokens.font_size,
            viewport_height: if self.height > 0.0 {
                self.height
            } else {
                DEFAULT_VIEWPORT
            },
            overscan: self.props.overscan,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let fill = to_paint(style.surface);
        let outline = to_paint(with_alpha(style.outline, 80));
        let rows = self.items;
        let viewport = style.viewport_height;
        let overscan = style.overscan;
        let item_height = style.item_height;
        let row_gap = style.row_gap;
        let on_reach_end = self.on_reach_end.clone();

        Surface::new()
            .request(LayoutRequest::new().height(viewport).width_pct(100.0))
            .background(fill)
            .border(1.0, outline)
            .push(move || {
                let mut scroller = VirtualScroller::new()
                    .rows(rows.clone())
                    .item_height(item_height)
                    .row_gap(row_gap)
                    .viewport(viewport)
                    .overscan(overscan);
                if let Some(on_reach_end) = on_reach_end.clone() {
                    scroller = scroller.on_reach_end(move || on_reach_end());
                }
                Box::new(scroller)
            })
    }
}

impl Default for FluentVirtualScroller {
    fn default() -> Self {
        Self::new()
    }
}
