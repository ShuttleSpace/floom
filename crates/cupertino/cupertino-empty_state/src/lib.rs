use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::empty_state::EmptyStateProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoEmptyStateStyle {
    pub design_system: &'static str,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub icon_size: f32,
    pub title_font_size: f32,
    pub description_font_size: f32,
    pub gap: f32,
    pub max_width: f32,
}

pub struct CupertinoEmptyState {
    props: EmptyStateProps,
    child: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoEmptyState {
    pub fn new() -> Self {
        let props = EmptyStateProps::default();
        Self {
            props,
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.props.icon = icon.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.props.description = description.into();
        self
    }

    /// Optional action / illustration slot shown below the text.
    pub fn child(mut self, child: impl IntoView) -> Self {
        self.child = Some(child.into_any());
        self
    }

    pub fn resolved_style(&self) -> CupertinoEmptyStateStyle {
        let theme = self.theme.get();
        let ComponentTokens::EmptyState(tokens) = theme.component(ComponentId::EmptyState) else {
            unreachable!("ComponentId::EmptyState must resolve to EmptyStateTokens")
        };
        CupertinoEmptyStateStyle {
            design_system: "cupertino",
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            icon_size: tokens.icon_size,
            title_font_size: tokens.title_font_size,
            description_font_size: tokens.description_font_size,
            gap: tokens.gap,
            max_width: tokens.max_width,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child;

        let tokens = match theme.get().component(ComponentId::EmptyState) {
            ComponentTokens::EmptyState(t) => t,
            _ => unreachable!(),
        };
        let icon_size = tokens.icon_size;
        let title_font = tokens.title_font_size;
        let desc_font = tokens.description_font_size;
        let gap = tokens.gap;
        let max_width = tokens.max_width;

        let mut kids: Vec<AnyView> = Vec::new();

        if !props.icon.is_empty() {
            kids.push(
                text(props.icon.clone())
                    .style(move |s| {
                        let t = theme.get();
                        s.font_size(icon_size)
                            .color(t.colors().on_surface_variant.to_peniko())
                    })
                    .into_any(),
            );
        }

        kids.push(
            text(props.title.clone())
                .style(move |s| {
                    let t = theme.get();
                    s.font_size(title_font)
                        .color(t.colors().on_surface.to_peniko())
                        .font_weight(FontWeight::Medium.to_floem())
                })
                .into_any(),
        );

        if !props.description.is_empty() {
            kids.push(
                text(props.description.clone())
                    .style(move |s| {
                        let t = theme.get();
                        s.font_size(desc_font)
                            .color(t.colors().on_surface_variant.to_peniko())
                    })
                    .into_any(),
            );
        }

        if let Some(c) = child {
            kids.push(c);
        }

        let content = list(kids).style(move |s| {
            let t = theme.get();
            s.flex_col()
                .items_center()
                .justify_center()
                .gap(gap)
                .max_width(max_width)
                .width_pct(100.0)
                .color(t.colors().on_surface.to_peniko())
        });

        Container::new(content).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .items_center()
                .justify_center()
                .padding(24.0)
                .background(t.colors().surface.to_peniko())
        })
    }
}

impl Default for CupertinoEmptyState {
    fn default() -> Self {
        Self::new()
    }
}
