
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, LayoutRequest, UpdateContext, Widget,
    WidgetAny,
};
use floom_widgets::{read_flag, write_flag, Align, Axis, SharedFlag, Surface, Text};

use crate::shared::SharedNumber;

/// The label the indicator shows while the user is pulling.
const PULL_LABEL: &str = "\u{2193} Pull to refresh";

/// The label the indicator shows while a refresh is running.
const REFRESH_LABEL: &str = "\u{21bb} Refreshing\u{2026}";

/// How far the user has to pull before the refresh fires. A v1 literal.
const INDICATOR_VISIBLE_PULL: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PullToRefreshStyle {
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

impl Default for PullToRefreshStyle {
    fn default() -> Self {
        Self {
            background: Color::rgb(0, 0, 0),
            on_surface_variant: Color::rgb(0, 0, 0),
            max_pull: 0.0,
            threshold: 0.0,
            indicator_size: 0.0,
            indicator_font_size: 0.0,
            indicator_max_height: 0.0,
        }
    }
}

#[derive(Clone)]
struct PullToRefreshColumn {
    style: PullToRefreshStyle,
    pull: SharedNumber,
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

impl Widget for PullToRefreshColumn {
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
        let indicator = PullToRefreshIndicator {
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
            out.push(Child::new(ChildKey::index(1), LayoutRequest::new(), {
                let child = child.clone();
                move || child.build()
            }));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // v1's `Stack::vertical((indicator, child))`.
        Some(LayoutRequest::new().stack(Axis::Vertical, 0.0))
    }
}

impl WidgetAny for PullToRefreshColumn {
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
struct PullToRefreshIndicator {
    style: PullToRefreshStyle,
    pull: SharedNumber,
    refreshing: SharedFlag,
    /// The label as of the last update.
    reported_label: Option<&'static str>,
}

impl PullToRefreshIndicator {
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

impl Widget for PullToRefreshIndicator {
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
        let color = self.style.on_surface_variant;
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

impl WidgetAny for PullToRefreshIndicator {
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
fn read_pull(pull: &SharedNumber) -> f32 {
    pull.lock().map(|guard| *guard).unwrap_or(0.0)
}

/// The pull-to-refresh: the column that watches the drag, and the indicator.
pub struct PullToRefresh {
    /// The kit's answer.
    style: PullToRefreshStyle,
    /// How far the user has pulled.
    pull: SharedNumber,
    /// Whether a refresh is running.
    refreshing: SharedFlag,
    /// Called when the user lets go far enough.
    on_refresh: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// The caller's content.
    child: Option<Child>,
}

impl PullToRefresh {
    pub fn new() -> Self {
        Self {
            style: PullToRefreshStyle::default(),
            pull: crate::shared::shared_number(0.0),
            refreshing: floom_widgets::shared_flag(false),
            on_refresh: None,
            child: None,
        }
    }

    /// The refresh's appearance.
    pub fn style(mut self, style: PullToRefreshStyle) -> Self {
        self.style = style;
        self
    }

    /// The shared number the pull distance lives in.
    pub fn pull(mut self, pull: SharedNumber) -> Self {
        self.pull = pull;
        self
    }

    /// The flag that says whether a refresh is running.
    pub fn refreshing(mut self, refreshing: SharedFlag) -> Self {
        self.refreshing = refreshing;
        self
    }

    /// Called when the user lets go far enough.
    pub fn on_refresh(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_refresh = Some(Arc::new(callback));
        self
    }

    /// The caller's content.
    pub fn child(mut self, child: Option<Child>) -> Self {
        self.child = child;
        self
    }

    /// The refresh, ready to mount.
    pub fn build(self) -> Surface {
        let style = self.style;
        let background = style.background;
        let column = PullToRefreshColumn {
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
