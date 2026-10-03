use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::pagination::PaginationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoPaginationStyle {
    pub design_system: &'static str,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub unselected_fg: Color,
    pub button_size: f32,
    pub gap: f32,
    pub radius: f32,
}

pub struct CupertinoPagination {
    props: PaginationProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoPagination {
    pub fn new() -> Self {
        Self {
            props: PaginationProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn props(&self) -> &PaginationProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoPaginationStyle {
        let theme = self.theme.get();
        let ComponentTokens::Pagination(tokens) = theme.component(ComponentId::Pagination) else {
            unreachable!("ComponentId::Pagination must resolve to PaginationTokens")
        };
        CupertinoPaginationStyle {
            design_system: DESIGN_SYSTEM,
            selected_bg: theme.colors().primary,
            selected_fg: theme.colors().on_primary,
            unselected_fg: theme.colors().on_surface,
            button_size: tokens.button_size,
            gap: tokens.gap,
            radius: theme.radius().md,
        }
    }

    /// Build a Cupertino pagination control (prev / numbered / next).
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
        let radius = theme.get().radius().md;

        let mut controls: Vec<AnyView> = Vec::with_capacity(total + 2);

        // Prev
        {
            let enabled = current > 1;
            let on_change = on_change.clone();
            let btn = Container::new(Label::new("‹".to_string()))
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
            let btn = Container::new(Label::new(p.to_string()))
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
            let btn = Container::new(Label::new("›".to_string()))
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

impl Default for CupertinoPagination {
    fn default() -> Self {
        Self::new()
    }
}
