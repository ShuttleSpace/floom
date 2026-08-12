use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::taffy::style::Overflow;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::window::WindowProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdWindowStyle {
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

pub struct MdWindow {
    props: WindowProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<AnyView>,
    height: f32,
    selected: usize,
    on_change: Option<Box<dyn Fn(usize)>>,
}

impl MdWindow {
    pub fn new() -> Self {
        Self {
            props: WindowProps::default(),
            theme: theme_signal(),
            items: Vec::new(),
            height: 0.0,
            selected: 0,
            on_change: None,
        }
    }

    pub fn items(mut self, items: Vec<AnyView>) -> Self {
        self.items = items;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
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

    pub fn show_dots(mut self, show_dots: bool) -> Self {
        self.props.show_dots = show_dots;
        self
    }

    pub fn continuous(mut self, continuous: bool) -> Self {
        self.props.continuous = continuous;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdWindowStyle {
        let theme = self.theme.get();
        let ComponentTokens::Window(tokens) = theme.component(ComponentId::Window) else {
            unreachable!("ComponentId::Window must resolve to WindowTokens")
        };
        let h = if self.height > 0.0 {
            self.height
        } else {
            tokens.height
        };
        MdWindowStyle {
            design_system: "material",
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
        let items = self.items;
        let n = items.len();
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.on_change);
        let show_arrows = self.props.show_arrows;
        let show_dots = self.props.show_dots;
        let continuous = self.props.continuous;

        let tokens = match theme.get().component(ComponentId::Window) {
            ComponentTokens::Window(t) => t,
            _ => unreachable!(),
        };
        let dot_size = tokens.dot_size;
        let dot_gap = 6.0f32;
        let dot_inactive = 0.4f32;
        let arrow_size = tokens.arrow_size;
        let border_radius = tokens.border_radius;
        let height = if self.height > 0.0 {
            self.height
        } else {
            tokens.height
        };

        let index = RwSignal::new(self.selected.min(n.saturating_sub(1)));

        let mut children: Vec<AnyView> = Vec::new();
        for (i, item) in items.into_iter().enumerate() {
            let index = index;
            children.push(
                item.style(move |s| {
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
            let prev = Container::new(
                text("‹").style(move |s| {
                    s.font_size(arrow_size * 0.6)
                        .color(theme.get().colors().on_surface.to_peniko())
                }),
            )
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
                let next = if cur == 0 {
                    if continuous {
                        n - 1
                    } else {
                        0
                    }
                } else {
                    cur - 1
                };
                index.set(next);
                if let Some(cb) = on_change_prev.as_ref() {
                    cb(next);
                }
                EventPropagation::Continue
            })
            .into_any();
            children.push(prev);

            let index = index;
            let on_change_next = on_change.clone();
            let next = Container::new(
                text("›").style(move |s| {
                    s.font_size(arrow_size * 0.6)
                        .color(theme.get().colors().on_surface.to_peniko())
                }),
            )
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
                let next = if cur >= n - 1 {
                    if continuous {
                        0
                    } else {
                        n - 1
                    }
                } else {
                    cur + 1
                };
                index.set(next);
                if let Some(cb) = on_change_next.as_ref() {
                    cb(next);
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
            let dots_row =
                h_stack_from_iter(dots).style(|s| s.justify_center().items_center().gap(0.0));
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

        Stack::new(children).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height(height)
                .border_radius(border_radius)
                .overflow_x(Overflow::Clip)
                .overflow_y(Overflow::Clip)
                .background(t.colors().surface.to_peniko())
                .border(1.0)
                .border_color(with_alpha(t.colors().outline, 80).to_peniko())
        })
    }
}

impl Default for MdWindow {
    fn default() -> Self {
        Self::new()
    }
}
