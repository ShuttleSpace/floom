//! `InfiniteScroller` -- a viewport that asks for the next page when the user
//! reaches the end.
//!
//! The content is the caller's own, with a footer that is a label while a
//! page is being fetched and an empty box otherwise. The viewport owns the
//! scroll offset and asks for the next page when the user gets close enough
//! to the end.
//!
//! The three floem-ui infinite-scroller kits each used to carry their own
//! copy of these two widgets; they share the engine's ones now.
//!
//! ## The content is a widget of its own
//!
//! Two of its numbers change after the tree is mounted: the height the host
//! keeps updating, and the loading flag that decides what the footer is. In
//! 2.0 a box's size is its own `layout_request`, and a `Stack` built once
//! keeps the size it was built with.
//!
//! ## The offset is the scroller's own
//!
//! The engine's `Scroll` keeps its offset to itself and applies one offset
//! to its whole subtree, so a widget inside it cannot ask how far it has
//! scrolled. This scroller therefore keeps an offset of its own and asks the
//! host to scroll. The offset is in the engine's scroll units -- `f64`, like
//! `ScrollOffset` -- while the content's height is a layout unit (`f32`).

use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Event, EventContext, LayoutRequest,
    PaintContext, ScrollOffset, UpdateContext, Widget, WidgetAny,
};
use floom_widgets::{Align, Axis, SharedFlag, Spacer, Stack, Text, read_flag, shared_flag};

use floom_types::ClipShape;

use crate::shared::{SharedNumber, read_number};

/// The shared number, under the name the scroller knows it by.
pub type SharedHeight = SharedNumber;

/// A fresh shared height.
pub use crate::shared::shared_number as shared_height;

/// The loading label's size. A v1 literal.
const LOADING_FONT_SIZE: f32 = 13.0;

/// The scrollable content: the caller's child and the loading footer.
///
/// A widget of its own because two of its numbers change after the tree is
/// mounted: the height the host keeps updating, and the loading flag that
/// decides what the footer is. In 2.0 a box's size is its own
/// `layout_request`, and a `Stack` built once keeps the size it was built
/// with.
#[derive(Clone)]
struct InfiniteScrollerContent {
    /// How tall the content is.
    height: SharedHeight,
    /// Whether a page is being fetched.
    loading: SharedFlag,
    /// The caller's content.
    child: Option<Child>,
    /// The footer's height while a page is being fetched.
    loading_height: f32,
    /// The footer's height while it is not.
    idle_height: f32,
    /// The loading label's ink.
    ink: Color,
    /// The loading label's size.
    font_size: f32,
    /// The loading state this widget last declared its children for.
    last_loading: bool,
}

impl Widget for InfiniteScrollerContent {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let loading = read_flag(&self.loading);
        if loading != self.last_loading {
            self.last_loading = loading;
            // The footer is a different child in each state -- a label while a
            // page is being fetched, an empty box otherwise -- so the box has
            // to be rebuilt rather than repainted.
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let mut children: Vec<Child> = Vec::new();
        if let Some(child) = &self.child {
            children.push(child.clone());
        }
        if read_flag(&self.loading) {
            let ink = self.ink;
            let font_size = self.font_size;
            let height = self.loading_height;
            children.push(Child::new(
                ChildKey::keyed(1),
                LayoutRequest::new().width_pct(100.0).height(height),
                move || {
                    Box::new(
                        Stack::new(Axis::Horizontal)
                            .request(
                                LayoutRequest::new()
                                    .width_pct(100.0)
                                    .height(height)
                                    .main_align(Align::Center)
                                    .cross_align(Align::Center),
                            )
                            .push(move || Box::new(Text::new("Loading…").size(font_size).color(ink))),
                    )
                },
            ));
        } else {
            let height = self.idle_height;
            children.push(Child::new(
                ChildKey::keyed(0),
                LayoutRequest::new().width_pct(100.0).height(height),
                move || Box::new(Spacer::new(f64::from(height)).axis(Axis::Vertical)),
            ));
        }
        children
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(read_number(&self.height))
                .stack(Axis::Vertical, 0.0),
        )
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}
}

