//! Fluent pull-to-refresh, ported to the Floom 2.0 widget API.

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

pub type PullDistance = floom_ui_composites::SharedNumber;

const INDICATOR_FONT_SIZE: f32 = 13.0;
const INDICATOR_MAX_HEIGHT: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentPullToRefreshStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub on_surface_variant: Color,
    pub max_pull: f32,
    pub threshold: f32,
    /// The token's indicator size. v1 never read it -- see the module note.
    pub indicator_size: f32,
    pub indicator_font_size: f32,
    pub indicator_max_height: f32,
}

pub struct FluentPullToRefresh {
    props: PullToRefreshProps,
    child: Option<Child>,
    refreshing: SharedFlag,
    on_refresh: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    pull: PullDistance,
    theme: RwSignal<MultiTheme>,
}

impl FluentPullToRefresh {
    pub fn new() -> Self {
        let props = PullToRefreshProps::default();
        Self {
            refreshing: shared_flag(props.loading),
            props,
            child: None,
            on_refresh: None,
            pull: Arc::new(Mutex::new(0.0)),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        write_flag(&self.refreshing, loading);
        self
    }

    pub fn threshold(mut self, threshold: f32) -> Self {
        self.props.threshold = threshold;
        self
    }

    pub fn on_refresh(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_refresh = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_refresh = Some(shared);
        self
    }

    pub fn refreshing_flag(&self) -> SharedFlag {
        self.refreshing.clone()
    }

    pub fn with_refreshing_flag(mut self, refreshing: SharedFlag) -> Self {
        self.props.loading = read_flag(&refreshing);
        self.refreshing = refreshing;
        self
    }

    /// The seam the drag writes to. See the module note for why the drag
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

    pub fn resolved_style(&self) -> FluentPullToRefreshStyle {
        let theme = self.theme.get();
        let ComponentTokens::PullToRefresh(tokens) = theme.component(ComponentId::PullToRefresh)
        else {
            unreachable!("ComponentId::PullToRefresh must resolve to PullToRefreshTokens")
        };
        FluentPullToRefreshStyle {
            design_system: "fluent",
            background: theme.colors().background,
            on_surface_variant: theme.colors().on_surface_variant,
            max_pull: tokens.max_pull,
            threshold: self.props.threshold,
            indicator_size: tokens.indicator_size,
            indicator_font_size: INDICATOR_FONT_SIZE,
            indicator_max_height: INDICATOR_MAX_HEIGHT,
        }
    }

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

impl Default for FluentPullToRefresh {
    fn default() -> Self {
        Self::new()
    }
}
