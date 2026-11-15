use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_ui_composites::{TabsBar, TabsBarStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tabs::TabsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

const LABEL_FONT_SIZE: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialTabsStyle {
    pub design_system: &'static str,
    /// The active label's ink.
    pub active_color: Color,
    /// The inactive label's ink.
    pub inactive_color: Color,
    /// The indicator line's colour.
    pub indicator_color: Color,
    /// The bar's height.
    pub height: f32,
    /// The indicator line's thickness.
    pub indicator_height: f32,
    /// The space at each side of a label.
    pub padding: f32,
    pub label_font_size: f32,
    pub active_label_font_weight: FontWeight,
    pub inactive_label_font_weight: FontWeight,
    pub bar_color: Option<Color>,
}

/// Material's tabs.
pub struct MaterialTabs {
    props: TabsProps,
    theme: RwSignal<MultiTheme>,
    /// The tab on show. Shared with the tree the kit builds.
    selection: Arc<AtomicUsize>,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl MaterialTabs {
    pub fn new() -> Self {
        Self {
            props: TabsProps::default(),
            theme: theme_signal(),
            selection: Arc::new(AtomicUsize::new(0)),
            on_change: None,
        }
    }

    /// Append a tab.
    pub fn tab(mut self, tab: impl Into<String>) -> Self {
        self.props.tabs.push(tab.into());
        self
    }

    /// Replace the tabs.
    pub fn tabs(mut self, tabs: Vec<String>) -> Self {
        self.props.tabs = tabs;
        self
    }

    /// The tab that is active before the bar is built.
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = selected;
        self.selection.store(selected, Ordering::SeqCst);
        self
    }

    /// Called with the index of the tab that was clicked.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
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

    /// Which tab is active, as the shared state reports it.
    pub fn active(&self) -> Option<usize> {
        let count = self.props.tabs.len();
        if count == 0 {
            return None;
        }
        Some(self.selection.load(Ordering::SeqCst).min(count - 1))
    }

    pub fn resolved_style(&self) -> MaterialTabsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tabs(tokens) = theme.component(ComponentId::Tabs) else {
            unreachable!("ComponentId::Tabs must resolve to TabsTokens")
        };
        MaterialTabsStyle {
            design_system: "material",
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            indicator_color: theme.colors().primary,
            height: tokens.height,
            indicator_height: tokens.indicator_height,
            padding: tokens.padding,
            label_font_size: LABEL_FONT_SIZE,
            active_label_font_weight: FontWeight::Medium,
            inactive_label_font_weight: FontWeight::Medium,
            bar_color: None,
        }
    }

    pub fn view(&self) -> TabsBar {
        let style = self.resolved_style();
        let bar_color = style.bar_color;
        let mut bar = TabsBar::new()
            .style(TabsBarStyle {
                active_color: to_paint(style.active_color),
                inactive_color: to_paint(style.inactive_color),
                indicator_color: to_paint(style.indicator_color),
                height: style.height,
                indicator_height: style.indicator_height,
                padding: style.padding,
                label_font_size: style.label_font_size,
                active_label_font_weight: style.active_label_font_weight,
                inactive_label_font_weight: style.inactive_label_font_weight,
                bar_color: bar_color.map(to_paint),
            })
            .tabs(self.props.tabs.clone())
            .selection(self.selection.clone());
        if let Some(on_change) = self.on_change.clone() {
            bar = bar.on_change(move |index| on_change(index));
        }
        bar
    }
}

impl Default for MaterialTabs {
    fn default() -> Self {
        Self::new()
    }
}
