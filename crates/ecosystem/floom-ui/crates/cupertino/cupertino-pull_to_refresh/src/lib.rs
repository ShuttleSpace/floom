//! Cupertino pull-to-refresh, ported to the Floom 2.0 widget API.
//!

//! ```text

use std::sync::{Arc, Mutex};

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{PullToRefresh, PullToRefreshStyle};
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::pull_to_refresh::PullToRefreshProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// The pull distance, shared between the kit and whoever drives the gesture.
///
/// The engine's shared types cover a bool (`SharedFlag`), a string
/// (`SharedBuffer`) and a map (`SharedMap`); a *distance* is none of those, so
/// this is the same `Arc<Mutex<f32>>` shape the engine's own `Scroll` uses for
/// its offset.
pub type PullDistance = floom_ui_composites::SharedNumber;

/// The label's size. A v1 literal.
const INDICATOR_FONT_SIZE: f32 = 13.0;
/// The tallest the indicator ever gets. A v1 literal.
const INDICATOR_MAX_HEIGHT: f32 = 28.0;

/// The pull-to-refresh's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoPullToRefreshStyle {
    pub design_system: &'static str,
    /// The box's fill.
    pub background: Color,
    /// The indicator's ink.
    pub on_surface_variant: Color,
    /// The furthest the content may be pulled. v1 read this token.
    pub max_pull: f32,
    /// The drag distance that fires the refresh. v1 took this from the props,
    /// never from the token.
    pub threshold: f32,
    /// The token's indicator size. v1 never read it -- see the module note.
    pub indicator_size: f32,
    /// The label's size. A v1 literal.
    pub indicator_font_size: f32,
    /// The tallest the indicator gets. A v1 literal.
    pub indicator_max_height: f32,
}

/// Cupertino's pull-to-refresh.
pub struct CupertinoPullToRefresh {
    props: PullToRefreshProps,
    child: Option<Child>,
    /// Whether a refresh is in flight.
    refreshing: SharedFlag,
    /// v1's `Box<dyn Fn()>`, kept in a shareable form so a slot closure can
    /// hold it. The props' own slot stays filled so `props()` still reports
    /// one.
    on_refresh: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// How far the content has been pulled.
    pull: PullDistance,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoPullToRefresh {
    pub fn new() -> Self {
        let props = PullToRefreshProps::default();
        Self {
            refreshing: shared_flag(props.loading),
            props,
            child: None,
            on_refresh: None,
            pull: Arc::new(Mutex::new(0.0)),
            // v1's Cupertino kit took a snapshot of its own design system.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// The content the pull wraps.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Whether a refresh is in flight. v1's `loading`.
    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        write_flag(&self.refreshing, loading);
        self
    }

    /// The drag distance that fires the refresh.
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.props.threshold = threshold;
        self
    }

    /// Invoked when the pull passes the threshold.
    ///
    /// `Send + Sync`, because 2.0's slots are shared between threads and v1's
    /// `Box<dyn Fn()>` was not.
    pub fn on_refresh(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_refresh = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_refresh = Some(shared);
        self
    }

    /// The shared "is a refresh in flight" flag.
    ///
    /// v1's `refreshing_signal()`, and the reason it cannot be an `RwSignal`:
    /// that is `Rc`-based and cannot cross a `Send + Sync` slot boundary.
    pub fn refreshing_flag(&self) -> SharedFlag {
        self.refreshing.clone()
    }

    /// Drive the kit from a flag somebody else owns.
    ///
    /// v1's `with_refreshing_signal(RwSignal<bool>)`.
    pub fn with_refreshing_flag(mut self, refreshing: SharedFlag) -> Self {
        self.props.loading = read_flag(&refreshing);
        self.refreshing = refreshing;
        self
    }

    /// The shared pull distance: how far the content has been pulled down.
    ///
    /// The seam the drag writes to. See the module note for why the drag
    /// itself is not in the port.
    pub fn pull_distance(&self) -> PullDistance {
        self.pull.clone()
    }

    pub fn props(&self) -> &PullToRefreshProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoPullToRefreshStyle {
        let theme = self.theme.get();
        let ComponentTokens::PullToRefresh(tokens) = theme.component(ComponentId::PullToRefresh)
        else {
            unreachable!("ComponentId::PullToRefresh must resolve to PullToRefreshTokens")
        };
        CupertinoPullToRefreshStyle {
            design_system: "cupertino",
            background: theme.colors().background,
            on_surface_variant: theme.colors().on_surface_variant,
            max_pull: tokens.max_pull,
            // v1 read the props' threshold, never the token's.
            threshold: self.props.threshold,
            indicator_size: tokens.indicator_size,
            indicator_font_size: INDICATOR_FONT_SIZE,
            indicator_max_height: INDICATOR_MAX_HEIGHT,
        }
    }

    /// The box, the column, and the column's two children.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let mut refresh = PullToRefresh::new()
            .style(PullToRefreshStyle {
                background: to_paint(style.background),
                on_surface_variant: to_paint(style.on_surface_variant),
                max_pull: style.max_pull,
                threshold: style.threshold,
                indicator_size: style.indicator_size,
                indicator_font_size: style.indicator_font_size,
                indicator_max_height: style.indicator_max_height,
            })
            .pull(self.pull.clone())
            .refreshing(self.refreshing.clone())
            .child(self.child.clone());
        if let Some(on_refresh) = self.on_refresh.clone() {
            refresh = refresh.on_refresh(move || on_refresh());
        }
        refresh.build()
    }
}

impl Default for CupertinoPullToRefresh {
    fn default() -> Self {
        Self::new()
    }
}
