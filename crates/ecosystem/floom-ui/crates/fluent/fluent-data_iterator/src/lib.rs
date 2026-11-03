//! Fluent data iterator, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Axis, Divider, RuleAlignment, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, DataIteratorTokens};
use floom_core_theme::props::data_iterator::DataIteratorProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

pub type ItemBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentDataIteratorStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub outline: Color,
    pub padding: f32,
    pub item_gap: f32,
    pub divided: bool,
}

pub struct FluentDataIterator {
    props: DataIteratorProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<ItemBuilder>,
    header: Option<ItemBuilder>,
    footer: Option<ItemBuilder>,
}

impl FluentDataIterator {
    pub fn new() -> Self {
        Self {
            props: DataIteratorProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            items: Vec::new(),
            header: None,
            footer: None,
        }
    }

    pub fn item(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.items.push(Arc::new(build));
        self
    }

    pub fn header(
        mut self,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.header = Some(Arc::new(build));
        self
    }

    pub fn footer(
        mut self,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.footer = Some(Arc::new(build));
        self
    }

    pub fn divided(mut self, divided: bool) -> Self {
        self.props.divided = divided;
        self
    }

    pub fn props(&self) -> &DataIteratorProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentDataIteratorStyle {
        let theme = self.theme.get();
        let ComponentTokens::DataIterator(tokens) = theme.component(ComponentId::DataIterator)
        else {
            unreachable!("ComponentId::DataIterator must resolve to DataIteratorTokens")
        };
        FluentDataIteratorStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            outline: theme.colors().outline,
            padding: tokens.padding,
            item_gap: tokens.item_gap,
            divided: self.props.divided,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let outline = to_paint(style.outline);
        let divided = style.divided;
        let items = self.items.clone();
        let header = self.header.clone();
        let footer = self.footer.clone();

        let mut column = Stack::new(Axis::Vertical);
        if let Some(header) = header.clone() {
            column = column.push(move || header());
        }
        for (index, item) in items.iter().enumerate() {
            if divided && index > 0 {
                let outline = outline;
                column = column.push(move || {
                    Box::new(
                        Divider::new()
                            .thickness(1.0)
                            .alignment(RuleAlignment::Start)
                            .color(outline),
                    )
                });
            }
            let item = item.clone();
            column = column.push(move || item());
        }
        if let Some(footer) = footer.clone() {
            column = column.push(move || footer());
        }

        Surface::new()
            .request(
                LayoutRequest::new()
                    .padding_horiz(f64::from(style.padding))
                    .padding_vert(f64::from(style.padding))
                    .width_pct(100.0),
            )
            .background(to_paint(style.surface))
            .radius(8.0)
            .push(move || Box::new(column.clone()))
    }
}

impl Default for FluentDataIterator {
    fn default() -> Self {
        Self::new()
    }
}
