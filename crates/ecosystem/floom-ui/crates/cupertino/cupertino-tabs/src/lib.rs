use std::rc::Rc;

use floom::event::EventPropagation;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Decorators, Empty, Label, Stack};
use floom::{
    prelude::*,
    reactive::RwSignal,
};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tabs::TabsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTabsStyle {
    pub design_system: &'static str,
    pub active_color: Color,
    pub inactive_color: Color,
    pub indicator_color: Color,
    pub height: f32,
    pub indicator_height: f32,
    pub padding: f32,
}

pub struct CupertinoTabs {
    props: TabsProps,
    theme: RwSignal<MultiTheme>,
    selected: RwSignal<usize>,
}

impl CupertinoTabs {
    pub fn new() -> Self {
        let props = TabsProps::default();
        let selected = props.selected;
        Self {
            props,
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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
    pub fn resolved_style(&self) -> CupertinoTabsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tabs(tokens) = theme.component(ComponentId::Tabs) else {
            unreachable!("ComponentId::Tabs must resolve to TabsTokens")
        };
        CupertinoTabsStyle {
            design_system: DESIGN_SYSTEM,
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            indicator_color: theme.colors().primary,
            height: tokens.height,
            indicator_height: tokens.indicator_height,
            padding: tokens.padding,
        }
    }

    /// Build a renderable, interactive Cupertino tab bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let on_change = Rc::new(self.props.on_change);
        let tabs = self.props.tabs.clone();

        let tokens = match theme.get().component(ComponentId::Tabs) {
            ComponentTokens::Tabs(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let indicator_height = tokens.indicator_height;
        let padding = tokens.padding;

        let tab_views: Vec<AnyView> = tabs
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let selected = selected;
                let on_change = on_change.clone();
                let theme = theme;

                let tab = Stack::vertical((
                    Label::new(label).style(move |s| {
                        let t = theme.get();
                        let active = selected.get() == i;
                        s.padding_horiz(padding)
                            .height(height)
                            .items_center()
                            .color(
                                if active {
                                    t.colors().primary
                                } else {
                                    t.colors().on_surface_variant
                                }
                                .to_peniko(),
                            )
                            .font_weight(
                                if active {
                                    t.typography().label.large.font_weight.to_floom()
                                } else {
                                    FontWeight::Regular.to_floom()
                                },
                            )
                            .cursor(CursorStyle::Pointer)
                    }),
                    Container::new(Empty::new()).style(move |s| {
                        let t = theme.get();
                        s.width_pct(100.0)
                            .height(indicator_height)
                            .background(t.colors().primary.to_peniko())
                            .apply_if(selected.get() != i, |s| s.hide())
                    }),
                ))
                .style(|s| s.items_center())
                .on_event(listener::Click, move |_, _| {
                    selected.set(i);
                    if let Some(cb) = on_change.as_ref() {
                        cb(i);
                    }
                    EventPropagation::Continue
                });

                tab.into_any()
            })
            .collect();

        Container::new(Stack::horizontal_from_iter(tab_views)).style(move |s| {
            let t = theme.get();
            s.height(height)
                .background(t.colors().surface.to_peniko())
                .items_center()
        })
    }
}

impl Default for CupertinoTabs {
    fn default() -> Self {
        Self::new()
    }
}
