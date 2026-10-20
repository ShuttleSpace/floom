//! Material pull-to-refresh, ported to the Floom 2.0 widget API.
//!
//! A pull-to-refresh wraps a scrollable: drag the content downwards and, once
//! the drag passes a threshold, the kit fires `on_refresh` and shows an
//! indicator above the content until the host says the refresh is done.
//!
//! ## What the port uses
//!
//! The engine's vocabulary, plus two widgets this module owns:
//!
//! ```text
//! Surface            the tinted box: the background, the full width
//!  └ Column          the column: the indicator, then the caller's content
//!     ├ Indicator    the label, and the height that *is* the pull
//!     └ content      the caller's child
//! ```
//!
//! The two exist for one reason, and it is the engine's rule rather than a
//! taste: a node's own layout request is only re-read when its *parent*
//! reconciles, and nothing in the engine reconciles a `Stack` on its own. The
//! indicator's height is the pull, so the column -- this kit's own widget --
//! has to be the one that asks for the reconcile.
//!
//! ## The gesture is not in the port
//!
//! v1 detected the drag itself: a `PointerDown` listener on the wrapping
//! container recorded the start, a `PointerMove` listener grew a `pull` signal,
//! and a `PointerUp` listener compared the pull with the threshold. 2.0's
//! event path cannot carry that, and the reasons are worth writing down
//! because they are the engine's, not this kit's:
//!
//! * **A container cannot measure a drag over its own content.** The pointer's
//!   position arrives in the *target's* coordinate space -- the space of the
//!   widget under the pointer -- and a widget is never told which widget that
//!   was. The content moves as the pull grows (the indicator's height pushes
//!   it down), so a pull measured in the content's space feeds itself: the
//!   pointer standing still reads as a pull of minus the pull.
//! * **The route has a capture phase.** A container sees the pointer down
//!   *before* the widget under the pointer does, so a container-level gesture
//!   starts over every control inside it, and the release it takes to end the
//!   drag is the release the control needed. v1's container saw the down only
//!   in the bubble phase, that is, only when the control did not take it.
//! * **There is no pointer capture.** `EventContext` has no
//!   `request_pointer_capture`, so a drag that leaves the widget's box is
//!   lost. v1 called `request_pointer_capture` on the way down.
//!
//! `floom-gesture`'s arena does not close the gap: it arbitrates competing
//! recognizers, but a widget still has to feed it the samples and read the
//! verdict, and it says nothing about the coordinate space or the capture.
//!
//! So the port carries everything the gesture *show* -- the indicator's label,
//! its size, its ink, the height the pull gives it, the content pushed down
//! under it, the refresh that fires past the threshold -- and leaves the drag
//! to the caller, through [`MaterialPullToRefresh::pull_distance`]. That is
//! the seam the engine's own drag widget would write to; the kit reads it,
//! draws it, and fires the refresh when it passes the threshold.
//!
//! ## What this kit answers
//!
//! * the furthest the content may be pulled: the token's `max_pull`;
//! * the drag distance that fires the refresh: the *prop's* threshold, not the
//!   token's -- v1 read `props.threshold` and never the token, and the two
//!   happen to share a default;
//! * the indicator's ink: `on_surface_variant`;
//! * the box's fill: `background`;
//! * the label's size (13) and the indicator's tallest height (28): v1
//!   literals, and the pull below which nothing is drawn (0.5) is one too.
//!
//! ## The token that never showed
//!
//! `PullToRefreshTokens::indicator_size` (24) is read by nobody in v1: the
//! indicator's size was the literal 13 and its height the literal 28. The
//! port keeps the number in the style and names it here, because a style that
//! omits a value the kit had is indistinguishable from one that chose to leave
//! it at a default.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The refresh fired on the release.** v1 compared the pull with the
//!   threshold when the pointer came up. The port compares when the pull
//!   *reaches* the threshold, because there is no release to wait for: the
//!   drag is the caller's. The refresh re-arms when the pull goes back to
//!   zero, which is the caller's move as well.
//! * **The child is a slot, not a view.** v1's `child(..)` took a factory
//!   (`impl Fn() -> AnyView`) and called it once while building the view.
//!   2.0's takes a [`Child`], because a 2.0 widget is mounted by its parent
//!   rather than returned.
//! * **The accessors.** v1's pull-to-refresh had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.

use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, CreateContext, ElementId, LayoutRequest, UpdateContext, Widget,
    WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::pull_to_refresh::PullToRefreshProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// The label v1 showed while the user was pulling.
const PULL_LABEL: &str = "↓ Pull to refresh";
/// The label v1 showed while the refresh was in flight.
const REFRESH_LABEL: &str = "↻ Refreshing…";
/// The label's size. A v1 literal.
const INDICATOR_FONT_SIZE: f32 = 13.0;
/// The tallest the indicator ever gets. A v1 literal.
const INDICATOR_MAX_HEIGHT: f32 = 28.0;
/// Below this pull the indicator is not drawn at all. A v1 literal.
const INDICATOR_VISIBLE_PULL: f32 = 0.5;

/// The pull distance, shared between the kit and whoever drives the gesture.
///
/// The engine's shared types cover a bool (`SharedFlag`), a string
/// (`SharedBuffer`) and a map (`SharedMap`); a *distance* is none of those, so
/// this is the same `Arc<Mutex<f32>>` shape the engine's own `Scroll` uses for
/// its offset.
pub type PullDistance = Arc<Mutex<f32>>;

/// The pull-to-refresh's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialPullToRefreshStyle {
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

