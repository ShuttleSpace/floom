//! Cupertino virtual scroller, ported to the Floom 2.0 widget API.
//!
//! A virtual scroller is a viewport over a list too long to draw: it mounts
//! only the rows the user can see, and mounts different rows as the viewport
//! moves.
//!
//! ## What the port uses
//!
//! The engine's vocabulary for the box -- a [`Surface`] for the viewport's
//! fill and outline, a [`Stack`] for the column, a [`List`] for the rows --
//! and a **kit-local scroller** for the two things the engine does not have.
//!
//! The engine's `List` is the right answer for the rows: it mirrors a
//! collection into keyed rows, so a row's state follows the item rather than
//! the slot. It cannot, on its own, be a *virtual* list: it mounts every item
//! it is given, and it has no notion of a viewport. So the port hands the
//! `List` only the rows the window covers -- which is the whole trick -- and
//! carries the window itself.
//!
//! The engine's `Scroll` is a viewport that scrolls, and it is the right
//! answer for a box that only scrolls. It cannot answer the question a
//! virtual scroller asks -- *where is the user?* -- because its offset is its
//! own business and nothing outside it can read it. So the port carries a
//! scroller of its own, and the gap is named in the engine's favour: this
//! belongs in `floom-ui-composites` beside the other scroll composites, and
//! until it is there, three kits carry three copies of it.
//!
//! ## What this kit answers
//!
//! * a row's height, the gap between rows and the row's font size: the
//!   tokens;
//! * the viewport's fill: `surface`; its outline: `outline` at 80;
//! * the row's ink: `on_surface`.
//!
//! ## The tokens that never showed
//!
//! `on_surface_variant` and `border_radius` are in v1's style and v1 never
//! painted with either: the rows wrote `on_surface`, and the viewport was a
//! square-cornered box. The style still reports them, because v1's style did.
//!
//! `VirtualScrollerProps::padding` is a dead field of the shared props: v1
//! never read it, and the port does not either.
//!
//! ## The numbers v1 wrote as literals
//!
//! The row's horizontal padding (12) and the viewport's corner radius (12,
//! which v1 reported and never drew).
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The row's hover.** v1 wrote `.hover(|s| s.background(with_alpha(on_surface, 6)))`
//!   on every row. 2.0 has no hover background for a row that is not a
//!   `Pressable`, and the port's row is the engine's vocabulary -- a `Stack`
//!   around the caller's content. The hover is named rather than faked.
//! * **The row's pointer cursor.** v1 asked for a pointer on every row, for
//!   the same reason: the row was a `Container` and 2.0's `Stack` cannot ask
//!   for a cursor. (The engine's `Widget::cursor` door exists; a `Stack` does
//!   not use it.)
//! * **The row's ink and size.** v1 put `.color(on_surface)` and
//!   `.font_size(fs)` on the row, and the caller's own text inherited both.
//!   2.0's `Text` carries its own colour and size, so the caller's content
//!   keeps whatever it was given.
//! * **The callback is not in the props.** `VirtualScrollerProps::on_reach_end`
//!   is a `Box<dyn Fn()>`, which is not `Send + Sync`, and 2.0's slots are.
//!   The kit keeps its own `Arc<dyn Fn() + Send + Sync>` and the props' slot
//!   stays empty, so `props()` reports `None` where v1 reported `Some`.
//! * **The accessors.** v1's Cupertino virtual scroller had no `props()`,
//!   `theme()` or `with_theme()`. The port gives all three kits the same
//!   three, because a kit that cannot be given a theme cannot be tested
//!   against one.
//!
//! ## The fields the port's style adds
//!
//! v1's style reported neither the viewport's height nor the overscan, and
//! the paint used both. The port's style reports them, because a style that
//! does not say what the widget draws is not the widget's output.
//!
//! ## The one field the port's style changes
//!
//! v1's style reported the *token's* row gap while the paint used the
//! caller's override. The port reports the gap the paint uses, and says so
//! here rather than leaving a style that disagrees with the widget.

use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    ScrollOffset, UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{ClipShape, Color as PaintColor};
use floom_widgets::{Align, Axis, List, Spacer, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::virtual_scroller::VirtualScrollerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// A builder for one row's content.
///
/// Replayed each time the window moves, which is what makes a virtual list
/// possible at all: the caller's content is built when its row is mounted, and
/// not before.
pub type RowBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// The row's horizontal padding. A v1 literal.
const ROW_PADDING: f32 = 12.0;

/// The viewport's height when the caller does not say. A v1 literal.
const DEFAULT_VIEWPORT: f32 = 360.0;

/// The virtual scroller's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoVirtualScrollerStyle {
    pub design_system: &'static str,
    /// The viewport's fill.
    pub surface: Color,
    /// The row's ink.
    pub on_surface: Color,
    /// v1 never painted with this; see the module documentation.
    pub on_surface_variant: Color,
    /// The viewport's outline.
    pub outline: Color,
    /// The viewport's corner radius. A v1 literal, and v1 never drew it.
    pub border_radius: f32,
    /// A row's height.
    pub item_height: f32,
    /// The gap between rows.
    pub row_gap: f32,
    /// The row's font size.
    pub font_size: f32,
    /// The viewport's height.
    pub viewport_height: f32,
    /// How many rows are mounted above and below the viewport.
    pub overscan: usize,
}

