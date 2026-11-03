//! Fluent bottom navigation, ported to the Floom 2.0 widget API.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
pub use floom_ui_composites::BottomNavigationBar;
use floom_ui_composites::BottomNavigationStyle;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::bottom_navigation::BottomNavigationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

pub use floom_ui_composites::IconBuilder;

type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

const DIVIDER_WIDTH: f32 = 1.0;

const INDICATOR_WIDTH: f32 = 2.0;

const DESTINATION_PADDING_VERT: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentBottomNavigationStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub selected_color: Color,
    pub unselected_color: Color,
    pub divider_color: Color,
    pub divider_width: f32,
    pub height: f32,
    pub item_min_width: f32,
    pub icon_size: f32,
    pub label_font_size: f32,
    pub gap: f32,
}

pub struct FluentBottomNavigation {
    props: BottomNavigationProps,
    icons: Vec<IconBuilder>,
    theme: RwSignal<MultiTheme>,
    selection: Arc<AtomicUsize>,
    on_change: Option<Callback>,
}

impl FluentBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        let selected = props.selected;
        Self {
            props,
            icons: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            selection: Arc::new(AtomicUsize::new(selected)),
            on_change: None,
        }
    }

    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.props.labels = labels;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = selected;
        self.selection.store(selected, Ordering::SeqCst);
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn icons(mut self, icons: Vec<IconBuilder>) -> Self {
        self.icons = icons;
        self
    }

    pub fn props(&self) -> &BottomNavigationProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn selection(&self) -> Arc<AtomicUsize> {
        self.selection.clone()
    }

    pub fn resolved_style(&self) -> FluentBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) =
            theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        FluentBottomNavigationStyle {
            design_system: "fluent",
            background: theme.colors().surface,
            selected_color: theme.colors().primary,
            unselected_color: theme.colors().on_surface_variant,
            divider_color: with_alpha(theme.colors().on_surface, 12),
            divider_width: DIVIDER_WIDTH,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
            gap: tokens.gap,
        }
    }

    pub fn view(self) -> BottomNavigationBar {
        let style = self.resolved_style();
        let mut bar = BottomNavigationBar::new()
            .style(BottomNavigationStyle {
                bar_color: to_paint(style.background),
                selected_color: to_paint(style.selected_color),
                unselected_color: to_paint(style.unselected_color),
                divider_color: to_paint(style.divider_color),
                divider_width: style.divider_width,
                height: style.height,
                item_min_width: None,
                icon_size: Some(style.icon_size),
                label_font_size: style.label_font_size,
                gap: style.gap,
                label_margin_top: 0.0,
                destination_padding_vert: DESTINATION_PADDING_VERT,
                indicator: Some((INDICATOR_WIDTH, to_paint(style.selected_color))),
                hover_color: None,
            })
            .labels(self.props.labels)
            .icons(self.icons)
            .shared_selection(self.selection);
        if let Some(on_change) = self.on_change {
            bar = bar.on_change(move |index| on_change(index));
        }
        bar
    }
}

impl Default for FluentBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}
