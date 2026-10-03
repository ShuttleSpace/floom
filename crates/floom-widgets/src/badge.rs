//! `Badge` — a small status indicator: a dot, or a rounded pill of text.
//!
//! Theme-agnostic by design. A 2.0 widget takes explicit values rather than
//! reading a global theme signal, so resolving the palette (which design
//! system, error colour, token sizes) is the *kit's* job — see
//! `docs/porting-floem-ui-to-2.0.md`.

use floom_accessibility::SemanticsRole;
use floom_core::{
    Color, CreateContext, ElementId, LayoutRequest, PaintContext, Point, SemanticsContext,
    UpdateContext, Widget,
};

use crate::primitives::{Revision, SemanticsHint, emit_semantics};

/// A status indicator rendered as a dot or a text pill.
#[derive(Debug, Clone, PartialEq)]
pub struct Badge {
    /// Text inside the pill. Ignored when `dot` is set.
    pub content: String,
    /// Render as a plain circle with no text.
    pub dot: bool,
    /// Fill of the bubble.
    pub background: Color,
    /// Colour of `content`.
    pub foreground: Color,
    pub font_size: f32,
    /// Lower bound on the bubble's width and height.
    pub min_size: f32,
    /// Padding around `content`, per side.
    pub padding: f32,
    /// Diameter used when `dot` is set.
    pub dot_size: f32,
    /// Corner radius. Half the height gives a pill.
    pub radius: f32,
    revision: Revision,
}

impl Default for Badge {
    fn default() -> Self {
        Self {
            content: String::new(),
            dot: false,
            background: Color::rgb(200, 0, 0),
            foreground: Color::rgb(255, 255, 255),
            font_size: 11.0,
            min_size: 16.0,
            padding: 4.0,
            dot_size: 8.0,
            radius: f32::INFINITY,
            revision: Revision::default(),
        }
    }
}

impl Badge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self.revision.bump();
        self
    }

    pub fn dot(mut self, dot: bool) -> Self {
        self.dot = dot;
        self.revision.bump();
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self.revision.bump();
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = color;
        self.revision.bump();
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self.revision.bump();
        self
    }

    pub fn min_size(mut self, size: f32) -> Self {
        self.min_size = size;
        self.revision.bump();
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self.revision.bump();
        self
    }

    pub fn dot_size(mut self, size: f32) -> Self {
        self.dot_size = size;
        self.revision.bump();
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.revision.bump();
        self
    }

    /// The extent this badge wants, given its own content.
    ///
    /// A dot is a fixed `dot_size` square. A pill is the text plus padding on
    /// each side, floored at `min_size` on both axes so a one-character badge
    /// still reads as a circle.
    fn intrinsic(&self) -> Option<(f32, f32)> {
        if self.dot {
            return Some((self.dot_size, self.dot_size));
        }
        if self.content.is_empty() {
            return None;
        }
        let measured = floom_text::measure_text(&self.content, self.font_size, f64::INFINITY);
        let text_w = measured.width as f32;
        let text_h = measured.height as f32;
        if text_w <= 0.0 || text_h <= 0.0 {
            return None;
        }
        Some((
            (text_w + self.padding * 2.0).max(self.min_size),
            (text_h + self.padding * 2.0).max(self.min_size),
        ))
    }
}

impl Widget for Badge {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        let (w, h) = self.intrinsic()?;
        Some(LayoutRequest::new().width(w).height(h))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        // A dot is a circle; a pill's radius defaults to half its height so it
        // reads as a stadium rather than a rectangle.
        let half_height = (bounds.height() / 2.0) as f32;
        let radius = if self.dot || !self.radius.is_finite() {
            half_height
        } else {
            self.radius
        };
        cx.fill_rounded_rect(bounds, radius, self.background);

        if self.dot || self.content.is_empty() {
            return;
        }
        let measured = floom_text::measure_text(&self.content, self.font_size, f64::INFINITY);
        let text_w = measured.width as f32;
        let text_h = measured.height as f32;
        // Centre the text in whatever box layout actually gave us, so a badge
        // stretched by its parent still looks right.
        let x = bounds.x0 + (bounds.width() - text_w as f64) / 2.0;
        let y = bounds.y0 + (bounds.height() - text_h as f64) / 2.0;
        cx.fill_text(
            Point::new(x, y),
            &self.content,
            self.font_size,
            self.foreground,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

impl Badge {
    /// What this badge reports to assistive technology.
    ///
    /// A dot badge has no text, so there is nothing to announce and it stays
    /// decorative. A pill reports its content as a static-text label.
    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.dot || self.content.is_empty() {
            return SemanticsHint::decorative();
        }
        SemanticsHint::labeled(SemanticsRole::StaticText, self.content.clone())
    }
}
