use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, dyn_view, h_stack, v_stack, v_stack_from_iter};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::expansion_panels::ExpansionPanelsProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

struct ExpansionPanelItem {
    title: String,
    content: Box<dyn Fn() -> AnyView>,
}

pub struct FluentExpansionPanels {
    props: ExpansionPanelsProps,
    panels: Vec<ExpansionPanelItem>,
    theme: RwSignal<MultiTheme>,
}

impl FluentExpansionPanels {
    pub fn new() -> Self {
        Self {
            props: ExpansionPanelsProps::default(),
            panels: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }

    pub fn panel(mut self, title: impl Into<String>, content: impl Fn() -> AnyView + 'static) -> Self {
        self.panels.push(ExpansionPanelItem {
            title: title.into(),
            content: Box::new(content),
        });
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize, bool) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let multiple = self.props.multiple;
        let on_change = Rc::new(self.props.on_change);
        let panels = self.panels;
        let n = panels.len();

        let tokens = match theme.get().component(ComponentId::ExpansionPanels) {
            ComponentTokens::ExpansionPanels(t) => t,
            _ => unreachable!(),
        };
        let expanded: RwSignal<Vec<bool>> = RwSignal::new(vec![false; n]);
        let tokens_final = tokens.clone();

        let panel_views: Vec<AnyView> = panels
            .into_iter()
            .enumerate()
            .map(move |(i, item)| {
                let theme = theme;
                let tk_h = tokens.clone();
                let tk_c = tokens.clone();
                let expanded = expanded;
                let on_change = on_change.clone();
                let multiple = multiple;
                let title = item.title;
                let content_factory = item.content;

                let glyph = label(move || {
                    if expanded.get()[i] {
                        "▾".to_string()
                    } else {
                        "▸".to_string()
                    }
                })
                .style(move |s| {
                    let t = theme.get();
                    s.color(t.colors().on_surface_variant.to_peniko())
                        .font_size(16.0)
                });

                let header = h_stack((text(title), glyph))
                    .style(move |s| {
                        let t = theme.get();
                        s.height(tk_h.header_height)
                            .padding_horiz(tk_h.content_padding)
                            .items_center()
                            .justify_between()
                            .cursor(CursorStyle::Pointer)
                            .border(1.0)
                            .border_color(with_alpha(t.colors().outline, 30).to_peniko())
                    })
                    .on_event(listener::Click, {
                        let expanded = expanded;
                        let on_change = on_change.clone();
                        move |_, _| {
                            let currently = expanded.get()[i];
                            let new_val = !currently;
                            let mut all = expanded.get();
                            if !multiple {
                                for v in all.iter_mut() {
                                    *v = false;
                                }
                            }
                            all[i] = new_val;
                            expanded.set(all);
                            if let Some(cb) = on_change.as_ref() {
                                cb(i, new_val);
                            }
                            EventPropagation::Continue
                        }
                    });

                let content = dyn_view(move || -> AnyView {
                    if expanded.get()[i] {
                        Container::new(content_factory())
                            .style(move |s| {
                                s.padding_horiz(tk_c.content_padding + tk_c.indent)
                                    .padding_vert(tk_c.content_padding)
                            })
                            .into_any()
                    } else {
                        Empty::new().into_any()
                    }
                });

                v_stack((header, content))
                    .style(|s| s.gap(0.0))
                    .into_any()
            })
            .collect();

        v_stack_from_iter(panel_views)
            .style(move |s| s.gap(tokens_final.gap))
    }
}

impl Default for FluentExpansionPanels {
    fn default() -> Self {
        Self::new()
    }
}
