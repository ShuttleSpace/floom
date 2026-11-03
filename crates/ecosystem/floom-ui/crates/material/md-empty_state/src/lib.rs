
use floom_core::Child;
use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{FontWeight, Insets};
use floom_widgets::{Align, Axis, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::empty_state::EmptyStateProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

const PADDING: f64 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialEmptyStateStyle {
    pub design_system: &'static str,
    /// The title's ink.
    pub on_surface: Color,
    /// The icon's and the description's ink.
    pub on_surface_variant: Color,
    /// The surface the column sits on.
    pub surface: Color,
    pub icon_size: f32,
    pub title_font_size: f32,
    pub description_font_size: f32,
    pub gap: f32,
    pub max_width: f32,
}

/// Material's empty state.
pub struct MaterialEmptyState {
    props: EmptyStateProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl MaterialEmptyState {
    pub fn new() -> Self {
        Self {
            props: EmptyStateProps::default(),
            child: None,
            theme: theme_signal(),
        }
    }

    /// The glyph above the title. Empty means no icon.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.props.icon = icon.into();
        self
    }

    /// The headline.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    /// The supporting description. Empty means no description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.props.description = description.into();
        self
    }

    /// The action or illustration under the text.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn props(&self) -> &EmptyStateProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialEmptyStateStyle {
        let theme = self.theme.get();
        let ComponentTokens::EmptyState(tokens) = theme.component(ComponentId::EmptyState) else {
            unreachable!("ComponentId::EmptyState must resolve to EmptyStateTokens")
        };
        MaterialEmptyStateStyle {
            design_system: "material",
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            surface: theme.colors().surface,
            icon_size: tokens.icon_size,
            title_font_size: tokens.title_font_size,
            description_font_size: tokens.description_font_size,
            gap: tokens.gap,
            max_width: tokens.max_width,
        }
    }

    /// The empty state: the surface, and the column inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let mut column = Stack::new(Axis::Vertical)
            .gap(style.gap)
            .align(Align::Center, Align::Center)
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .max_width(style.max_width),
            );

        if !self.props.icon.is_empty() {
            let icon = self.props.icon.clone();
            let size = style.icon_size;
            let color = to_paint(style.on_surface_variant);
            column = column.push(move || Box::new(Text::new(icon.clone()).size(size).color(color)));
        }

        let title = self.props.title.clone();
        let title_size = style.title_font_size;
        let title_color = to_paint(style.on_surface);
        column = column.push(move || {
            Box::new(
                Text::new(title.clone())
                    .size(title_size)
                    .color(title_color)
                    .weight(FontWeight::Medium),
            )
        });

        if !self.props.description.is_empty() {
            let description = self.props.description.clone();
            let size = style.description_font_size;
            let color = to_paint(style.on_surface_variant);
            column = column
                .push(move || Box::new(Text::new(description.clone()).size(size).color(color)));
        }

        if let Some(child) = self.child {
            column = column.child(child);
        }

        let surface = to_paint(style.surface);
        Surface::new()
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .padding(Insets {
                left: PADDING,
                right: PADDING,
                top: PADDING,
                bottom: PADDING,
            })
            .background(surface)
            .push(move || Box::new(column.clone()))
    }
}

impl Default for MaterialEmptyState {
    fn default() -> Self {
        Self::new()
    }
}
