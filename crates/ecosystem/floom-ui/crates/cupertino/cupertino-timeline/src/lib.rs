use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::timeline::{TimelineItem, TimelineProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTimelineStyle {
    pub design_system: &'static str,
    pub dot_color: Color,
    pub line_color: Color,
    pub dot_size: f32,
    pub line_width: f32,
    pub item_gap: f32,
    pub padding: f32,
}

pub struct CupertinoTimeline {
    props: TimelineProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTimeline {
    pub fn new() -> Self {
        Self {
            props: TimelineProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn item(mut self, title: impl Into<String>, description: impl Into<String>) -> Self {
        self.props.items.push(TimelineItem {
            title: title.into(),
            description: description.into(),
        });
        self
    }

    pub fn items(mut self, items: Vec<TimelineItem>) -> Self {
        self.props.items = items;
        self
    }

    pub fn props(&self) -> &TimelineProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoTimelineStyle {
        let theme = self.theme.get();
        let ComponentTokens::Timeline(tokens) = theme.component(ComponentId::Timeline) else {
            unreachable!("ComponentId::Timeline must resolve to TimelineTokens")
        };
        CupertinoTimelineStyle {
            design_system: DESIGN_SYSTEM,
            dot_color: theme.colors().primary,
            line_color: theme.colors().outline_variant,
            dot_size: tokens.dot_size,
            line_width: tokens.line_width,
            item_gap: tokens.item_gap,
            padding: tokens.padding,
        }
    }

    /// Build a vertical Cupertino timeline connecting dot markers with a line.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.props.items.clone();

        let tokens = match theme.get().component(ComponentId::Timeline) {
            ComponentTokens::Timeline(t) => t,
            _ => unreachable!(),
        };
        let dot_size = tokens.dot_size;
        let line_width = tokens.line_width;
        let item_gap = tokens.item_gap;
        let padding = tokens.padding;

        let n = items.len();
        let mut rows: Vec<AnyView> = Vec::with_capacity(n.max(1));
        for (idx, item) in items.into_iter().enumerate() {
            let is_last = idx + 1 == n;
            let line_w = if is_last { 0.0 } else { line_width };

            let dot = Container::new(Label::new("".to_string())).style(move |s| {
                let t = theme.get();
                s.width(dot_size)
                    .height(dot_size)
                    .border_radius(9999.0)
                    .background(t.colors().primary.to_peniko())
            });
            let line = Container::new(Label::new("".to_string())).style(move |s| {
                let t = theme.get();
                s.width(line_w)
                    .flex_grow(1.0)
                    .background(t.colors().outline_variant.to_peniko())
            });
            let left = Stack::vertical((dot, line)).style(|s| s.items_center());

            let right = Stack::vertical((
                Label::new(item.title).style(move |s| {
                    let t = theme.get();
                    s.font_size(15.0)
                        .font_weight(FontWeight::SemiBold.to_floom())
                        .color(t.colors().on_surface.to_peniko())
                }),
                Label::new(item.description).style(move |s| {
                    let t = theme.get();
                    s.font_size(13.0)
                        .color(t.colors().on_surface_variant.to_peniko())
                }),
            ))
            .style(|s| s.gap(2.0));

            let row = Stack::horizontal((left, right)).style(move |s| s.gap(padding).items_start());
            rows.push(row.into_any());
        }

        Container::new(list(rows)).style(move |s| s.gap(item_gap).padding(padding).items_start())
    }
}

impl Default for CupertinoTimeline {
    fn default() -> Self {
        Self::new()
    }
}
