// SPDX-License-Identifier: MIT
//! A compact trend line drawn from a series of values.

use floom_core::{
    Color, CreateContext, ElementId, EventContext, LayoutRequest, PaintContext, Point, Rect,
    SemanticsContext, UpdateContext, Widget,
};
use floom_types::{Path, Stroke};

use floom_widgets::{emit_semantics, Revision, SemanticsHint};

/// A small line chart: the values in order, scaled to fit the box it is given.
///
/// It draws nothing without `data`, so a sparkline with an empty series is a
/// decorative zero-size box rather than a flat baseline pretending to mean
/// something.
pub struct Sparkline {
    /// The series, in order. `[]` draws nothing.
    pub data: Vec<f32>,
    /// The line's ink.
    pub color: Color,
    /// The line's thickness, in logical pixels.
    pub line_width: f64,
    /// The space between the line's extremes and the box edge.
    pub padding: f64,
    revision: Revision,
}

impl Default for Sparkline {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            color: Color::rgb(0x67, 0x53, 0xEC),
            line_width: 2.0,
            padding: 2.0,
            revision: Revision::default(),
        }
    }
}

impl std::fmt::Debug for Sparkline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sparkline")
            .field("len", &self.data.len())
            .finish_non_exhaustive()
    }
}

impl Sparkline {
    pub fn new() -> Self {
        Self::default()
    }

    /// The series to plot, in order.
    pub fn data(mut self, data: impl Into<Vec<f32>>) -> Self {
        self.data = data.into();
        self.revision.bump();
        self
    }

    /// The line's ink.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    /// The line's thickness, in logical pixels.
    pub fn line_width(mut self, width: f64) -> Self {
        self.line_width = width;
        self.revision.bump();
        self
    }

    /// The space between the line's extremes and the box edge.
    pub fn padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self.revision.bump();
        self
    }

    /// The points the data maps to, scaled into `bounds`.
    ///
    /// `None` when there is no data, or a single repeated value (a line needs a
    /// spread to draw), so the painter can skip the degenerate cases instead of
    /// dividing by zero.
    fn points(&self, bounds: Rect) -> Option<Vec<Point>> {
        if self.data.is_empty() {
            return None;
        }
        let min = self.data.iter().copied().fold(f32::INFINITY, f32::min);
        let max = self.data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let span = max - min;
        let inner_w = (bounds.width() - self.padding * 2.0).max(0.0);
        let inner_h = (bounds.height() - self.padding * 2.0).max(0.0);
        let n = self.data.len();
        let step_x = if n > 1 {
            inner_w / (n - 1) as f64
        } else {
            0.0
        };
        let y = |v: f32| -> f64 {
            if span == 0.0 {
                bounds.y0 + self.padding + inner_h / 2.0
            } else {
                let t = (v - min) / span;
                bounds.y0 + self.padding + (1.0 - t as f64) * inner_h
            }
        };
        Some(
            self.data
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    Point::new(bounds.x0 + self.padding + i as f64 * step_x, y(*v))
                })
                .collect(),
        )
    }
}

impl Widget for Sparkline {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // The box is the caller's to size; a sparkline has no intrinsic content
        // width beyond "enough to read the trend", so it asks for nothing and
        // fills what it is given.
        None
    }

    fn event(&mut self, _cx: &mut dyn EventContext, _id: ElementId) -> bool {
        // A trend line is read, not touched.
        false
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let Some(points) = self.points(bounds) else {
            return;
        };
        let mut path = Path::new();
        for (i, p) in points.iter().enumerate() {
            if i == 0 {
                path.move_to(*p);
            } else {
                path.line_to(*p);
            }
        }
        let stroke = Stroke::new(self.line_width, self.color);
        cx.stroke_path(&path, &stroke);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(SemanticsHint::decorative(), cx);
    }
}
