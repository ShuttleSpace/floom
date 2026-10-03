use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::taffy::style::Overflow;
use floom::ui_events::{ScrollDelta, pointer::PointerScrollEvent};
use floom::view::AnyView;
use floom::views::{dyn_view, Empty, Stack};
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::virtual_scroller::VirtualScrollerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdVirtualScrollerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub border_radius: f32,
    pub item_height: f32,
    pub row_gap: f32,
    pub font_size: f32,
}

pub struct MdVirtualScroller {
    props: VirtualScrollerProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<Box<dyn Fn() -> Box<dyn View>>>,
    height: f32,
}

impl MdVirtualScroller {
    pub fn new() -> Self {
        Self {
            props: VirtualScrollerProps::default(),
            theme: theme_signal(),
            items: Vec::new(),
            height: 0.0,
        }
    }

    /// Item content factories. A factory is invoked per visible window so views are created lazily
    /// (required because `AnyView` is not `Clone` and cannot be re-indexed across scroll updates).
    pub fn items(mut self, items: Vec<Box<dyn Fn() -> Box<dyn View>>>) -> Self {
        self.items = items;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn on_reach_end(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_reach_end = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdVirtualScrollerStyle {
        let theme = self.theme.get();
        let ComponentTokens::VirtualScroller(tokens) = theme.component(ComponentId::VirtualScroller)
        else {
            unreachable!("ComponentId::VirtualScroller must resolve to VirtualScrollerTokens")
        };
        let ih = if self.props.item_height > 0.0 {
            self.props.item_height
        } else {
            tokens.item_height
        };
        MdVirtualScrollerStyle {
            design_system: "material",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            border_radius: 12.0,
            item_height: ih,
            row_gap: tokens.row_gap,
            font_size: tokens.font_size,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.items;
        let on_reach_end: Rc<Option<Box<dyn Fn()>>> = Rc::new(self.props.on_reach_end);

        let tokens = match theme.get().component(ComponentId::VirtualScroller) {
            ComponentTokens::VirtualScroller(t) => t,
            _ => unreachable!(),
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
        let font_size = tokens.font_size;
        let overscan = self.props.overscan;
        let n = items.len();
        let pitch = item_height + row_gap;
        let viewport = if self.height > 0.0 {
            self.height
        } else {
            360.0
        };
        let total = n as f32 * pitch;

        let offset = RwSignal::new(0.0_f32);

        let viewport_view = dyn_view(move || {
            let off = offset.get();
            let start = ((off / pitch).floor() as isize).max(0) as usize;
            let start = start.saturating_sub(overscan);
            let visible = ((viewport / pitch).ceil() as usize) + 1 + overscan * 2;
            let end = (start + visible).min(n);

            let top = (start as f32) * pitch;
            let bottom = ((n - end) as f32).max(0.0) * pitch;

            let mut vis: Vec<AnyView> = Vec::new();
            for i in start..end {
                let factory = &items[i];
                let node: Box<dyn View> = factory();
                let ih = item_height;
                let fs = font_size;
                let row = Container::new(node.into_any())
                    .style(move |s| {
                        s.height(ih)
                            .width_pct(100.0)
                            .items_center()
                            .padding_horiz(12.0)
                            .font_size(fs)
                            .color(theme.get().colors().on_surface.to_peniko())
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                s.background(
                                    with_alpha(theme.get().colors().on_surface, 6).to_peniko(),
                                )
                            })
                    });
                vis.push(row.into_any());
            }

            let top_sp = Empty::new()
                .style(move |s| s.height(top))
                .into_any();
            let bot_sp = Empty::new()
                .style(move |s| s.height(bottom))
                .into_any();
            let body = Stack::vertical_from_iter(vis).style(move |s| s.gap(row_gap));

            Stack::vertical((top_sp, body, bot_sp)).style(|s| s.width_pct(100.0))
        });

        viewport_view
            .style(move |s| {
                let t = theme.get();
                s.height(viewport)
                    .width_pct(100.0)
                    .overflow_x(Overflow::Clip)
                    .overflow_y(Overflow::Clip)
                    .background(t.colors().surface.to_peniko())
                    .border(1.0)
                    .border_color(with_alpha(t.colors().outline, 80).to_peniko())
            })
            .on_event(listener::PointerWheel, move |_, e: &PointerScrollEvent| {
                let dy = match e.delta {
                    ScrollDelta::PixelDelta(v) => v.y as f32,
                    ScrollDelta::LineDelta(_, y) => y * item_height,
                    ScrollDelta::PageDelta(_, y) => y * item_height * 10.0,
                };
                let max = (total - viewport).max(0.0);
                let next = (offset.get() - dy).clamp(0.0, max);
                let at_end = next >= max - 1.0;
                offset.set(next);
                if at_end {
                    if let Some(cb) = on_reach_end.as_ref() {
                        cb();
                    }
                }
                EventPropagation::Continue
            })
    }
}

impl Default for MdVirtualScroller {
    fn default() -> Self {
        Self::new()
    }
}
