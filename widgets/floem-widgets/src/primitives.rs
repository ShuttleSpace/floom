//! Tier-0 primitive widgets.
//!
//! Primitives are *descriptions*: they carry the data needed to lay out, paint
//! and describe a node, and they lower that data into the renderer-neutral
//! contracts in [`floem_core`]. Nothing in this module touches a concrete
//! renderer, a windowing system, or a GPU backend.
//!
//! Invalidation follows the architecture's dirty rules: mutating a primitive
//! bumps an internal revision, and the next [`Widget::update`] marks the
//! element dirty exactly once until the change has been painted.

use floem_accessibility::{SemanticsRole, SemanticsState};
use floem_core::{
    Color, CreateContext, ElementId, PaintContext, Point, Rect, ResourceId, SemanticsContext, Size,
    UpdateContext, Widget,
};
use floem_types::Insets;

/// Main axis for one-dimensional primitives ([`Spacer`], [`Divider`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

/// How an [`Image`] maps its intrinsic size onto the bounds it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ImageFit {
    /// Stretch to exactly fill the bounds (may distort).
    Fill,
    /// Scale down/up preserving aspect ratio so the whole image is visible.
    #[default]
    Contain,
    /// Scale preserving aspect ratio so the bounds are fully covered (crops).
    Cover,
    /// Draw at intrinsic size, centered.
    None,
}

/// What a primitive reports to assistive technology.
///
/// `SemanticsContext` currently only carries a label, so `role` is published
/// here for the runtime's semantics bridge to consume when it lowers widgets
/// into `floem_accessibility::SemanticsTree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsHint {
    pub role: SemanticsRole,
    pub label: Option<String>,
    pub state: SemanticsState,
}

impl SemanticsHint {
    /// Decorative / transparent: skipped by assistive technology.
    pub const fn decorative() -> Self {
        Self {
            role: SemanticsRole::None,
            label: None,
            state: SemanticsState::empty(),
        }
    }

    pub fn labeled(role: SemanticsRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: Some(label.into()),
            state: SemanticsState::empty(),
        }
    }

    /// Attaches accessibility state (`CHECKED`, `DISABLED`, `FOCUSED`, …).
    pub fn with_state(mut self, state: SemanticsState) -> Self {
        self.state = self.state.union(state);
        self
    }
}

impl Default for SemanticsHint {
    fn default() -> Self {
        Self::decorative()
    }
}

/// Revision counter driving one-shot `PAINT` invalidation.
///
/// Starts "unpainted" so the first [`Widget::update`] after creation marks the
/// element dirty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Revision {
    current: u64,
    painted: u64,
}

impl Default for Revision {
    fn default() -> Self {
        Self {
            current: 1,
            painted: 0,
        }
    }
}

impl Revision {
    /// Call from every mutating builder.
    pub(crate) fn bump(&mut self) {
        self.current = self.current.wrapping_add(1);
    }

    /// Marks `id` dirty at most once per batch of mutations.
    pub(crate) fn sync(&mut self, cx: &mut dyn UpdateContext, id: ElementId) -> bool {
        if self.painted == self.current {
            return false;
        }
        self.painted = self.current;
        cx.mark_dirty(id);
        true
    }
}

pub(crate) fn emit_semantics(hint: SemanticsHint, cx: &mut dyn SemanticsContext) {
    if hint.role == SemanticsRole::None && hint.label.is_none() {
        return;
    }
    if let Some(label) = hint.label.as_deref() {
        cx.set_label(label);
    }
    let bounds = cx.bounds();
    cx.set_bounds(bounds);
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// A run of static text.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    content: String,
    size: f32,
    color: Color,
    /// Overrides the accessibility label; defaults to `content`.
    label: Option<String>,
    revision: Revision,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 14.0,
            color: Color::BLACK,
            label: None,
            revision: Revision::default(),
        }
    }

    /// Font size in logical pixels.
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

    /// Replaces the accessibility label (the visible text stays unchanged).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// In-place content change; bumps the revision so the element repaints.
    pub fn set_content(&mut self, content: impl Into<String>) -> &mut Self {
        self.content = content.into();
        self.revision.bump();
        self
    }

    pub fn font_size(&self) -> f32 {
        self.size
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.label.clone().or_else(|| Some(self.content.clone())) {
            Some(label) if !label.is_empty() => {
                SemanticsHint::labeled(SemanticsRole::StaticText, label)
            }
            _ => SemanticsHint::decorative(),
        }
    }
}

