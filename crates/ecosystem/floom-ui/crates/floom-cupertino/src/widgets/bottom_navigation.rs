//! Cupertino bottom navigation, ported to the Floom 2.0 widget API.
//!

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

/// A builder for one destination's icon.
///
/// v1 took `Vec<AnyView>`: a built view per destination. 2.0's slots are
/// replayed `Fn`s and a widget is mounted by its parent, so the caller hands
/// over a builder rather than a widget.
pub use floom_ui_composites::IconBuilder;

/// A callback in the shape 2.0's slots need.
///
/// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
/// `Child` closure the host may hand between threads, so the callback is
/// shared as an `Arc`.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The rule along the bar's top edge. A v1 literal.
const DIVIDER_WIDTH: f32 = 0.5;

/// The gap between a destination's icon and the top of its label. A v1
/// literal: v1 wrote `margin_top(4.0)` on the label.
const LABEL_MARGIN_TOP: f32 = 4.0;

/// The bottom navigation's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoBottomNavigationStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub background: Color,
    /// The chosen destination's ink.
    pub active_color: Color,
    /// The other destinations' ink.
    pub inactive_color: Color,
    /// The rule along the bar's top edge.
    pub divider_color: Color,
    /// The rule's thickness. A v1 literal.
    pub divider_width: f32,
    pub height: f32,
    pub item_min_width: f32,
    pub icon_size: f32,
    pub label_font_size: f32,
    /// The gap between a destination's icon and its label. v1's style did not
    /// report it.
    pub gap: f32,
    /// The fill under a destination the pointer is over. v1's style did not
    /// report it.
    pub hover_color: Color,
}

/// Cupertino's bottom navigation.
pub struct CupertinoBottomNavigation {
    props: BottomNavigationProps,
    /// The icons, one per destination, in the same order as the labels.
    icons: Vec<IconBuilder>,
    theme: RwSignal<MultiTheme>,
    /// The chosen destination.
    selection: Arc<AtomicUsize>,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// `BottomNavigationProps::on_change` stays empty: it is a
    /// `Box<dyn Fn(usize)>`, which cannot cross a slot boundary.
    on_change: Option<Callback>,
}

impl CupertinoBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        // The props are moved into the kit below, so the selection they carry
        let selected = props.selected;
        Self {
            props,
            icons: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            selection: Arc::new(AtomicUsize::new(selected)),
            on_change: None,
        }
    }

    /// The destinations' labels, in order.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.props.labels = labels;
        self
    }

    /// The destination chosen before mount.
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = selected;
        self.selection.store(selected, Ordering::SeqCst);
        self
    }

    /// Called with the chosen destination's index whenever the choice changes.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The icons, one per destination, in the same order as the labels.
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

    /// The shared choice, so a caller can read it and drive it.
    ///
    /// v1's `selected_signal()` was an `RwSignal<usize>`; 2.0's shared state is
    /// an `Arc<AtomicUsize>`, the shape the engine's own `Tabs` uses.
    pub fn selection(&self) -> Arc<AtomicUsize> {
        self.selection.clone()
    }

    pub fn resolved_style(&self) -> CupertinoBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) =
            theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        CupertinoBottomNavigationStyle {
            design_system: "cupertino",
            // v1's bar: `with_alpha(surface, 235)`.
            background: with_alpha(theme.colors().surface, 235),
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            // v1's bar: `with_alpha(on_surface, 30)`.
            divider_color: with_alpha(theme.colors().on_surface, 30),
            divider_width: DIVIDER_WIDTH,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
            gap: tokens.gap,
            // v1's hovered destination: `with_alpha(on_surface, 12)`.
            hover_color: with_alpha(theme.colors().on_surface, 12),
        }
    }

    /// The bar: the destinations, the shared choice, and the bar's own box.
    pub fn view(self) -> BottomNavigationBar {
        let style = self.resolved_style();
        let mut bar = BottomNavigationBar::new()
            .style(BottomNavigationStyle {
                bar_color: to_paint(style.background),
                selected_color: to_paint(style.active_color),
                unselected_color: to_paint(style.inactive_color),
                divider_color: to_paint(style.divider_color),
                divider_width: style.divider_width,
                height: style.height,
                item_min_width: Some(style.item_min_width),
                icon_size: Some(style.icon_size),
                label_font_size: style.label_font_size,
                gap: style.gap,
                label_margin_top: LABEL_MARGIN_TOP,
                destination_padding_vert: 0.0,
                indicator: None,
                hover_color: Some(to_paint(style.hover_color)),
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

impl Default for CupertinoBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}
