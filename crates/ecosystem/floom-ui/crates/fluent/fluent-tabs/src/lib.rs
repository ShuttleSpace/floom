use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Decorators, Empty, Label, Stack};
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tabs::TabsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentTabsStyle {
    pub design_system: &'static str,
    pub active_color: Color,
    pub inactive_color: Color,
    pub height: f32,
    pub indicator_height: f32,
    pub padding: f32,
}

pub struct FluentTabs {
    props: TabsProps,
    theme: RwSignal<MultiTheme>,
    selected: RwSignal<usize>,
}

impl FluentTabs {
    pub fn new() -> Self {
        let props = TabsProps::default();
        let selected = props.selected;
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            props,
            selected: RwSignal::new(selected),
        }
    }
    pub fn tab(mut self, tab: impl Into<String>) -> Self {
        self.props.tabs.push(tab.into());
        self
    }
    pub fn tabs(mut self, tabs: Vec<String>) -> Self {
        self.props.tabs = tabs;
        self
    }
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = selected;
        self.selected.set(selected);
        self
    }
    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &TabsProps {
        &self.props
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentTabsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tabs(tk) = theme.component(ComponentId::Tabs) else {
            unreachable!("ComponentId::Tabs must resolve to TabsTokens")
        };
        FluentTabsStyle {
            design_system: DESIGN_SYSTEM,
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            height: tk.height,
            indicator_height: tk.indicator_height,
            padding: tk.padding,
        }
    }

    /// Build a renderable, interactive Fluent tab bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let on_change = Rc::new(self.props.on_change);
        let tabs = self.props.tabs;

        let rows: Vec<AnyView> = tabs
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let theme = theme;
                let selected = selected;
                let on_change = on_change.clone();
                let label = label.clone();

                let label_view = Label::new(label).style(move |s| {
                    let t = theme.get();
                    let ComponentTokens::Tabs(tk) = t.component(ComponentId::Tabs) else {
                        unreachable!()
                    };
                    let active = selected.get() == i;
                    s.height(tk.height)
                        .items_center()
                        .justify_center()
                        .padding_horiz(tk.padding)
                        .color(
                            (if active {
                                t.colors().primary
                            } else {
                                t.colors().on_surface_variant
                            })
                            .to_peniko(),
                        )
                        .font_weight(
                            (if active {
                                FontWeight::SemiBold
                            } else {
                                FontWeight::Medium
                            })
                            .to_floom(),
                        )
                });

                let indicator = Container::new(Empty::new()).style(move |s| {
                    let t = theme.get();
                    let ComponentTokens::Tabs(tk) = t.component(ComponentId::Tabs) else {
                        unreachable!()
                    };
                    let active = selected.get() == i;
                    s.width_full()
                        .height(tk.indicator_height)
                        .background(t.colors().primary.to_peniko())
                        .apply_if(!active, |s| s.hide())
                });

                Stack::vertical((label_view, indicator))
                    .style(|s| s.items_center().cursor(CursorStyle::Pointer))
                    .on_event(listener::Click, move |_, _| {
                        selected.set(i);
                        if let Some(cb) = on_change.as_ref() {
                            cb(i);
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        Container::new(list(rows)).style(move |s| {
            let ComponentTokens::Tabs(tk) = theme.get().component(ComponentId::Tabs) else {
                unreachable!()
            };
            s.width_full().height(tk.height + tk.indicator_height)
        })
    }
}

impl Default for FluentTabs {
    fn default() -> Self {
        Self::new()
    }
}
