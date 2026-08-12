use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::Decorators;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::tabs::TabsProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdTabsStyle {
    pub design_system: &'static str,
    pub active_color: Color,
    pub inactive_color: Color,
    pub indicator_color: Color,
    pub height: f32,
    pub indicator_height: f32,
    pub padding: f32,
}

pub struct MdTabs {
    props: TabsProps,
    theme: RwSignal<MultiTheme>,
}

impl MdTabs {
    pub fn new() -> Self {
        Self {
            props: TabsProps::default(),
            theme: theme_signal(),
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
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &TabsProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdTabsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tabs(tokens) = theme.component(ComponentId::Tabs) else {
            unreachable!("ComponentId::Tabs must resolve to TabsTokens")
        };
        MdTabsStyle {
            design_system: "material",
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            indicator_color: theme.colors().primary,
            height: tokens.height,
            indicator_height: tokens.indicator_height,
            padding: tokens.padding,
        }
    }

    /// Build a renderable, interactive Material 3 tab bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let on_change: Option<Rc<Box<dyn Fn(usize)>>> = self.props.on_change.map(Rc::new);
        let selected = RwSignal::new(self.props.selected);

        let tokens = match theme.get().component(ComponentId::Tabs) {
            ComponentTokens::Tabs(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let indicator_height = tokens.indicator_height;
        let padding = tokens.padding;

        let rows: Vec<AnyView> = self
            .props
            .tabs
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let i = i;
                let label = label.clone();
                let theme = theme;
                let selected = selected;
                let on_change = on_change.clone();
                let height = height;
                let indicator_height = indicator_height;
                let padding = padding;

                let is_active = move || selected.get() == i;

                v_stack((
                    text(label).style(move |s| {
                        let t = theme.get();
                        let active = is_active();
                        s.padding_horiz(padding)
                            .height(height)
                            .items_center()
                            .justify_center()
                            .color(
                                if active {
                                    t.colors().primary
                                } else {
                                    t.colors().on_surface_variant
                                }
                                .to_peniko(),
                            )
                    }),
                    Container::new(Empty::new()).style(move |s| {
                        let t = theme.get();
                        s.height(indicator_height)
                            .width_pct(100.0)
                            .background(t.colors().primary.to_peniko())
                            .apply_if(!is_active(), |s| s.hide())
                    }),
                ))
                .style(move |s| {
                    s.cursor(CursorStyle::Pointer)
                        .items_center()
                        .apply_if(!is_active(), |s| s.hover(|s| s.background(with_alpha(
                            theme.get().colors().on_surface,
                            8,
                        )
                        .to_peniko())))
                })
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

        list(rows).style(move |s| s.flex_row().height(height))
    }
}

impl Default for MdTabs {
    fn default() -> Self {
        Self::new()
    }
}