impl Widget for Text {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.content.is_empty() || self.size <= 0.0 {
            return;
        }
        // Nothing to draw when layout gave us no space.
        if cx.bounds().width() <= 0.0 || cx.bounds().height() <= 0.0 {
            return;
        }
        let origin = cx.bounds().origin();
        cx.fill_text(origin, &self.content, self.size, self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Icon
// ---------------------------------------------------------------------------

/// A single glyph from an icon font.
///
/// Icons are decorative by default: they emit no semantics unless a label is
/// supplied (which promotes them to an accessible static text node).
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    glyph: char,
    size: f32,
    color: Color,
    label: Option<String>,
    revision: Revision,
}

impl Icon {
    pub fn new(glyph: char) -> Self {
        Self {
            glyph,
            size: 16.0,
            color: Color::BLACK,
            label: None,
            revision: Revision::default(),
        }
    }

    /// Font size in logical pixels (icons are square).
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

    /// Gives the icon an accessible name; without it the icon is decorative.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    pub fn glyph(&self) -> char {
        self.glyph
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.label.clone() {
            Some(label) if !label.is_empty() => {
                SemanticsHint::labeled(SemanticsRole::StaticText, label)
            }
            _ => SemanticsHint::decorative(),
        }
    }
}

impl Widget for Icon {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.size <= 0.0 {
            return;
        }
        if cx.bounds().width() <= 0.0 || cx.bounds().height() <= 0.0 {
            return;
        }
        let mut buf = [0u8; 4];
        let glyph = self.glyph.encode_utf8(&mut buf);
        let origin = cx.bounds().origin();
        cx.fill_text(origin, glyph, self.size, self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Image
// ---------------------------------------------------------------------------

/// A registered image resource.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    resource: ResourceId,
    /// Intrinsic size in logical pixels; used by [`ImageFit`].
    intrinsic: Size,
    fit: ImageFit,
    alt: Option<String>,
    revision: Revision,
}

impl Image {
    pub fn new(resource: ResourceId) -> Self {
        Self {
            resource,
            intrinsic: Size::ZERO,
            fit: ImageFit::default(),
            alt: None,
            revision: Revision::default(),
        }
    }

    pub fn intrinsic_size(mut self, size: Size) -> Self {
        self.intrinsic = size;
        self.revision.bump();
        self
    }

    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self.revision.bump();
        self
    }

    /// Alternative text; an image without alt text is decorative.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self.revision.bump();
        self
    }

    pub fn resource(&self) -> ResourceId {
        self.resource
    }

    /// Maps the intrinsic size onto `bounds` according to the fit policy.
    pub fn resolve_bounds(&self, bounds: Rect) -> Rect {
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Rect::ZERO;
        }
        if self.fit == ImageFit::Fill {
            return bounds;
        }
        let scale = match self.fit {
            ImageFit::None => 1.0,
            ImageFit::Contain => {
                if self.intrinsic.width <= 0.0 || self.intrinsic.height <= 0.0 {
                    return bounds;
                }
                (bounds.width() / self.intrinsic.width).min(bounds.height() / self.intrinsic.height)
            }
            ImageFit::Cover => {
                if self.intrinsic.width <= 0.0 || self.intrinsic.height <= 0.0 {
                    return bounds;
                }
                (bounds.width() / self.intrinsic.width).max(bounds.height() / self.intrinsic.height)
            }
            ImageFit::Fill => unreachable!(),
        };
        let width = self.intrinsic.width * scale;
        let height = self.intrinsic.height * scale;
        let origin = Point::new(
            bounds.x0 + (bounds.width() - width) / 2.0,
            bounds.y0 + (bounds.height() - height) / 2.0,
        );
        Rect::from_origin_size(origin, Size::new(width, height))
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.alt.clone() {
            Some(alt) if !alt.is_empty() => SemanticsHint::labeled(SemanticsRole::Image, alt),
            _ => SemanticsHint {
                role: SemanticsRole::Image,
                label: None,
                state: SemanticsState::empty(),
            },
        }
    }
}

