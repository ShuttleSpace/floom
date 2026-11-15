//! Material tabs, ported to the Floom 2.0 widget API.
//!

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
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

/// The label's size. A v1 literal.
const LABEL_FONT_SIZE: f32 = 14.0;

/// The tabs' appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTabsStyle {
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
    /// The label's size. v1 wrote 14.
    pub label_font_size: f32,
    /// The active label's weight. v1 wrote medium.
    pub active_label_font_weight: FontWeight,
    /// The inactive label's weight. v1 wrote medium.
    pub inactive_label_font_weight: FontWeight,
    /// The bar's fill. `None` for the kits whose bar v1 left unpainted.
    pub bar_color: Option<Color>,
}

/// Material's tabs.
pub struct CupertinoTabs {
    props: TabsProps,
    theme: RwSignal<MultiTheme>,
    /// The tab on show. Shared with the tree the kit builds.
    selection: Arc<AtomicUsize>,
    /// The change callback, in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(usize)>`, which
    /// cannot cross that boundary.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl CupertinoTabs {
    pub fn new() -> Self {
        Self {
            props: TabsProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    /// The shared props.
    ///
    /// `on_change` is deliberately absent from what it reports: the props are
    /// the v1 carrier of a `Box<dyn Fn(usize)>` and the kit holds that here.
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

    pub fn resolved_style(&self) -> CupertinoTabsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tabs(tokens) = theme.component(ComponentId::Tabs) else {
            unreachable!("ComponentId::Tabs must resolve to TabsTokens")
        };
        CupertinoTabsStyle {
            design_system: "cupertino",
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            indicator_color: theme.colors().primary,
            height: tokens.height,
            indicator_height: tokens.indicator_height,
            padding: tokens.padding,
            label_font_size: LABEL_FONT_SIZE,
            // v1 took the active label's weight from the theme's ramp ...
            active_label_font_weight: to_engine_weight(theme.typography().label.large.font_weight),
            inactive_label_font_weight: FontWeight::Normal,
            bar_color: Some(theme.colors().surface),
        }
    }

    /// The bar: a row of tabs, with the active one marked.
    ///
    /// The kit is borrowed rather than consumed, so a caller can read
    /// [`Self::active`] after the bar has been built -- and the tree and the
    /// kit share the same selection, which is what makes the click move the
    /// highlight.
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

impl Default for CupertinoTabs {
    fn default() -> Self {
        Self::new()
    }
}

fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        // v1's name for 400 is `NORMAL`; the theme's is `Regular`.
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}
