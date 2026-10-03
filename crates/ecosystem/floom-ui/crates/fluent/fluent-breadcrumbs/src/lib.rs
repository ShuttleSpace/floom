use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Container, Decorators, Empty, Stack, Label};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentBreadcrumbsStyle {
    pub design_system: &'static str,
    pub gap: f32,
    pub font_size: f32,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline_variant: Color,
}

pub struct FluentBreadcrumbs {
    theme: RwSignal<MultiTheme>,
    items: RwSignal<Vec<String>>,
    on_click: Option<Box<dyn Fn(usize)>>,
}

impl FluentBreadcrumbs {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            items: RwSignal::new(Vec::new()),
            on_click: None,
        }
    }

    /// The ordered breadcrumb labels, last item is the current location.
    pub fn items(self, items: Vec<String>) -> Self {
        self.items.set(items);
        self
    }

    /// Fired when a non-current item is clicked, passing its index.
    pub fn on_click(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> FluentBreadcrumbsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Breadcrumbs(tokens) = theme.component(ComponentId::Breadcrumbs) else {
            unreachable!("ComponentId::Breadcrumbs must resolve to BreadcrumbsTokens")
        };
        let c = theme.colors();
        FluentBreadcrumbsStyle {
            design_system: "fluent",
            gap: tokens.gap,
            font_size: tokens.font_size,
            on_surface: c.on_surface,
            on_surface_variant: c.on_surface_variant,
            outline_variant: c.outline_variant,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.items;
        let on_click: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.on_click);

        let tokens = match theme.get().component(ComponentId::Breadcrumbs) {
            ComponentTokens::Breadcrumbs(t) => t,
            _ => unreachable!(),
        };
        let gap = tokens.gap;
        let font_size = tokens.font_size;
        let divider = tokens.divider.clone();

        let child = dyn_view(move || {
            let labels = items.get();
            let theme = theme;
            let on_click = on_click.clone();
            let n = labels.len();

            let mut kids: Vec<AnyView> = Vec::with_capacity(n * 2);
            for (i, label) in labels.iter().enumerate() {
                let is_last = i == n - 1;
                let theme_it = theme;
                let label_view = Label::new(label.clone()).style(move |s| {
                    let t = theme_it.get();
                    let c = if is_last {
                        t.colors().on_surface
                    } else {
                        t.colors().on_surface_variant
                    };
                    let w = if is_last {
                        FontWeight::Medium
                    } else {
                        FontWeight::Regular
                    };
                    s.font_size(font_size)
                        .color(c.to_peniko())
                        .font_weight(w.to_floom())
                });

                let item: AnyView = if is_last {
                    label_view.into_any()
                } else {
                    let theme_k = theme;
                    let on_click_k = on_click.clone();
                    label_view
                        .on_event(listener::Click, move |_, _| {
                            if let Some(cb) = on_click_k.as_ref() {
                                cb(i);
                            }
                            EventPropagation::Continue
                        })
                        .style(move |s| {
                            let _ = theme_k;
                            s.cursor(CursorStyle::Pointer)
                        })
                        .into_any()
                };
                kids.push(item);

                if !is_last {
                    let theme_d = theme;
                    let d = Label::new(divider.clone()).style(move |s| {
                        let t = theme_d.get();
                        s.font_size(font_size)
                            .color(t.colors().outline_variant.to_peniko())
                    });
                    kids.push(d.into_any());
                }
            }

            if kids.is_empty() {
                return Empty::new().into_any();
            }
            Stack::horizontal_from_iter(kids)
                .style(move |s| s.items_center().gap(gap).width_pct(100.0))
                .into_any()
        });

        Container::new(child)
            .style(move |s| s.width_pct(100.0).padding_horiz(8.0).padding_vert(8.0))
    }
}

impl Default for FluentBreadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}