/// Cupertino's virtual scroller.
pub struct CupertinoVirtualScroller {
    props: VirtualScrollerProps,
    theme: RwSignal<MultiTheme>,
    /// The rows, in order.
    items: Vec<RowBuilder>,
    /// The viewport's height.
    height: f32,
    /// The kit's own handle on the callback.
    ///
    /// Not the props': `VirtualScrollerProps::on_reach_end` is a
    /// `Box<dyn Fn()>`, and 2.0's slots are `Send + Sync`.
    on_reach_end: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl CupertinoVirtualScroller {
    pub fn new() -> Self {
        Self {
            props: VirtualScrollerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            items: Vec::new(),
            height: 0.0,
            on_reach_end: None,
        }
    }

    /// The rows, in order.
    ///
    /// Each row is a builder, not a widget: a virtual list builds a row when
    /// it is mounted, and the rows it mounts change as the viewport moves.
    pub fn items(mut self, items: Vec<RowBuilder>) -> Self {
        self.items = items;
        self
    }

    /// Add one row (repeatable).
    pub fn item(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.items.push(Arc::new(build));
        self
    }

    /// The viewport's height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// A row's height, overriding the token's.
    pub fn item_height(mut self, height: f32) -> Self {
        self.props.item_height = height;
        self
    }

    /// The gap between rows, overriding the token's.
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.props.row_gap = gap;
        self
    }

    /// How many rows are mounted above and below the viewport.
    pub fn overscan(mut self, overscan: usize) -> Self {
        self.props.overscan = overscan;
        self
    }

    /// Called when the user reaches the end of the list.
    pub fn on_reach_end(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_reach_end = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &VirtualScrollerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoVirtualScrollerStyle {
        let theme = self.theme.get();
        let ComponentTokens::VirtualScroller(tokens) = theme.component(ComponentId::VirtualScroller)
        else {
            unreachable!("ComponentId::VirtualScroller must resolve to VirtualScrollerTokens")
        };
        let item_height = if self.props.item_height > 0.0 {
            self.props.item_height
        } else {
            tokens.item_height
        };
        // v1's style reported the token's row gap while the paint used the
        // caller's override; the port reports the gap the paint uses.
        let row_gap = if self.props.row_gap > 0.0 {
            self.props.row_gap
        } else {
            tokens.row_gap
        };
        CupertinoVirtualScrollerStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            // v1 hardcoded 12.0 and never drew it.
            border_radius: 12.0,
            item_height,
            row_gap,
            font_size: tokens.font_size,
            viewport_height: if self.height > 0.0 {
                self.height
            } else {
                DEFAULT_VIEWPORT
            },
            overscan: self.props.overscan,
        }
    }

    /// The viewport: the box, and the scroller inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let fill = to_paint(style.surface);
        let outline = to_paint(with_alpha(style.outline, 80));
        let rows: Arc<Vec<RowBuilder>> = Arc::new(self.items);
        let count = rows.len();
        let pitch = style.item_height + style.row_gap;
        let viewport = style.viewport_height;
        let overscan = style.overscan;
        let item_height = style.item_height;
        let row_gap = style.row_gap;
        let on_reach_end = self.on_reach_end.clone();
        // The window the viewport starts on: the top of the list.
        let window = window_for(count, 0.0, viewport, pitch, overscan);

        Surface::new()
            .request(LayoutRequest::new().height(viewport).width_pct(100.0))
            .background(fill)
            // v1's `.border(1.0)` and `.border_color(outline at 80)`.
            .border(1.0, outline)
            .push(move || {
                Box::new(CupertinoVirtualScrollerView {
                    rows: rows.clone(),
                    offset: Arc::new(Mutex::new(0.0)),
                    window,
                    item_height,
                    row_gap,
                    pitch,
                    viewport,
                    total: count as f32 * pitch,
                    overscan,
                    on_reach_end: on_reach_end.clone(),
                })
            })
    }
}

impl Default for CupertinoVirtualScroller {
    fn default() -> Self {
        Self::new()
    }
}

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
struct CupertinoVirtualScrollerView {
    /// The rows, in order.
    rows: Arc<Vec<RowBuilder>>,
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

impl Widget for CupertinoVirtualScrollerView {
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
                    let items: Vec<(u64, RowBuilder)> = rows[start..end]
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
        Some(LayoutRequest::new().width_pct(100.0).stack(Axis::Vertical, 0.0))
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

impl WidgetAny for CupertinoVirtualScrollerView {
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

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
