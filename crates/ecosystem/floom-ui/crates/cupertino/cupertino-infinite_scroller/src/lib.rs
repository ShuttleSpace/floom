//! Cupertino infinite scroller, ported to the Floom 2.0 widget API.
//!
//! An infinite scroller is a viewport that grows: the host hands it a page of
//! content, and when the user reaches the bottom of it the scroller asks for
//! the next page.
//!
//! ## What the port uses
//!
//! The engine's vocabulary for the box -- a [`Surface`] for the viewport's
//! fill and outline -- and the engine's `InfiniteScrollerView` for the two
//! things the engine's own widgets do not have.
//!
//! The engine's own `Scroll` is a viewport that scrolls, and it is the right
//! answer for a box that only scrolls. It cannot answer the question an
//! infinite scroller asks -- *how close to the end is the user?* -- because
//! its offset is its own business and nothing outside it can read it. So the
//! scroller keeps an offset of its own and asks the host to scroll; that is
//! the gap, and it is still the engine's to close: a `Scroll` that can hand
//! its offset to a descendant would take it away.
//!
//! ## What this kit answers
//!
//! * the loading label's size and the footer's padding: the tokens;
//! * the viewport's fill: `surface`; its outline: `outline` at 80;
//! * the loading label's ink: `on_surface_variant`.
//!
//! ## The token that never showed
//!
//! `on_surface` is in v1's style and v1 never painted with it: the footer
//! wrote `on_surface_variant` and the viewport wrote `surface`. The style
//! still reports it, because v1's style did.
//!
//! ## The numbers v1 wrote as literals
//!
//! The loading label's size (13). v1's `loading_indicator_size` is not the
//! size of an indicator -- there is no indicator, only a label -- it is the
//! *height* the loading footer adds to the content, and the port uses it the
//! same way.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The content height was a signal.** v1's `content_height_signal` took
//!   an `RwSignal<f32>`, and `RwSignal` is `Rc`-based, so it cannot cross a
//!   2.0 slot boundary. The port hands back a shared cell instead: the engine
//!   has a shared type for a bool, a string and a map, and none for a number,
//!   so the kit carries its own.
//! * **The callback is not in the props.** `InfiniteScrollerProps::on_load_more`
//!   is a `Box<dyn Fn()>`, which is not `Send + Sync`, and 2.0's slots are.
//!   The kit keeps its own `Arc<dyn Fn() + Send + Sync>` and the props' slot
//!   stays empty, so `props()` reports `None` where v1 reported `Some`.
//! * **The accessors.** v1's Cupertino infinite scroller had no `props()`,
//!   `theme()` or `with_theme()`. The port gives all three kits the same
//!   three, because a kit that cannot be given a theme cannot be tested
//!   against one.
//!
//! ## What the port does not do
//!
//! v1 emulated the scroll by *translating* the content (`translate_y`) inside
//! a box that clipped it. 2.0's scroll offset is a post-layout transform the
//! host applies, so the port asks for it instead of moving anything itself.

use std::sync::{Arc, Mutex};

use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::InfiniteScrollerView;
use floom_widgets::{shared_flag, write_flag, SharedFlag, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::infinite_scroller::InfiniteScrollerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// How tall the scrollable content is, shared between the kit and the host.
///
/// A shared cell rather than a signal: 2.0's slots are `Send + Sync` and a
/// signal is `Rc`-based. The engine has a shared type for a bool, a string and
/// a map, and none for a number, so the kit carries its own.
pub type SharedHeight = floom_ui_composites::SharedHeight;

/// A fresh height cell, starting at `height`.
pub fn shared_height(height: f32) -> SharedHeight {
    Arc::new(Mutex::new(height))
}

/// Read the height, treating a poisoned cell as zero.
/// The infinite scroller's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoInfiniteScrollerStyle {
    pub design_system: &'static str,
    /// The viewport's fill.
    pub surface: Color,
    /// v1 never painted with this; see the module documentation.
    pub on_surface: Color,
    /// The loading label's ink.
    pub on_surface_variant: Color,
    /// The viewport's outline.
    pub outline: Color,
    /// The height the loading footer adds to the content.
    pub loading_indicator_size: f32,
    /// The footer's padding.
    pub footer_padding: f32,
}

