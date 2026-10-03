use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom_widget_cupertino_icon::cupertino_icon_svg;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{svg, Container, Decorators, Empty, dyn_view, Stack, Label};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::list_group::ListGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;

pub struct CupertinoListGroup {
    props: ListGroupProps,
    items: Vec<String>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoListGroup {
    pub fn new() -> Self {
        Self {
            props: ListGroupProps::default(),
            items: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn item(mut self, label: impl Into<String>) -> Self {
        self.items.push(label.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = Some(title.into());
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.props.expanded = expanded;
        self
    }

    pub fn inset(mut self, inset: bool) -> Self {
        self.props.inset = inset;
        self
    }

    pub fn on_toggle(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.props.on_toggle = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.items;
        let title = self.props.title.clone();
        let inset = self.props.inset;
        let on_toggle = Rc::new(self.props.on_toggle);
        let expanded: RwSignal<bool> = RwSignal::new(self.props.expanded);

        let tokens = match theme.get().component(ComponentId::ListGroup) {
            ComponentTokens::ListGroup(t) => t,
            _ => unreachable!(),
        };

        let tk_header = tokens.clone();
        let tk_items = tokens.clone();
        let tk_gap = tokens.clone();
        let icon_size = tokens.icon_size;

        let header_title = title.clone().unwrap_or_default();
        let glyph = svg(move || {
            if expanded.get() {
                cupertino_icon_svg("expand_more")
            } else {
                cupertino_icon_svg("chevron_right")
            }
        })
        .style(move |s| {
            let t = theme.get();
            s.size(icon_size, icon_size).color(t.colors().on_surface_variant.to_peniko())
        });

        let header = Stack::horizontal((Label::new(header_title), glyph))
            .style(move |s| {
                let t = theme.get();
                s.height(tk_header.header_height)
                    .padding_horiz(tk_header.content_padding)
                    .items_center()
                    .justify_between()
                    .cursor(CursorStyle::Pointer)
                    .border(1.0)
                    .border_color(with_alpha(t.colors().outline, 30).to_peniko())
            })
            .on_event(listener::Click, {
                let expanded = expanded;
                let on_toggle = on_toggle.clone();
                move |_, _| {
                    let new_val = !expanded.get();
                    expanded.set(new_val);
                    if let Some(cb) = on_toggle.as_ref() {
                        cb(new_val);
                    }
                    EventPropagation::Continue
                }
            });

        let items_view = dyn_view(move || -> AnyView {
            if expanded.get() {
                let item_views: Vec<AnyView> = items
                    .clone()
                    .into_iter()
                    .map(|label| {
                        let theme = theme;
                        let tk = tk_items.clone();
                        let pad = if inset { tk.indent } else { tk.content_padding };
                        Container::new(Label::new(label))
                            .style(move |s| {
                                let t = theme.get();
                                s.height(tk.header_height)
                                    .padding_horiz(pad)
                                    .items_center()
                                    .color(t.colors().on_surface.to_peniko())
                                    .font_size(t.typography().body.medium.font_size)
                            })
                            .into_any()
                    })
                    .collect();
                Stack::vertical_from_iter(item_views)
                    .style(move |s| s.gap(tk_gap.gap).padding_vert(tk_gap.content_padding))
                    .into_any()
            } else {
                Empty::new().into_any()
            }
        });

        Stack::vertical((header, items_view)).style(|s| s.gap(0.0))
    }
}

impl Default for CupertinoListGroup {
    fn default() -> Self {
        Self::new()
    }
}