/// Material's pull-to-refresh.
pub struct MaterialPullToRefresh {
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

impl MaterialPullToRefresh {
    pub fn new() -> Self {
        let props = PullToRefreshProps::default();
        Self {
            refreshing: shared_flag(props.loading),
            props,
            child: None,
            on_refresh: None,
            pull: Arc::new(Mutex::new(0.0)),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MaterialPullToRefreshStyle {
        let theme = self.theme.get();
        let ComponentTokens::PullToRefresh(tokens) = theme.component(ComponentId::PullToRefresh) else {
            unreachable!("ComponentId::PullToRefresh must resolve to PullToRefreshTokens")
        };
        MaterialPullToRefreshStyle {
            design_system: "material",
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
        let background = to_paint(style.background);
        let column = MaterialPullToRefreshColumn {
            style,
            pull: self.pull.clone(),
            refreshing: self.refreshing.clone(),
            on_refresh: self.on_refresh.clone(),
            child: self.child,
            reported_pull: None,
            reported_refreshing: false,
            fired: false,
        };

        Surface::new()
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    // v1's `items_start()`: the column sits at the top of the
                    // box, and at its own width on the other axis.
                    .cross_align(Align::Start),
            )
            .background(background)
            .push(move || Box::new(column.clone()))
    }
}

impl Default for MaterialPullToRefresh {
    fn default() -> Self {
        Self::new()
    }
}

/// The column: the indicator, then the caller's content.
///
/// A kit-local widget because the engine has none that re-declares its
/// children when a shared value changes -- see the module note.
#[derive(Clone)]
struct MaterialPullToRefreshColumn {
    style: MaterialPullToRefreshStyle,
    pull: PullDistance,
    refreshing: SharedFlag,
    on_refresh: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    child: Option<Child>,
    /// The pull as of the last update, so a change is a change.
    reported_pull: Option<f32>,
    /// The refreshing flag as of the last update.
    reported_refreshing: bool,
    /// Whether the refresh has already fired for this pull.
    fired: bool,
}

impl Widget for MaterialPullToRefreshColumn {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let pull = read_pull(&self.pull);
        let refreshing = read_flag(&self.refreshing);
        if self.reported_pull != Some(pull) || self.reported_refreshing != refreshing {
            self.reported_pull = Some(pull);
            self.reported_refreshing = refreshing;
            // The indicator's height is the pull, and a node's own layout
            // request is only re-read when its parent reconciles. The parent
            // here is this widget, so this is the reconcile to ask for.
            cx.mark_children_dirty(id);
        }
        // v1 fired the refresh when the pointer came up past the threshold.
        // The drag is the caller's here, so the threshold is crossed by
        // whoever writes the pull distance.
        if !self.fired && pull >= self.style.threshold {
            self.fired = true;
            write_flag(&self.refreshing, true);
            if let Some(on_refresh) = &self.on_refresh {
                on_refresh();
            }
        }
        if pull <= 0.0 {
            // Back at the top: the next pull fires again.
            self.fired = false;
        }
    }

    fn children(&self) -> Vec<Child> {
        let indicator = MaterialPullToRefreshIndicator {
            style: self.style,
            pull: self.pull.clone(),
            refreshing: self.refreshing.clone(),
            reported_label: None,
        };
        let mut out = vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(indicator.clone()),
        )];
        if let Some(child) = &self.child {
            out.push(Child::new(
                ChildKey::index(1),
                LayoutRequest::new(),
                {
                    let child = child.clone();
                    move || child.build()
                },
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // v1's `Stack::vertical((indicator, child))`.
        Some(LayoutRequest::new().stack(Axis::Vertical, 0.0))
    }
}

impl WidgetAny for MaterialPullToRefreshColumn {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// The indicator: the label, and the height that is the pull.
#[derive(Clone)]
struct MaterialPullToRefreshIndicator {
    style: MaterialPullToRefreshStyle,
    pull: PullDistance,
    refreshing: SharedFlag,
    /// The label as of the last update.
    reported_label: Option<&'static str>,
}

impl MaterialPullToRefreshIndicator {
    /// The label v1 would have drawn right now.
    fn label(&self) -> &'static str {
        if read_flag(&self.refreshing) {
            REFRESH_LABEL
        } else {
            PULL_LABEL
        }
    }

    /// Whether v1 would have drawn anything at all.
    fn visible(&self) -> bool {
        read_pull(&self.pull) > INDICATOR_VISIBLE_PULL || read_flag(&self.refreshing)
    }

    /// The height v1 gave the label's box.
    fn height(&self) -> f32 {
        if !self.visible() {
            return 0.0;
        }
        if read_flag(&self.refreshing) {
            self.style.indicator_max_height
        } else {
            read_pull(&self.pull).min(self.style.indicator_max_height)
        }
    }
}

impl Widget for MaterialPullToRefreshIndicator {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let label = self.label();
        if self.reported_label != Some(label) {
            self.reported_label = Some(label);
            // A child's content is fixed when it is built, so a new label is a
            // new key: the text is remounted rather than restyled.
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        if !self.visible() {
            // v1 drew an `Empty` here: nothing at all, and no height.
            return Vec::new();
        }
        let label = self.label();
        let font_size = self.style.indicator_font_size;
        let color = to_paint(self.style.on_surface_variant);
        vec![Child::new(
            ChildKey::from(label),
            LayoutRequest::new(),
            move || Box::new(Text::new(label).size(font_size).color(color)),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().height(self.height()))
    }
}

impl WidgetAny for MaterialPullToRefreshIndicator {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// Read the shared pull distance. A poisoned mutex reads as "not pulled".
fn read_pull(pull: &PullDistance) -> f32 {
    pull.lock().map(|guard| *guard).unwrap_or(0.0)
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
