use floom::event::EventPropagation;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Decorators, Label};
use floom::{
    prelude::*,
    reactive::RwSignal,
};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::list::ListProps;
use floom_core_theme::props::menu::MenuItem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoListStyle {
    pub design_system: &'static str,
    pub surface_color: Color,
    pub text_color: Color,
    pub hover_color: Color,
    pub item_height: f32,
    pub padding: f32,
    pub min_width: f32,
    pub gap: f32,
}

pub struct CupertinoList {
    props: ListProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoList {
    pub fn new() -> Self {
        Self {
            props: ListProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }
    /// Append a clickable list item.
    pub fn item(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
        self.props.items.push(MenuItem {
            label: label.into(),
            on_select: Some(Box::new(callback)),
        });
        self
    }
    pub fn items(mut self, items: Vec<MenuItem>) -> Self {
        self.props.items = items;
        self
    }
    pub fn props(&self) -> &ListProps {
        &self.props
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoListStyle {
        let theme = self.theme.get();
        let ComponentTokens::List(tokens) = theme.component(ComponentId::List) else {
            unreachable!("ComponentId::List must resolve to ListTokens")
        };
        CupertinoListStyle {
            design_system: DESIGN_SYSTEM,
            surface_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            hover_color: theme.colors().primary,
            item_height: tokens.item_height,
            padding: tokens.padding,
            min_width: tokens.min_width,
            gap: tokens.gap,
        }
    }

    /// Build a renderable Cupertino list of clickable rows.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.props.items;

        let tokens = match theme.get().component(ComponentId::List) {
            ComponentTokens::List(t) => t,
            _ => unreachable!(),
        };
        let item_height = tokens.item_height;
        let padding = tokens.padding;
        let min_width = tokens.min_width;
        let gap = tokens.gap;
        let padding_horizontal = tokens.padding_horizontal;

        let rows: Vec<AnyView> = items
            .into_iter()
            .map(|item| {
                let on_select = item.on_select;
                let label = item.label.clone();
                let theme = theme;
                Label::new(label)
                    .style(move |s| {
                        let t = theme.get();
                        s.width_pct(100.0)
                            .height(item_height)
                            .padding_horiz(padding_horizontal)
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(t.typography().body.medium.font_size)
                            .items_center()
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                let t = theme.get();
                                s.background(with_alpha(t.colors().primary, 8).to_peniko())
                            })
                    })
                    .on_event(listener::Click, move |_, _| {
                        if let Some(cb) = &on_select {
                            cb();
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        Container::new(list(rows)).style(move |s| {
            let t = theme.get();
            s.min_width(min_width)
                .padding(padding)
                .background(t.colors().surface.to_peniko())
                .border_radius(t.radius().md)
                .gap(gap)
        })
    }
}

impl Default for CupertinoList {
    fn default() -> Self {
        Self::new()
    }
}
