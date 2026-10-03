use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, h_stack_from_iter, v_stack, Stack, Label};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::bottom_navigation::BottomNavigationProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdBottomNavigationStyle {
    pub design_system: &'static str,
    pub bar_color: Color,
    pub selected_color: Color,
    pub unselected_color: Color,
    pub divider_color: Color,
    pub height: f32,
    pub item_min_width: f32,
    pub icon_size: f32,
    pub label_font_size: f32,
}

pub struct MdBottomNavigation {
    props: BottomNavigationProps,
    selected: RwSignal<usize>,
    icons: Vec<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl MdBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        Self {
            selected: RwSignal::new(props.selected),
            icons: Vec::new(),
            theme: theme_signal(),
            props,
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

    /// Optional icons, one per destination, in the same order as `labels`.
    pub fn icons(mut self, icons: Vec<AnyView>) -> Self {
        self.icons = icons;
        self
    }

    pub fn resolved_style(&self) -> MdBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) =
            theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        MdBottomNavigationStyle {
            design_system: "material",
            bar_color: theme.colors().surface,
            selected_color: theme.colors().primary,
            unselected_color: theme.colors().on_surface_variant,
            divider_color: theme.colors().outline_variant,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
        }
    }

    /// Build a renderable Material 3 bottom navigation bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let labels = self.props.labels.clone();
        let icons = self.icons;
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::BottomNavigation) {
            ComponentTokens::BottomNavigation(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
// layout-provider auto-register
        floem_core_theme::layout::register_layout(
            floem_core_theme::layout::LayoutSlot::BottomNavigation,
            height,
        );

        let _icon_size = tokens.icon_size;
        let label_font = tokens.label_font_size;
        let min_width = tokens.item_min_width;
        let gap = tokens.gap;

        let mut icon_iter = icons.into_iter();
        let destinations: Vec<AnyView> = labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let icon = icon_iter.next();
                let selected = selected;
                let on_change = on_change.clone();
                let theme = theme;

                let icon_view: AnyView = match icon {
                    Some(iv) => iv,
                    None => Empty::new().into_any(),
                };

                Stack::vertical((icon_view, Label::new(label)))
                    .style(move |s| {
                        let t = theme.get();
                        let is_sel = selected.get() == i;
                        let c = if is_sel {
                            t.colors().primary
                        } else {
                            t.colors().on_surface_variant
                        };
                        s.items_center()
                            .justify_center()
                            .gap(gap)
                            .width_full()
                            .min_width(min_width)
                            .font_size(label_font)
                            .color(c.to_peniko())
                            .cursor(CursorStyle::Pointer)
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

        Container::new(Stack::horizontal_from_iter(destinations).style(move |s| {
            s.width_full()
                .height_full()
                .items_center()
                .justify_center()
        }))
        .style(move |s| {
            let t = theme.get();
            s.width_full()
                .height(height)
                .background(t.colors().surface.to_peniko())
                .border_color(t.colors().outline_variant.to_peniko())
                .border_top(1.0)
        })
    }
}

impl Default for MdBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}