impl Widget for Image {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        let resolved = self.resolve_bounds(bounds);
        if resolved.width() <= 0.0 || resolved.height() <= 0.0 {
            return;
        }
        cx.draw_image(resolved, self.resource);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

/// A decoration box: background, corner radius, and inner padding.
///
/// Semantically transparent — it never publishes a node of its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Container {
    background: Option<Color>,
    radius: f32,
    padding: Insets,
    revision: Revision,
}

impl Container {
    pub fn new() -> Self {
        Self {
            background: None,
            radius: 0.0,
            padding: Insets::ZERO,
            revision: Revision::default(),
        }
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self.revision.bump();
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.revision.bump();
        self
    }

    /// Padding is consumed by the layout layer; the primitive only carries it.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self.revision.bump();
        self
    }

    pub fn background_color(&self) -> Option<Color> {
        self.background
    }

    pub fn corner_radius(&self) -> f32 {
        self.radius
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Container {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let Some(color) = self.background else {
            return;
        };
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        if self.radius > 0.0 {
            cx.fill_rounded_rect(bounds, self.radius, color);
        } else {
            cx.fill_rect(bounds, color);
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Spacer
// ---------------------------------------------------------------------------

/// Empty space along one axis. Paints nothing and emits no semantics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacer {
    extent: f64,
    axis: Axis,
    revision: Revision,
}

impl Spacer {
    /// Fixed extent in logical pixels along [`Spacer::axis`].
    pub fn new(extent: f64) -> Self {
        Self {
            extent,
            axis: Axis::default(),
            revision: Revision::default(),
        }
    }

    /// A spacer that absorbs all remaining free space (flex-grow style).
    ///
    /// Represented as `f64::INFINITY` so the layout layer can recognise it
    /// without a separate variant.
    pub fn flexible() -> Self {
        Self::new(f64::INFINITY)
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self.revision.bump();
        self
    }

    pub fn extent(&self) -> f64 {
        self.extent
    }

    pub fn is_flexible(&self) -> bool {
        self.extent.is_infinite()
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Spacer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, _cx: &mut dyn SemanticsContext, _id: ElementId) {}
}

// ---------------------------------------------------------------------------
// Divider
// ---------------------------------------------------------------------------

/// A thin rule separating content along one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Divider {
    thickness: f64,
    color: Color,
    axis: Axis,
    /// Inset applied to both ends of the rule.
    indent: f64,
    revision: Revision,
}

impl Divider {
    pub fn new() -> Self {
        Self {
            thickness: 1.0,
            color: Color::rgb(0, 0, 0),
            axis: Axis::default(),
            indent: 0.0,
            revision: Revision::default(),
        }
    }

    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self.revision.bump();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self.revision.bump();
        self
    }

    pub fn indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self.revision.bump();
        self
    }

    /// The rect the rule occupies inside `bounds`.
    pub fn resolve_bounds(&self, bounds: Rect) -> Rect {
        let thickness = self.thickness.max(0.0);
        match self.axis {
            Axis::Horizontal => {
                let y = bounds.y0 + (bounds.height() - thickness) / 2.0;
                Rect {
                    x0: bounds.x0 + self.indent,
                    y0: y,
                    x1: bounds.x1 - self.indent,
                    y1: y + thickness,
                }
            }
            Axis::Vertical => {
                let x = bounds.x0 + (bounds.width() - thickness) / 2.0;
                Rect {
                    x0: x,
                    y0: bounds.y0 + self.indent,
                    x1: x + thickness,
                    y1: bounds.y1 - self.indent,
                }
            }
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Divider {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let rule = self.resolve_bounds(bounds);
        if rule.width() <= 0.0 || rule.height() <= 0.0 {
            return;
        }
        cx.fill_rect(rule, self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
