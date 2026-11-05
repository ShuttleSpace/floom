// SPDX-License-Identifier: MIT
//! `VectorIcon` — an icon drawn from vector path data.
//!
//! Icons used to be handed as SVG markup to the renderer (`usvg` +
//! `Renderer::draw_svg`). Floom's paint contract has no SVG: a widget draws with
//! rectangles, text and [`Path`]s. Rather than port a document renderer into a
//! widget crate, this draws the one shape every icon set is made of — a filled
//! outline — and [`crate::svg_path`] parses the `d` attribute.
//!
//! ## Why this is a widget and not a `Canvas`
//!
//! A `Canvas` closure could do the same drawing, but the icon is the most reused
//! piece of vector UI there is (16 floem-ui components draw one), so it earns a
//! type: intrinsic size, semantics, and a scene snapshot a test can assert. All
//! three kits ship identical icon sets — Material, Cupertino and Fluent's tables
//! differ only in glyphs — so this is promoted once here rather than written
//! three times.
//!
//! ## Theme-agnostic
//!
//! Like every 2.0 widget it takes a resolved `color` and `size`; deciding *which*
//! palette is the kit's job (`floom-core-theme` -> `MdIcon::resolve`). See
//! `docs/porting-floem-ui-to-2.0.md`.
//!
//! ## Geometry
//!
//! The path is authored in a view box (`viewBox="0 0 24 24"`, or any other user
//! space) and mapped onto whatever box layout hands the widget. The mapping
//! preserves aspect ratio and centres the result, because stretching a glyph to
//! fill an over-sized slot is never what a caller means and a squashed `menu`
//! reads as a bug. Paint fills `cx.bounds()`; `size` only feeds the layout
//! request.

use floom_accessibility::SemanticsRole;
use floom_core::{
    Color, CreateContext, ElementId, LayoutRequest, PaintContext, Rect, SemanticsContext,
    UpdateContext, Widget,
};
use floom_types::{Path, Transform};

use crate::primitives::{Revision, SemanticsHint, emit_semantics};
use crate::svg_path::parse_path_data;

/// An icon drawn as a filled outline.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorIcon {
    /// The outline, in the view box's own coordinates.
    path: Path,
    /// The user-space box the outline was authored in.
    view_box: Rect,
    /// Edge length in logical px; also the intrinsic size.
    size: f32,
    color: Color,
    label: Option<String>,
    revision: Revision,
}

impl VectorIcon {
    /// An icon from an already-parsed path, authored inside `view_box`.
    pub fn new(path: Path, view_box: Rect) -> Self {
        Self {
            path,
            view_box,
            size: 24.0,
            color: Color::BLACK,
            label: None,
            revision: Revision::default(),
        }
    }

    /// An icon from SVG path data — the `d` attribute of a `<path>`.
    ///
    /// The parse happens once, here, so `paint` never re-reads a string. Data
    /// that cannot be parsed yields an empty path, which paints nothing; icon
    /// sets are compile-time constants, so a bad one is a bug to fix rather
    /// than a condition to survive.
    pub fn from_path_data(data: &str, view_box: Rect) -> Self {
        Self::new(parse_path_data(data), view_box)
    }

    /// Edge length in logical px. Icons are square by convention, so this is
    /// both axes.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self.revision.bump();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    /// Gives the icon an accessible name. Without one it is decorative.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The colour this icon is filled with.
    pub fn fill(&self) -> Color {
        self.color
    }

    /// The in-place form of [`Self::color`].
    ///
    /// It exists because a widget that *contains* an icon cannot restyle it: a
    /// 2.0 slot is a replayed builder that runs once per key, so a composite that
    /// changed the icon's colour — a disabled toggle, a theme switch — would
    /// otherwise keep painting the old one forever. Every other widget here that
    /// owns a decoration has the same pair (`Surface::set_background`,
    /// `Ripple::set_ink`, `Text::set_weight`), and the reasoning about
    /// idempotence is theirs: a theme signal fires on unrelated changes too, so a
    /// setter that always bumped would repaint the tree on every one of them.
    pub fn set_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::size`], for the same reason as
    /// [`Self::set_color`].
    pub fn set_size(&mut self, size: f32) {
        if self.size != size {
            self.size = size;
            self.revision.bump();
        }
    }

    /// The configured edge length, in logical px. Paint fills `cx.bounds()`;
    /// this is the intrinsic size, which is also what `layout_request` asks for.
    pub fn edge(&self) -> f32 {
        self.size
    }

    /// The accessible name, when one was given.
    pub fn name(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// What this icon reports to assistive technology.
    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.label.clone() {
            Some(label) if !label.is_empty() => SemanticsHint::labeled(SemanticsRole::Image, label),
            // An unlabelled glyph has no text to announce, so it publishes
            // nothing — the same default the font-icon `primitives::Icon` uses.
            _ => SemanticsHint::decorative(),
        }
    }

    /// The transform mapping this icon's view box onto `bounds`: uniform
    /// scale, centred.
    ///
    /// Public because it is the whole of the icon's geometry — a caller
    /// composing icons into a larger drawing wants the same mapping rather than
    /// its own approximation of it. Returns `None` when either box is
    /// degenerate: nothing to draw, and a zero divisor would produce a `NaN`
    /// path rather than an empty one.
    pub fn transform_for(&self, bounds: Rect) -> Option<Transform> {
        let (vw, vh) = (self.view_box.width(), self.view_box.height());
        if vw <= 0.0 || vh <= 0.0 || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return None;
        }
        let scale = (bounds.width() / vw).min(bounds.height() / vh);
        let tx = bounds.x0 + (bounds.width() - vw * scale) / 2.0;
        let ty = bounds.y0 + (bounds.height() - vh * scale) / 2.0;
        Some(
            Transform::translate(tx - self.view_box.x0 * scale, ty - self.view_box.y0 * scale)
                .multiply(Transform::scale(scale, scale)),
        )
    }
}

impl Widget for VectorIcon {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        if self.size <= 0.0 {
            return None;
        }
        Some(LayoutRequest::new().width(self.size).height(self.size))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.path.is_empty() {
            return;
        }
        let Some(transform) = self.transform_for(cx.bounds()) else {
            return;
        };
        cx.fill_path(&self.path.transform(transform), self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
