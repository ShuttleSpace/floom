use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::Decorators;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::list::ListProps;
use floem_core_theme::props::menu::MenuItem;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentListStyle {
    pub design_system: &'static str,
    pub surface_color: Color,
    pub text_color: Color,
    pub item_height: f32,
    pub padding: f32,
    pub min_width: f32,
    pub gap: f32,
}

pub struct FluentList {
    props: ListProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentList {
    pub fn new() -> Self {
        Self {
            props: ListProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }
    /// Append a clickable item to the list.
    pub fn item(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
        self.props.items.push(MenuItem {
            label: label.into(),
            on_select: Some(Box::new(callback)),
        });
        self
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentListStyle {
        let theme = self.theme.get();
        let ComponentTokens::List(tk) = theme.component(ComponentId::List) else {
            unreachable!("ComponentId::List must resolve to ListTokens")
        };
        FluentListStyle {
            design_system: DESIGN_SYSTEM,
            surface_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            item_height: tk.item_height,
            padding: tk.padding,
            min_width: tk.min_width,
            gap: tk.gap,
        }
    }

    /// Build a renderable Fluent list of clickable rows.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.props.items;

        let rows: Vec<AnyView> = items
            .into_iter()
            .map(|item| {
                let theme = theme;
                let on_select = Rc::new(item.on_select);
                let label = item.label.clone();

                text(label)
                    .style(move |s| {
                        let t = theme.get();
                        let ComponentTokens::List(tk) = t.component(ComponentId::List) else {
                            unreachable!()
                        };
                        let theme_hover = theme;
                        s.width_full()
                            .height(tk.item_height)
                            .padding_horiz(tk.padding)
                            .items_center()
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(t.typography().body.medium.font_size)
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                let t = theme_hover.get();
                                s.background(with_alpha(t.colors().primary, 8).to_peniko())
                            })
                    })
                    .on_event(listener::Click, move |_, _| {
                        if let Some(cb) = on_select.as_ref() {
                            cb();
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        Container::new(list(rows)).style(move |s| {
            let t = theme.get();
            let ComponentTokens::List(tk) = t.component(ComponentId::List) else {
                unreachable!()
            };
            s.min_width(tk.min_width)
                .padding(tk.padding)
                .gap(tk.gap)
                .background(t.colors().surface.to_peniko())
                .border_radius(t.radius().md)
        })
    }
}

impl Default for FluentList {
    fn default() -> Self {
        Self::new()
    }
}
