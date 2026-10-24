//! `VirtualScroller` -- a viewport that mounts only the rows it can show.
//!
//! The list is a plain list of rows; the scroller mounts the window of them
//! that the viewport covers, and leaves the rest unmounted. The content's
//! height stays the whole list's height, so the scrollbar is honest.
//!
//! The three floem-ui virtual-scroller kits each used to carry their own
//! copy of this viewport; they share the engine's one now.
//!
//! ## The offset is the scroller's own
//!
//! The engine's `Scroll` keeps its offset to itself and applies one offset
//! to its whole subtree, so a widget inside it cannot ask how far it has
//! scrolled. This scroller therefore keeps an offset of its own and asks the
//! host to scroll: the offset is in the engine's own units -- `f64`, like
//! `ScrollOffset` -- and it is the scroller's business, not the caller's.
//!
//! ## The rows to mount for an offset
//!
//! v1's window, exactly: the first row is the one the viewport is on, less
//! the overscan; the count is the viewport's worth of rows, plus one, plus
//! the overscan on both sides.

use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    ScrollOffset, UpdateContext, Widget, WidgetAny,
};
use floom_types::ClipShape;
use floom_widgets::{Align, Axis, List, Spacer, Stack};

use crate::builders::WidgetBuilder;

/// The padding at each side of a row. A v1 literal.
const ROW_PADDING: f32 = 12.0;

/// The rows to mount for an offset: v1's window, exactly.
///
/// The first row is the one the viewport is on, less the overscan; the count
/// is the viewport's worth of rows, plus one, plus the overscan on both
/// sides.
fn window_for(
    rows: usize,
    offset: f64,
    viewport: f32,
    pitch: f32,
    overscan: usize,
) -> (usize, usize) {
    if pitch <= 0.0 {
        // A row of no height has no pitch to divide by, so the window is
        // empty rather than the whole list.
        return (0, 0);
    }
    let start = ((offset / f64::from(pitch)).floor() as isize).max(0) as usize;
    let start = start.saturating_sub(overscan);
    let visible = ((viewport / pitch).ceil() as usize) + 1 + overscan * 2;
    let end = (start + visible).min(rows);
    (start, end)
}

