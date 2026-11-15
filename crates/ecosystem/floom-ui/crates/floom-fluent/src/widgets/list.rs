//! Fluent list, ported to the Floom 2.0 widget API.

use floom_core::Axis;
use floom_core::LayoutRequest;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::list::ListProps;
use floom_core_theme::props::menu::MenuItem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ListItemRow, ListItemStyle};
use floom_widget_runtime::theme_signal;
use floom_widgets::compose::Stack;
use floom_widgets::primitives::{Container, Spacer};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct FluentRowGeometry {
    pub height: f32,
    pub padding_horizontal: f32,
    pub gap: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentListStyle {
    pub design_system: &'static str,
    pub surface_color: Color,
    pub text_color: Color,
    pub item_height: f32,
    pub padding: f32,
    pub padding_horizontal: f32,
    pub min_width: f32,
    pub gap: f32,
    pub hover_color: Color,
    pub border_radius: f32,
    pub label_font_size: f32,
}

const DESIGN_SYSTEM: &str = "fluent";

pub struct FluentList {
    props: ListProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentList {
    pub fn new() -> Self {
        Self {
            props: ListProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn item(
        mut self,
        label: impl Into<String>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Self {
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
            padding_horizontal: tk.padding_horizontal,
            min_width: tk.min_width,
            gap: tk.gap,
            hover_color: with_alpha(theme.colors().primary, 8),
            border_radius: theme.radius().md,
            label_font_size: theme.typography().body.medium.font_size,
        }
    }

    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let padding = style.padding;
        let gap = style.gap;
        let (surface, radius, min_width) = (
            to_paint(style.surface_color),
            style.border_radius,
            style.min_width,
        );
        let items = self.props.items;

        let mut stack = Stack::new(Axis::Vertical)
            .gap(gap)
            .request(LayoutRequest::new().min_width(min_width))
            .push_layout(LayoutRequest::new().absolute_fill(), move || {
                Box::new(Container::new().background(surface).radius(radius))
            })
            .push_layout(LayoutRequest::new().height(padding), move || {
                Box::new(Spacer::new(padding as f64).axis(Axis::Vertical))
            });

        for item in items {
            let on_select = Arc::new(item.on_select);
            let title = Arc::new(item.label);
            let geometry = FluentRowGeometry {
                height: style.item_height,
                padding_horizontal: style.padding_horizontal,
                gap,
            };
            stack = stack.push(move || {
                let on_select = on_select.clone();
                let title = title.clone();
                Box::new(
                    ListItemRow::new(title.as_str())
                        .style(FluentList::row_style(geometry))
                        .on_click(move || {
                            if let Some(cb) = on_select.as_ref() {
                                cb();
                            }
                        }),
                )
            });
        }

        stack.push_layout(LayoutRequest::new().height(padding), move || {
            Box::new(Spacer::new(padding as f64).axis(Axis::Vertical))
        })
    }

    fn row_style(geometry: FluentRowGeometry) -> ListItemStyle {
        ListItemStyle {
            min_height: geometry.height,
            padding_horizontal: geometry.padding_horizontal as f64,
            gap: geometry.gap,
            ..ListItemStyle::default()
        }
    }
}

impl Default for FluentList {
    fn default() -> Self {
        Self::new()
    }
}
