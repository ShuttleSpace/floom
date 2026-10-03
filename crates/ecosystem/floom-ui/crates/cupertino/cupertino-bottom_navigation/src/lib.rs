use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom::IntoView;
use floom::style::CursorStyle;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::bottom_navigation::BottomNavigationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoBottomNavigationStyle {
    pub design_system: &'static str,
    pub height: f32,
    pub item_min_width: f32,
    pub icon_size: f32,
    pub label_font_size: f32,
    pub background: Color,
    pub active_color: Color,
    pub inactive_color: Color,
    pub divider_color: Color,
}

pub struct CupertinoBottomNavigation {
    props: BottomNavigationProps,
    theme: RwSignal<MultiTheme>,
    selected: RwSignal<usize>,
    icons: Vec<AnyView>,
}

impl CupertinoBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        let selected = props.selected;
        Self {
            props,
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            selected: RwSignal::new(selected),
            icons: Vec::new(),
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
    /// Optional icons paired with `labels` by index.
    pub fn icons(mut self, icons: Vec<AnyView>) -> Self {
        self.icons = icons;
        self
    }
    pub fn props(&self) -> &BottomNavigationProps {
        &self.props
    }
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
    pub fn resolved_style(&self) -> CupertinoBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) =
            theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        CupertinoBottomNavigationStyle {
            design_system: DESIGN_SYSTEM,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
            background: with_alpha(theme.colors().surface, 235),
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            divider_color: with_alpha(theme.colors().on_surface, 30),
        }
    }

    /// Build a renderable Cupertino bottom navigation bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let on_change = Rc::new(self.props.on_change);
        let labels = self.props.labels.clone();
        let icons = self.icons;

        let tokens = match theme.get().component(ComponentId::BottomNavigation) {
            ComponentTokens::BottomNavigation(t) => t,
            _ => unreachable!(),
        };
        let bar_height = tokens.height;
// layout-provider auto-register
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::BottomNavigation,
            bar_height,
        );

        let icon_size = tokens.icon_size;
        let label_font_size = tokens.label_font_size;
        let item_min_width = tokens.item_min_width;
        let gap = tokens.gap;

        let mut icon_iter = icons.into_iter();
        let items: Vec<AnyView> = labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let icon = icon_iter
                    .next()
                    .unwrap_or_else(|| Empty::new().into_any());
                let theme_i = theme;
                let selected_i = selected;
                let on_change_i = on_change.clone();
                let label_i = label;

                Container::new(
                    Stack::vertical((
                        icon.style(move |s| s.size(icon_size, icon_size).items_center().justify_center()),
                        Label::new(label_i.clone()).style(move |s| {
                            let t = theme_i.get();
                            let active = selected_i.get() == i;
                            s.margin_top(4.0)
                                .color(
                                    if active {
                                        t.colors().primary
                                    } else {
                                        t.colors().on_surface_variant
                                    }
                                    .to_peniko(),
                                )
                                .font_size(label_font_size)
                        }),
                    ))
                    .style(move |s| s.items_center().justify_center().gap(gap)),
                )
                .style(move |s| {
                    let _t = theme_i.get();
                    let _active = selected_i.get() == i;
                    s.flex_grow(1.0)
                        .min_width(item_min_width)
                        .height(bar_height)
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::Pointer)
                        .hover(move |s| {
                            let t = theme_i.get();
                            let active = selected_i.get() == i;
                            if active {
                                s
                            } else {
                                s.background(with_alpha(t.colors().on_surface, 12).to_peniko())
                            }
                        })
                })
                .on_event(listener::Click, move |_, _| {
                    selected_i.set(i);
                    if let Some(cb) = on_change_i.as_ref() {
                        cb(i);
                    }
                    EventPropagation::Continue
                })
                .into_any()
            })
            .collect();

        list(items)
            .style(move |s| {
                let t = theme.get();
                s.flex_row()
                .width_pct(100.0)
                    .height(bar_height)
                    .items_center()
                    .background(with_alpha(t.colors().surface, 235).to_peniko())
                    .border_top(0.5)
                    .border_color(with_alpha(t.colors().on_surface, 30).to_peniko())
            })
    }
}

impl Default for CupertinoBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}
