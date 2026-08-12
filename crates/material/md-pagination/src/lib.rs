use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::pagination::PaginationProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, PartialEq)]
pub struct MdPaginationStyle {
    pub design_system: &'static str,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub unselected_fg: Color,
    pub button_size: f32,
    pub gap: f32,
    pub radius: f32,
}

pub struct MdPagination {
    props: PaginationProps,
    theme: RwSignal<MultiTheme>,
}

impl MdPagination {
    pub fn new() -> Self {
        Self {
            props: PaginationProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn total(mut self, total: usize) -> Self {
        self.props.total = total;
        self
    }

    pub fn current(mut self, current: usize) -> Self {
        self.props.current = current;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdPaginationStyle {
        let theme = self.theme.get();
        let ComponentTokens::Pagination(tokens) = theme.component(ComponentId::Pagination) else {
            unreachable!("ComponentId::Pagination must resolve to PaginationTokens")
        };
        MdPaginationStyle {
            design_system: "material",
            selected_bg: theme.colors().primary,
            selected_fg: theme.colors().on_primary,
            unselected_fg: theme.colors().on_surface,
            button_size: tokens.button_size,
            gap: tokens.gap,
            radius: theme.radius().md,
        }
    }

    /// Build a Material 3 pagination control (prev / numbered / next).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let total = self.props.total.max(1);
        let current = self.props.current.clamp(1, total);
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Pagination) {
            ComponentTokens::Pagination(t) => t,
            _ => unreachable!(),
        };
        let button_size = tokens.button_size;
        let gap = tokens.gap;
        let radius = theme_radius(&theme);

        let mut controls: Vec<AnyView> = Vec::with_capacity(total + 2);

        // Prev
        {
            let enabled = current > 1;
            let on_change = on_change.clone();
            let btn = Container::new(text("‹".to_string()))
                .style(move |s| {
                    let t = theme.get();
                    let fg = if enabled {
                        t.colors().on_surface
                    } else {
                        with_alpha(t.colors().on_surface, 38)
                    };
                    s.width(button_size)
                        .height(button_size)
                        .items_center()
                        .justify_center()
                        .border_radius(radius)
                        .color(fg.to_peniko())
                        .cursor(if enabled {
                            CursorStyle::Pointer
                        } else {
                            CursorStyle::Default
                        })
                        .apply_if(!enabled, |s| s.set_disabled(true))
                })
                .on_event(listener::Click, move |_, _| {
                    if enabled {
                        let nv = current - 1;
                        if let Some(cb) = on_change.as_ref() {
                            cb(nv);
                        }
                    }
                    EventPropagation::Continue
                })
                .into_any();
            controls.push(btn);
        }

        // Page numbers
        for p in 1..=total {
            let selected = p == current;
            let on_change = on_change.clone();
            let btn = Container::new(text(p.to_string()))
                .style(move |s| {
                    let t = theme.get();
                    let (bg, fg) = if selected {
                        (t.colors().primary, t.colors().on_primary)
                    } else {
                        (Color::rgba(0, 0, 0, 0), t.colors().on_surface)
                    };
                    s.width(button_size)
                        .height(button_size)
                        .items_center()
                        .justify_center()
                        .border_radius(radius)
                        .background(bg.to_peniko())
                        .color(fg.to_peniko())
                        .cursor(CursorStyle::Pointer)
                })
                .on_event(listener::Click, move |_, _| {
                    if let Some(cb) = on_change.as_ref() {
                        cb(p);
                    }
                    EventPropagation::Continue
                })
                .into_any();
            controls.push(btn);
        }

        // Next
        {
            let enabled = current < total;
            let on_change = on_change.clone();
            let btn = Container::new(text("›".to_string()))
                .style(move |s| {
                    let t = theme.get();
                    let fg = if enabled {
                        t.colors().on_surface
                    } else {
                        with_alpha(t.colors().on_surface, 38)
                    };
                    s.width(button_size)
                        .height(button_size)
                        .items_center()
                        .justify_center()
                        .border_radius(radius)
                        .color(fg.to_peniko())
                        .cursor(if enabled {
                            CursorStyle::Pointer
                        } else {
                            CursorStyle::Default
                        })
                        .apply_if(!enabled, |s| s.set_disabled(true))
                })
                .on_event(listener::Click, move |_, _| {
                    if enabled {
                        let nv = current + 1;
                        if let Some(cb) = on_change.as_ref() {
                            cb(nv);
                        }
                    }
                    EventPropagation::Continue
                })
                .into_any();
            controls.push(btn);
        }

        Container::new(list(controls)).style(move |s| s.flex_row().gap(gap).items_center())
    }
}

fn theme_radius(theme: &RwSignal<MultiTheme>) -> f32 {
    theme.get().radius().md
}

impl Default for MdPagination {
    fn default() -> Self {
        Self::new()
    }
}
