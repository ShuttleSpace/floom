//! Material tabs, ported to the Floom 2.0 widget API.
//!
//! A tab bar: a row of labels, one of them active, with an indicator line
//! under the active one. Clicking a label makes it the active one.
//!
//! ## What the port uses
//!
//! The engine's vocabulary -- a `Pressable` per tab for the click, a vertical
//! `Stack` for the label and the indicator, a `Text` for the label -- plus the
//! engine's `TabsBar`, the bar that owns the selection and re-declares itself
//! when the selection moves.
//!
//! ## The engine's `Tabs` composite, and why this kit does not use it
//!
//! `floom_ui_composites::Tabs` is the engine's tab bar, and its own
//! documentation is honest about what it is: "which indicator, how tall the
//! strip and what the inactive label looks like are three kits' answers to the
//! same question". But it does not take those answers. It draws a row of
//! engine buttons, prefixes the active label with "▶ ", uses a height of 24
//! and a gap of 4, and every tab carries a panel to show.
//!
//! v1's Material tabs are none of those things: an indicator line under the
//! active label, the theme's inks, the token's height of 48 and padding of
//! 16, and no panel at all. Using the composite would replace this kit with
//! something v1 never drew, so the port does not use it.
//!
//! What the composite *is*, though, is the shape this kit wants, and the
//! engine now has that shape: `floom_ui_composites::TabsBar` is a bar that
//! takes a kit's answer -- the inks, the two weights, the indicator's colour
//! and thickness, the label's size, the padding and the height -- and owns
//! the selection, re-declaring itself when the selection moves and baking the
//! active label's appearance into the widget it builds. The three kits used
//! to carry that bar three times; they now use the engine's one and say only
//! what they answer.
//!
//! ## What this kit answers
//!
//! * the bar's height, the indicator's thickness and the space at each side of
//!   a label: the tokens;
//! * the active label's ink: `primary`; the inactive label's:
//!   `on_surface_variant`; the indicator's: `primary`;
//! * the label's size (14) and its weight (medium, for the active label and
//!   the inactive one alike): v1 literals;
//! * the bar's fill: none, because v1's Material bar was left unpainted.
//!
//! ## The active tab does not fit in the bar
//!
//! v1's Material bar is 48 tall -- the token's height -- and the active tab is
//! a 48-tall label plus a 3-tall indicator, so the label is squeezed by three
//! pixels to fit and the indicator sits inside the bar. 2.0's layout engine
//! shrinks the same way, so the port keeps the numbers and the squeeze.
//!
//! ## The bar is not painted
//!
//! v1's Material bar was left unpainted and sat on whatever was behind it;
//! Cupertino's was painted the theme's `surface`. The port keeps that
//! difference: this kit's bar is painted with nothing and Cupertino's with the
//! surface.
//!
//! ## The tab is rebuilt when the selection moves
//!
//! 2.0 mounts a tree once, so a label's ink cannot change after the mount
//! unless the widget that draws it is rebuilt. The bar therefore keys each
//! tab by the selection, and a tab is rebuilt -- its label's ink and its
//! indicator baked in -- when the selection moves. The cost is that the tab
//! loses the keyboard focus it had, which v1 did not: v1's bar was built once
//! and only its *styles* were re-evaluated. The engine's own `Tabs` composite
//! makes the same trade, for the same reason.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The hover fill.** v1 painted `on_surface` at 8/255 over the *inactive*
//!   tab while the pointer was over it. 2.0's `Pressable` paints nothing of
//!   its own and the engine has no state-layer widget for a text label, so the
//!   fill is not carried. The tab is still clickable, and by keyboard too,
//!   which the pointer was not.
//! * **The pointer cursor.** v1 asked for `CursorStyle::Pointer` on the tab.
//!   2.0's `Pressable` does not answer `Widget::cursor`, so the pointer is not
//!   carried either.
//! * **The callback in the props.** v1 stored the change callback in
//!   `TabsProps::on_change`, which is a `Box<dyn Fn(usize)>`. 2.0's slots are
//!   shared between threads, so the callback the tree uses is an
//!   `Arc<dyn Fn(usize) + Send + Sync>` held beside the props, and the props'
//!   own slot stays empty.

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

/// The label's size. A v1 literal.
const LABEL_FONT_SIZE: f32 = 14.0;

/// The tabs' appearance, in the terms v1 used.
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
pub struct MaterialTabs {
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
            // v1 wrote `FontWeight::Medium` for the label, active or not.
            active_label_font_weight: FontWeight::Medium,
            inactive_label_font_weight: FontWeight::Medium,
            // v1's Material bar was left unpainted.
            bar_color: None,
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

impl Default for MaterialTabs {
    fn default() -> Self {
        Self::new()
    }
}
