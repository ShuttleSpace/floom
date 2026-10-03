use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::taffy::style::Overflow;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::slide_group::SlideGroupProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;


const ITEM_W: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSlideGroupStyle {
    pub design_system: &'static str,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub outline: Color,
    pub height: f32,
    pub gap: f32,
    pub active_indicator_height: f32,
    pub arrow_size: f32,
}

pub struct FluentSlideGroup {
    props: SlideGroupProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<AnyView>,
    selected: usize,
    on_change: Option<Box<dyn Fn(usize)>>,
}

impl FluentSlideGroup {
    pub fn new() -> Self {
        Self {
            props: SlideGroupProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            items: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }

    pub fn items(mut self, items: Vec<AnyView>) -> Self {
        self.items = items;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> FluentSlideGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::SlideGroup(tokens) =
            theme.component(ComponentId::SlideGroup)
        else {
            unreachable!("ComponentId::SlideGroup must resolve to SlideGroupTokens")
        };
        FluentSlideGroupStyle {
            design_system: "fluent",
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            outline: theme.colors().outline,
            height: tokens.height,
            gap: tokens.gap,
            active_indicator_height: tokens.active_indicator_height,
            arrow_size: tokens.arrow_size,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.items;
        let n = items.len();
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.on_change);
        let show_arrows = self.props.show_arrows;
        let mandatory = self.props.mandatory;

        let tokens = match theme.get().component(ComponentId::SlideGroup) {
            ComponentTokens::SlideGroup(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let gap = tokens.gap;
        let indicator = tokens.active_indicator_height;
        let arrow_size = tokens.arrow_size;

        let selected = RwSignal::new(self.selected.min(n.saturating_sub(1)));

        let row: Vec<AnyView> = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let selected = selected;
                let on_change = on_change.clone();
                let mandatory = mandatory;
                Container::new(item)
                    .style(move |s| {
                        let active = i == selected.get();
                        let t = theme.get();
                        let ind = if active {
                            t.colors().primary.to_peniko()
                        } else {
                            with_alpha(t.colors().outline, 0).to_peniko()
                        };
                        s.height(height)
                            .min_width(ITEM_W)
                            .padding_horiz(gap)
                            .items_center()
                            .justify_center()
                            .border_bottom(indicator)
                            .border_color(ind)
                            .cursor(CursorStyle::Pointer)
                    })
                    .on_event(listener::Click, move |_, _| {
                        let cur = selected.get();
                        if mandatory && cur == i {
                            return EventPropagation::Continue;
                        }
                        selected.set(i);
                        if let Some(cb) = on_change.as_ref() {
                            cb(i);
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        let row_stack =
            Stack::horizontal_from_iter(row).style(move |s| s.items_center().gap(0.0).height(height));

        let mut area_children: Vec<AnyView> = Vec::new();
        area_children.push(row_stack.into_any());

        if show_arrows && n > 1 {
            let selected = selected;
            let prev = Container::new(
                Label::new("‹").style(move |s| {
                    s.font_size(arrow_size * 0.6)
                        .color(theme.get().colors().on_surface_variant.to_peniko())
                }),
            )
            .style(move |s| {
                s.absolute()
                    .inset_left_pct(0.0)
                    .inset_top_pct(0.0)
                    .height_pct(100.0)
                    .width(32.0)
                    .items_center()
                    .background(with_alpha(theme.get().colors().surface, 220).to_peniko())
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                let cur = selected.get();
                if cur > 0 {
                    selected.set(cur - 1);
                }
                EventPropagation::Continue
            })
            .into_any();
            area_children.push(prev);

            let selected = selected;
            let next = Container::new(
                Label::new("›").style(move |s| {
                    s.font_size(arrow_size * 0.6)
                        .color(theme.get().colors().on_surface_variant.to_peniko())
                }),
            )
            .style(move |s| {
                s.absolute()
                    .inset_right_pct(0.0)
                    .inset_top_pct(0.0)
                    .height_pct(100.0)
                    .width(32.0)
                    .items_center()
                    .background(with_alpha(theme.get().colors().surface, 220).to_peniko())
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                let cur = selected.get();
                if cur < n - 1 {
                    selected.set(cur + 1);
                }
                EventPropagation::Continue
            })
            .into_any();
            area_children.push(next);
        }

        Stack::new(area_children).style(move |s| {
            s.width_pct(100.0)
                .height(height)
                .overflow_x(Overflow::Clip)
                .overflow_y(Overflow::Clip)
        })
    }
}

impl Default for FluentSlideGroup {
    fn default() -> Self {
        Self::new()
    }
}