/// Cupertino's infinite scroller.
pub struct CupertinoInfiniteScroller {
    props: InfiniteScrollerProps,
    theme: RwSignal<MultiTheme>,
    /// The scrollable content.
    child: Option<Child>,
    /// How tall the content is. The host writes it as it appends.
    content_height: SharedHeight,
    /// Whether a page is being fetched.
    loading: SharedFlag,
    /// The viewport's height.
    viewport: f32,
    /// The kit's own handle on the callback.
    ///
    /// Not the props': `InfiniteScrollerProps::on_load_more` is a
    /// `Box<dyn Fn()>`, and 2.0's slots are `Send + Sync`.
    on_load_more: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl CupertinoInfiniteScroller {
    pub fn new() -> Self {
        Self {
            props: InfiniteScrollerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            child: None,
            content_height: shared_height(0.0),
            loading: shared_flag(false),
            viewport: 360.0,
            on_load_more: None,
        }
    }

    /// The scrollable content.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// The cell the host writes the content's height into.
    ///
    /// v1's `content_height_signal`, with the signal replaced by a cell the
    /// host can write from outside the tree.
    pub fn content_height_cell(mut self, height: SharedHeight) -> Self {
        self.content_height = height;
        self
    }

    /// A fixed content height, for content that does not grow.
    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = shared_height(height);
        self
    }

    /// The visible viewport height.
    pub fn viewport(mut self, height: f32) -> Self {
        self.viewport = height;
        self
    }

    /// How close to the end the user has to be for the scroller to ask for
    /// more.
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.props.threshold = threshold;
        self
    }

    /// Called when the user reaches the end of the content.
    pub fn on_load_more(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_load_more = Some(Arc::new(callback));
        self
    }

    /// The flag the host sets while a page is being fetched.
    ///
    /// v1's `loading_signal`, with the signal replaced by the engine's shared
    /// flag: the flag is what a 2.0 slot can hold.
    pub fn loading_flag(&self) -> SharedFlag {
        self.loading.clone()
    }

    /// The loading state to start in.
    pub fn loading(self, loading: bool) -> Self {
        write_flag(&self.loading, loading);
        self
    }

    pub fn props(&self) -> &InfiniteScrollerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoInfiniteScrollerStyle {
        let theme = self.theme.get();
        let ComponentTokens::InfiniteScroller(tokens) =
            theme.component(ComponentId::InfiniteScroller)
        else {
            unreachable!("ComponentId::InfiniteScroller must resolve to InfiniteScrollerTokens")
        };
        CupertinoInfiniteScrollerStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            loading_indicator_size: tokens.loading_indicator_size,
            footer_padding: tokens.footer_padding,
        }
    }

    /// The viewport: the box, and the scroller inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let viewport = self.viewport;
        let fill = to_paint(style.surface);
        let outline = to_paint(with_alpha(style.outline, 80));
        let ink = to_paint(style.on_surface_variant);
        let loading_height = style.loading_indicator_size + style.footer_padding * 2.0;
        let idle_height = style.footer_padding;
        let content_height = self.content_height.clone();
        let loading = self.loading.clone();
        let on_load_more = self.on_load_more.clone();
        let child = self.child.clone();
        let threshold = self.props.threshold;

        Surface::new()
            .request(LayoutRequest::new().height(viewport).width_pct(100.0))
            .background(fill)
            // v1's `.border(1.0)` and `.border_color(outline at 80)`.
            .border(1.0, outline)
            .push(move || {
                let mut scroller = InfiniteScrollerView::new()
                    .content(
                        child.clone(),
                        content_height.clone(),
                        loading.clone(),
                        loading_height,
                        idle_height,
                        ink,
                    )
                    .threshold(threshold)
                    .viewport(viewport);
                if let Some(on_load_more) = on_load_more.clone() {
                    scroller = scroller.on_load_more(move || on_load_more());
                }
                Box::new(scroller)
            })
    }
}

impl Default for CupertinoInfiniteScroller {
    fn default() -> Self {
        Self::new()
    }
}