/// A key that changes when the window does, so the rows are remounted.
fn window_key(start: usize, end: usize) -> u64 {
    ((start as u64) << 32) | (end as u64)
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

/// The viewport: it owns the scroll offset and mounts the rows the window
/// covers.
#[derive(Clone)]
pub struct VirtualScroller {
    /// The rows, in order.
    rows: Arc<Vec<WidgetBuilder>>,
    /// The scroll offset, in pixels from the top.
    ///
    /// Shared, because the widget is rebuilt whenever its parent re-declares
    /// its children, and a plain field would put the viewport back at the top
    /// every time anything else in the tree changed.
    offset: Arc<Mutex<f64>>,
    /// The rows currently mounted: the first, and the last (exclusive).
    window: (usize, usize),
    /// A row's height.
    item_height: f32,
    /// The gap between rows.
    row_gap: f32,
    /// A row's height plus the gap: the distance between two rows.
    pitch: f32,
    /// The viewport's height.
    viewport: f32,
    /// The whole list's height.
    total: f32,
    /// How many rows are mounted above and below the viewport.
    overscan: usize,
    /// The kit's handle on the callback.
    on_reach_end: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl VirtualScroller {
    pub fn new() -> Self {
        let rows: Arc<Vec<WidgetBuilder>> = Arc::new(Vec::new());
        Self {
            rows,
            offset: Arc::new(Mutex::new(0.0)),
            window: (0, 0),
            item_height: 0.0,
            row_gap: 0.0,
            pitch: 0.0,
            viewport: 0.0,
            total: 0.0,
            overscan: 0,
            on_reach_end: None,
        }
    }

    /// The rows, in order.
    pub fn rows(mut self, rows: Vec<WidgetBuilder>) -> Self {
        self.rows = Arc::new(rows);
        let pitch = self.item_height + self.row_gap;
        let count = self.rows.len();
        self.pitch = pitch;
        self.total = count as f32 * pitch;
        self.window = window_for(count, 0.0, self.viewport, pitch, self.overscan);
        self
    }

    /// A row's height.
    pub fn item_height(mut self, item_height: f32) -> Self {
        self.item_height = item_height;
        self.pitch = self.item_height + self.row_gap;
        self.total = self.rows.len() as f32 * self.pitch;
        self
    }

    /// The gap between rows.
    pub fn row_gap(mut self, row_gap: f32) -> Self {
        self.row_gap = row_gap;
        self.pitch = self.item_height + self.row_gap;
        self.total = self.rows.len() as f32 * self.pitch;
        self
    }

    /// The viewport's height.
    pub fn viewport(mut self, viewport: f32) -> Self {
        self.viewport = viewport;
        self.window = window_for(self.rows.len(), 0.0, viewport, self.pitch, self.overscan);
        self
    }

    /// How many rows are mounted above and below the viewport.
    pub fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self.window = window_for(self.rows.len(), 0.0, self.viewport, self.pitch, overscan);
        self
    }

    /// Called when the scroller reaches the end of the list.
    pub fn on_reach_end(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_reach_end = Some(Arc::new(callback));
        self
    }
}

impl Widget for VirtualScroller {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let offset = read_offset(&self.offset);
        let window = window_for(
            self.rows.len(),
            offset,
            self.viewport,
            self.pitch,
            self.overscan,
        );
        if window != self.window {
            self.window = window;
            // The window moved, so the rows mounted are not the rows wanted.
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let (start, end) = self.window;
        let rows = self.rows.clone();
        let pitch = self.pitch;
        let row_gap = self.row_gap;
        let item_height = self.item_height;
        // The space the rows above and below the window take up, so the
        // content's height stays the whole list's height.
        let top = start as f32 * pitch;
        let bottom = (rows.len().saturating_sub(end)) as f32 * pitch;

        vec![Child::new(
            ChildKey::keyed(window_key(start, end)),
            LayoutRequest::new().width_pct(100.0),
            move || {
                // The inner closure owns its copy: a replayed closure cannot
                // move a value out of its own capture.
                let rows = rows.clone();
                // The rows: the engine's `List`, keyed by the row's index, so
                // a row's state follows the item rather than the slot.
                let list = Child::new(ChildKey::index(1), LayoutRequest::new(), move || {
                    let items: Vec<(u64, WidgetBuilder)> = rows[start..end]
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(offset, build)| ((start + offset) as u64, build))
                        .collect();
                    Box::new(
                        List::new(
                            Axis::Vertical,
                            items,
                            move |(index, _)| *index,
                            move |(_, build)| {
                                // v1's row: the caller's content in a band of
                                // the row's height, indented and centred on
                                // the cross axis.
                                let mut row = Stack::new(Axis::Horizontal)
                                    .align(Align::Start, Align::Center)
                                    .request(
                                        LayoutRequest::new()
                                            .height(item_height)
                                            .width_pct(100.0)
                                            .padding_horiz(f64::from(ROW_PADDING))
                                            .cross_align(Align::Center),
                                    );
                                row = row.push(move || build());
                                Box::new(row)
                            },
                        )
                        .gap(row_gap),
                    )
                });

                let mut column = Stack::new(Axis::Vertical);
                column = column.push_layout(LayoutRequest::new().height(top), move || {
                    Box::new(Spacer::new(f64::from(top)).axis(Axis::Vertical))
                });
                column = column.child(list);
                column = column.push_layout(LayoutRequest::new().height(bottom), move || {
                    Box::new(Spacer::new(f64::from(bottom)).axis(Axis::Vertical))
                });
                Box::new(column)
            },
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .stack(Axis::Vertical, 0.0),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Scroll(delta) = cx.event() else {
            return false;
        };
        let max = (f64::from(self.total) - f64::from(self.viewport)).max(0.0);
        let current = read_offset(&self.offset);
        let next = (current + delta.delta_y).clamp(0.0, f64::from(max));
        if next != current {
            write_offset(&self.offset, next);
            cx.request_scroll(ScrollOffset::new(0.0, next));
        }
        // v1's end test: within a pixel of the end. It had no "there is
        // anything to scroll" guard, so an empty list asks for more on the
        // first wheel; the port keeps that.
        if next >= max - 1.0 {
            if let Some(on_reach_end) = &self.on_reach_end {
                on_reach_end();
            }
        }
        true
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    /// The viewport clips its content.
    ///
    /// Without the clip the rows paint over whatever is beside them, which is
    /// the one thing a scroll area exists to prevent.
    fn clip(&self) -> Option<ClipShape> {
        Some(ClipShape::Box)
    }
}

impl WidgetAny for VirtualScroller {
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
