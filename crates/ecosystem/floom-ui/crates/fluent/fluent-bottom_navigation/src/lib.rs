use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::IntoView;
use floom::views::{Decorators, Empty, Label, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::bottom_navigation::BottomNavigationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentBottomNavigationStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub selected_color: Color,
    pub unselected_color: Color,
    pub height: f32,
    pub item_min_width: f32,
    pub icon_size: f32,
    pub label_font_size: f32,
}

pub struct FluentBottomNavigation {
    props: BottomNavigationProps,
    icons: Vec<AnyView>,
    selected: RwSignal<usize>,
    theme: RwSignal<MultiTheme>,
}

impl FluentBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        let selected = props.selected;
        Self {
            props,
            icons: Vec::new(),
            selected: RwSignal::new(selected),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.props.labels = labels;
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
    /// Optional icons (one per destination, in order) shown above each label.
    pub fn icons(mut self, icons: Vec<AnyView>) -> Self {
        self.icons = icons;
        self
    }
    pub fn props(&self) -> &BottomNavigationProps {
        &self.props
    }
    /// Expose the internal selection signal so external code can read/set it.
    pub fn selected_signal(&self) -> RwSignal<usize> {
        self.selected
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) =
            theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        FluentBottomNavigationStyle {
            design_system: DESIGN_SYSTEM,
            background: theme.colors().surface,
            selected_color: theme.colors().primary,
            unselected_color: theme.colors().on_surface_variant,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
        }
    }

    /// Build a renderable Fluent bottom navigation bar. Tapping a destination
    /// updates the internal selection signal and calls `on_change`.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let on_change = Rc::new(self.props.on_change);
        let labels = self.props.labels.clone();
        let mut icons = self.icons;

        let tokens = match theme.get().component(ComponentId::BottomNavigation) {
            ComponentTokens::BottomNavigation(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
// layout-provider auto-register
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::BottomNavigation,
            height,
        );

        let icon_size = tokens.icon_size;
        let label_fs = tokens.label_font_size;
        let gap = tokens.gap;

        let items: Vec<AnyView> = labels
            .iter()
            .enumerate()
            .map(|(i, lbl)| {
                let icon: AnyView = if !icons.is_empty() {
                    icons.remove(0)
                } else {
                    Empty::new().into_any()
                };
                let selected = selected;
                let on_change = on_change.clone();
                let icon_size = icon_size;
                let label_fs = label_fs;
                let lbl = lbl.clone();

                let item = Stack::vertical((
                    Container::new(icon).style(move |s| {
                        s.width(icon_size).height(icon_size).items_center().justify_center()
                    }),
                    Label::new(lbl).style(move |s| {
                        let t = theme.get();
                        let sel = selected.get() == i;
                        s.font_size(label_fs).color(if sel {
                            t.colors().primary.to_peniko()
                        } else {
                            t.colors().on_surface_variant.to_peniko()
                        })
                    }),
                ))
                .style(move |s| {
                    let t = theme.get();
                    let sel = selected.get() == i;
                    s.flex()
                        .items_center()
                        .justify_center()
                        .gap(gap)
                        .padding_vert(8.0)
                        .cursor(CursorStyle::Pointer)
                        .apply_if(sel, |s| {
                            s.border_top(2.0).border_color(t.colors().primary.to_peniko())
                        })
                })
                .on_event(listener::Click, move |_, _| {
                    selected.set(i);
                    if let Some(cb) = on_change.as_ref() {
                        cb(i);
                    }
                    EventPropagation::Continue
                });
                item.into_any()
            })
            .collect();

        Container::new(list(items)).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height(height)
                .flex_row()
                .items_center()
                .background(t.colors().surface.to_peniko())
                .border_top(1.0)
                .border_color(with_alpha(t.colors().on_surface, 12).to_peniko())
        })
    }
}

impl Default for FluentBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}