impl WidgetAny for InfiniteScrollerContent {
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

/// The viewport: it owns the scroll offset and asks for the next page when
/// the user reaches the end.
#[derive(Clone)]
pub struct InfiniteScrollerView {
    /// The content: the caller's child and the loading footer.
    content: InfiniteScrollerContent,
    /// How tall the content is.
    height: SharedHeight,
    /// Whether a page is being fetched: the re-fire guard.
    loading: SharedFlag,
    /// The kit's handle on the callback.
    on_load_more: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// How close to the end the user has to be.
    threshold: f32,
    /// The viewport's height.
    viewport: f32,
    /// The scroll offset, in pixels from the top.
    ///
    /// Shared, because the widget is rebuilt whenever its parent re-declares
    /// its children, and a plain field would put the viewport back at the top
    /// every time anything else in the tree changed.
    offset: Arc<Mutex<f64>>,
    /// The height this widget last laid its content out at.
    last_height: f32,
}

/// Read the offset, treating a poisoned cell as the top.
fn read_offset(offset: &Arc<Mutex<f64>>) -> f64 {
    offset.lock().map(|guard| *guard).unwrap_or(0.0)
}

/// Write the offset, ignoring a poisoned cell.
fn write_offset(offset: &Arc<Mutex<f64>>, value: f64) {
    if let Ok(mut guard) = offset.lock() {
        *guard = value;
    }
}

impl InfiniteScrollerView {
    pub fn new() -> Self {
        Self {
            content: InfiniteScrollerContent {
                height: crate::shared::shared_number(0.0),
                loading: shared_flag(false),
                child: None,
                loading_height: 0.0,
                idle_height: 0.0,
                ink: Color::rgb(0x49, 0x45, 0x55),
                font_size: LOADING_FONT_SIZE,
                last_loading: false,
            },
            height: crate::shared::shared_number(0.0),
            loading: shared_flag(false),
            on_load_more: None,
            threshold: 0.0,
            viewport: 0.0,
            offset: Arc::new(Mutex::new(0.0)),
            last_height: 0.0,
        }
    }

    /// The content: the caller's child, the loading footer, and the two
    /// numbers they follow.
    pub fn content(
        mut self,
        child: Option<Child>,
        height: SharedHeight,
        loading: SharedFlag,
        loading_height: f32,
        idle_height: f32,
        ink: Color,
    ) -> Self {
        self.content = InfiniteScrollerContent {
            height: height.clone(),
            loading: loading.clone(),
            child,
            loading_height,
            idle_height,
            ink,
            font_size: LOADING_FONT_SIZE,
            last_loading: false,
        };
        self.height = height;
        self.loading = loading;
        self
    }

    /// How close to the end the user has to be before the next page is asked
    /// for.
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// The viewport's height.
    pub fn viewport(mut self, viewport: f32) -> Self {
        self.viewport = viewport;
        self
    }

    /// Called when the user reaches the end of the list.
    pub fn on_load_more(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_load_more = Some(Arc::new(callback));
        self
    }
}

impl Widget for InfiniteScrollerView {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let height = read_number(&self.height);
        if height != self.last_height {
            self.last_height = height;
            // The content's height is the content's own `layout_request`, and
            // only a reconcile re-reads it.
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let content = self.content.clone();
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().width_pct(100.0),
            move || Box::new(content.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().width_pct(100.0).stack(Axis::Vertical, 0.0))
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Scroll(delta) = cx.event() else {
            return false;
        };
        let max =
            (f64::from(read_number(&self.height)) - f64::from(self.viewport)).max(0.0);
        let current = read_offset(&self.offset);
        let next = (current + delta.delta_y).clamp(0.0, f64::from(max));
        if next != current {
            write_offset(&self.offset, next);
            cx.request_scroll(ScrollOffset::new(0.0, next));
        }
        // v1's end test: within `threshold` of the end, and not already
        // fetching a page.
        if max > 0.0 && next >= max - f64::from(self.threshold) && !read_flag(&self.loading) {
            if let Some(on_load_more) = &self.on_load_more {
                on_load_more();
            }
        }
        true
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    /// The viewport clips its content.
    ///
    /// Without the clip the content paints over whatever is beside it, which
    /// is the one thing a scroll area exists to prevent.
    fn clip(&self) -> Option<ClipShape> {
        Some(ClipShape::Box)
    }
}

impl WidgetAny for InfiniteScrollerView {
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
